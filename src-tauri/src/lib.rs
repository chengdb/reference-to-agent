mod actions;
mod config;
mod win;

use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, State, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use config::{Config, MenuConfig, Recipe};
use actions::AxisPos;

struct AppState {
    config: Mutex<Config>,
    /// 菜单显示前的前台窗口（恢复焦点用）。
    prev_foreground: Mutex<Option<isize>>,
}

fn hide_menu(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("menu") {
        let _ = win.hide();
    }
}

/// 把菜单窗口尺寸同步为配置的圆盘直径，避免圆盘与窗口不一致导致裁剪/错位。
fn resize_menu_window(app: &AppHandle, size: u16) {
    if let Some(win) = app.get_webview_window("menu") {
        let s = size.max(100) as f64;
        let _ = win.set_size(LogicalSize::new(s, s));
    }
}

fn show_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// 在鼠标位置弹出菜单，并记录当前前台窗口用于执行后恢复。
fn show_menu(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let menu_win = app.get_webview_window("menu").ok_or("菜单窗口不存在")?;
    let main_win = app.get_webview_window("main").ok_or("主窗口不存在")?;

    // 菜单已显示则收起（toggle）。
    if menu_win.is_visible().unwrap_or(false) {
        let _ = menu_win.hide();
        return Ok(());
    }

    // 按配置调整菜单窗口尺寸（逻辑像素，与前端 CSS px 一致，避免 DPI 缩放偏差）。
    resize_menu_window(app, {
        let cfg = state.config.lock().unwrap();
        cfg.menu.size.max(100)
    });

    let fg = win::current_foreground();
    if fg != 0 {
        let fg_pid = win::window_pid(fg);
        let self_pids = [
            menu_win.hwnd().ok().map(|h| win::window_pid(h.0 as isize)),
            main_win.hwnd().ok().map(|h| win::window_pid(h.0 as isize)),
        ];
        if !self_pids.iter().flatten().any(|p| *p == fg_pid) {
            *state.prev_foreground.lock().unwrap() = Some(fg);
        }
    }

    let pos = win::cursor_pos().ok_or("无法获取鼠标位置")?;
    let size = menu_win.inner_size().map_err(|e| e.to_string())?;
    // 窗口中心对准鼠标位置。
    menu_win
        .set_position(PhysicalPosition::new(
            pos.x - (size.width / 2) as i32,
            pos.y - (size.height / 2) as i32,
        ))
        .map_err(|e| e.to_string())?;
    menu_win.show().map_err(|e| e.to_string())?;
    let _ = menu_win.set_focus();
    // 窗口显示后再通知前端刷新配置（隐藏时前端可能收不到事件）。
    let _ = app.emit_to("menu", "menu-updated", ());
    // 鼠标保持在圆盘中心，方便向各方向移动选择。
    let _ = win::set_cursor_pos(pos.x, pos.y);
    Ok(())
}

fn apply_hotkey(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let shortcut: tauri_plugin_global_shortcut::Shortcut = hotkey
        .parse()
        .map_err(|_| format!("快捷键格式无效: {hotkey}"))?;
    gs.register(shortcut).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_config(state: State<AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn save_config(app: AppHandle, state: State<AppState>, cfg: Config) -> Result<(), String> {
    let path = config::config_path(app.path().app_config_dir().map_err(|e| e.to_string())?);
    config::save(&path, &cfg)?;
    *state.config.lock().unwrap() = cfg.clone();
    apply_hotkey(&app, &cfg.global_hotkey)?;
    // 同步菜单窗口尺寸：窗口显示中保存时，圆盘会立即刷新，窗口必须跟随，否则会被裁剪。
    resize_menu_window(&app, cfg.menu.size);
    let _ = app.emit_to("menu", "menu-updated", ());
    Ok(())
}

#[derive(serde::Serialize)]
struct MenuData {
    recipes: Vec<Recipe>,
    menu: MenuConfig,
}

/// 返回圆盘菜单需要的全部数据（配方 + 菜单外观配置）。
#[tauri::command]
fn get_menu(state: State<AppState>) -> MenuData {
    let cfg = state.config.lock().unwrap();
    MenuData {
        recipes: cfg.recipes.clone(),
        menu: cfg.menu.clone(),
    }
}

#[tauri::command]
fn get_recipes(state: State<AppState>) -> Vec<Recipe> {
    state.config.lock().unwrap().recipes.clone()
}

#[derive(serde::Serialize)]
struct AppEntry {
    name: String,
    exe: String,
}

/// 列出本机已安装的应用（来源：开始菜单快捷方式），供配置界面选择目标。
#[tauri::command]
fn list_apps() -> Vec<AppEntry> {
    win::list_apps()
        .into_iter()
        .map(|(name, exe)| AppEntry { name, exe })
        .collect()
}

#[tauri::command]
fn run_recipe(app: AppHandle, state: State<AppState>, name: String) -> Result<(), String> {
    let recipe = {
        let cfg = state.config.lock().unwrap();
        cfg.recipes
            .iter()
            .find(|r| r.name == name)
            .cloned()
            .ok_or("未找到配方")?
    };

    let prev = *state.prev_foreground.lock().unwrap();
    hide_menu(&app);
    if let Some(hwnd) = prev {
        if hwnd != 0 {
            let _ = win::set_foreground(hwnd);
        }
    }

    let result = actions::run_steps(&recipe.steps);
    if result.is_err() {
        // 失败时重新弹出菜单，前端在 status 里展示原因。
        let _ = show_menu(&app);
    }
    result
}

#[tauri::command]
fn show_main_window(app: AppHandle) {
    show_main(&app);
}

#[tauri::command]
fn hide_menu_window(app: AppHandle) {
    hide_menu(&app);
}

/// 拾取点击坐标结果：光标位置相对标题匹配窗口的 x/y 轴定位。
#[derive(serde::Serialize)]
struct ClickCoords {
    x: AxisPos,
    y: AxisPos,
}

/// 拾取点击坐标：返回光标位置相对标题匹配窗口的 x/y 轴定位，单位由调用方指定。
#[tauri::command]
fn pick_click_coords(
    title: String,
    x_unit: actions::Unit,
    y_unit: actions::Unit,
) -> Result<ClickCoords, String> {
    let (x, y) = win::cursor_ratio_in_window(&title, x_unit, y_unit)?;
    Ok(ClickCoords { x, y })
}

/// 按 x/y 轴定位在目标窗口内测试点击一次（供配置界面验证坐标）。
#[tauri::command]
fn test_click(title: String, x: AxisPos, y: AxisPos) -> Result<(), String> {
    win::click_in_window(&title, x, y)
}

/// 查询标题匹配窗口的真实标题（不聚焦、不启动）。未找到匹配窗口则报错。
#[tauri::command]
fn get_window_info(title: String) -> Result<String, String> {
    win::find_window_by_title(&title)
        .and_then(win::window_title)
        .ok_or_else(|| format!("未找到“{title}”对应的窗口"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    let _ = show_menu(app);
                }
            })
            .build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            get_recipes,
            get_menu,
            run_recipe,
            show_main_window,
            hide_menu_window,
            list_apps,
            pick_click_coords,
            test_click,
            get_window_info,
        ])
        .setup(|app| {
            let app_dir = app.path().app_config_dir()?;
            let cfg = config::load(app_dir);
            app.manage(AppState {
                config: Mutex::new(cfg.clone()),
                prev_foreground: Mutex::new(None),
            });

            apply_hotkey(app.handle(), &cfg.global_hotkey)
                .map_err(|e| e.to_string())?;

            // 主窗口关闭时隐藏而非退出（常驻托盘）。
            if let Some(win) = app.get_webview_window("main") {
                let w = win.clone();
                win.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w.hide();
                    }
                });
            }

            // 托盘。
            let show_item = MenuItem::with_id(app, "show", "打开配置界面", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Reference to Agent")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
