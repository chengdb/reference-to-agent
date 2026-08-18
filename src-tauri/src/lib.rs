mod actions;
mod config;
mod win;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, State, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut as GsShortcut, ShortcutState};

use config::{Config, MenuConfig, Recipe};
use actions::{AxisPos, ConfirmChoice, Step, describe_step};

/// 弹出层类型：圆盘菜单 / 完整配方列表。用于失败后恢复用户正在使用的弹出层。
#[derive(Clone, Copy, PartialEq)]
enum PopupKind {
    Menu,
    List,
}

/// 人工确认请求：确认窗口展示的内容（一次配方执行中可多次更新）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmRequest {
    recipe_name: String,
    /// 全流程递增的步骤序号（含 if 分支内步骤）。
    step_seq: u32,
    step_desc: String,
}

struct AppState {
    config: Mutex<Config>,
    /// 菜单显示前的前台窗口（恢复焦点用）。
    prev_foreground: Mutex<Option<isize>>,
    /// 最近一次显示的弹出层（配方执行失败时原样恢复）。
    last_popup: Mutex<PopupKind>,
    /// 配方执行互斥锁：同一时刻只允许一个配方运行，
    /// 避免双击/连按导致两个配方的按键、剪贴板、鼠标注入交错乱序。
    exec_lock: Mutex<()>,
    /// 人工确认应答通道：确认窗口按键（Enter / Shift+Enter / Esc）时写入，阻塞线程据此继续。
    /// 每次询问前重新放入新通道，答过一次即被取走，防止一次误按拖到下一个确认。
    confirm_tx: Mutex<Option<std::sync::mpsc::Sender<ConfirmChoice>>>,
    /// 当前待确认的请求内容（确认窗口可主动拉取，兜底事件时序问题）。
    pending_confirm: Mutex<Option<ConfirmRequest>>,
    /// 轻提示代数计数：每次 show_toast 递增；只有最新一次提示的计时线程才允许隐藏窗口，
    /// 避免连续多条提示时“较早的 3 秒计时把较新的提示先隐藏掉”。
    toast_gen: AtomicU32,
}

impl AppState {
    /// 是否有配方正在执行（不占用锁，只查询）。
    fn is_running(&self) -> bool {
        self.exec_lock.try_lock().is_err()
    }
}

fn hide_menu(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("menu") {
        let _ = win.hide();
    }
}

fn hide_list(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("list") {
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
        hide_list(app);
        let _ = menu_win.hide();
        return Ok(());
    }
    // 显示菜单前收起列表，避免两个弹出层叠放。
    hide_list(app);

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
    *state.last_popup.lock().unwrap() = PopupKind::Menu;
    // 窗口显示后再通知前端刷新配置（隐藏时前端可能收不到事件）。
    let _ = app.emit_to("menu", "menu-updated", ());
    // 鼠标保持在圆盘中心，方便向各方向移动选择。
    let _ = win::set_cursor_pos(pos.x, pos.y);
    Ok(())
}

/// 在鼠标附近弹出完整配方列表，并记录当前前台窗口用于执行后恢复。
fn show_recipe_list(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let list_win = app.get_webview_window("list").ok_or("配方列表窗口不存在")?;
    let main_win = app.get_webview_window("main").ok_or("主窗口不存在")?;
    let menu_win = app.get_webview_window("menu").ok_or("菜单窗口不存在")?;

    // 列表已显示则收起（toggle）。
    if list_win.is_visible().unwrap_or(false) {
        let _ = list_win.hide();
        return Ok(());
    }
    // 显示列表前收起圆盘，避免两个弹出层叠放。
    hide_menu(app);

    let fg = win::current_foreground();
    if fg != 0 {
        let fg_pid = win::window_pid(fg);
        let self_pids = [
            list_win.hwnd().ok().map(|h| win::window_pid(h.0 as isize)),
            main_win.hwnd().ok().map(|h| win::window_pid(h.0 as isize)),
            menu_win.hwnd().ok().map(|h| win::window_pid(h.0 as isize)),
        ];
        if !self_pids.iter().flatten().any(|p| *p == fg_pid) {
            *state.prev_foreground.lock().unwrap() = Some(fg);
        }
    }

    let pos = win::cursor_pos().ok_or("无法获取鼠标位置")?;
    let size = list_win.inner_size().map_err(|e| e.to_string())?;
    let (w, h) = (size.width as i32, size.height as i32);
    // 列表左上角从鼠标右下偏移弹出；超出光标所在显示器工作区时向内收拢，保证可见。
    let mut x = pos.x + 16;
    let mut y = pos.y + 16;
    if let Some(area) = win::work_area_at(pos.x, pos.y) {
        if x + w > area.right {
            x = pos.x - 16 - w;
        }
        if y + h > area.bottom {
            y = pos.y - 16 - h;
        }
        x = x.clamp(area.left, (area.right - w).max(area.left));
        y = y.clamp(area.top, (area.bottom - h).max(area.top));
    }
    list_win
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    list_win.show().map_err(|e| e.to_string())?;
    let _ = list_win.set_focus();
    *state.last_popup.lock().unwrap() = PopupKind::List;
    // 窗口显示后再通知前端刷新配方列表。
    let _ = app.emit_to("list", "list-updated", ());
    Ok(())
}

/// 注册全局快捷键：圆盘菜单快捷键 + 完整配方列表快捷键。
/// 每个快捷键绑定各自的处理器，互不干扰；相同快捷键只注册一次（列表键与菜单键相同时跳过）。
fn apply_hotkeys(app: &AppHandle) -> Result<(), String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    let cfg = app.state::<AppState>().config.lock().unwrap().clone();

    let menu_shortcut: GsShortcut = cfg
        .global_hotkey
        .parse()
        .map_err(|_| format!("快捷键格式无效: {}", cfg.global_hotkey))?;
    gs.on_shortcut(menu_shortcut, |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            // 配方执行中不弹出窗口（避免干扰注入），改为顶部轻提示。
            if app.state::<AppState>().is_running() {
                show_toast(app, "配方正在执行中");
                return;
            }
            let _ = show_menu(app);
        }
    })
    .map_err(|e| e.to_string())?;

    if !cfg.list_hotkey.trim().is_empty() {
        let list_shortcut: GsShortcut = cfg
            .list_hotkey
            .parse()
            .map_err(|_| format!("列表快捷键格式无效: {}", cfg.list_hotkey))?;
        if list_shortcut.id() != menu_shortcut.id() {
            gs.on_shortcut(list_shortcut, |app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    // 同上：执行中不弹出窗口，改为顶部轻提示。
                    if app.state::<AppState>().is_running() {
                        show_toast(app, "配方正在执行中");
                        return;
                    }
                    let _ = show_recipe_list(app);
                }
            })
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
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
    apply_hotkeys(&app)?;
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
async fn run_recipe(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<(), String> {
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
    hide_list(&app);
    if let Some(hwnd) = prev {
        if hwnd != 0 {
            let _ = win::set_foreground(hwnd);
        }
    }

    // 防止上一次异常残留：执行开始时确保确认窗口收起、通道清空。
    if let Some(win) = app.get_webview_window("confirm") {
        let _ = win.hide();
    }
    *state.confirm_tx.lock().unwrap() = None;
    *state.pending_confirm.lock().unwrap() = None;

    let recipe_name = recipe.name.clone();
    let recipe_confirm = recipe.confirm;
    let steps = recipe.steps.clone();

    // 串行化 + 执行：在阻塞线程内获取并持有互斥锁，避免 async 未来持有非 Send 的
    // MutexGuard（确认流程会在等待用户按键时阻塞该线程，但不会卡住主事件循环）。
    let app2 = app.clone();
    let result: Result<(), String> = tauri::async_runtime::spawn_blocking(move || {
        let state = app2.state::<AppState>();
        let _guard = state
            .exec_lock
            .try_lock()
            .map_err(|_| "已有配方正在执行，请等待完成后再试")?;
        // 确认回调：run_steps_confirmed 已按「配方开关 + 步骤开关」双重过滤，
        // 走到这里说明需要弹窗等待用户按键。
        let mut ask = |seq: usize, step: &Step| {
            ask_confirm(&app2, &recipe_name, seq, step, prev)
        };
        actions::run_steps_confirmed(&steps, recipe_confirm, &mut ask)
    })
    .await
    .map_err(|e| format!("配方执行线程异常: {e}"))?;

    // 清理确认状态（无论成功失败）。
    *state.confirm_tx.lock().unwrap() = None;
    *state.pending_confirm.lock().unwrap() = None;
    if let Some(win) = app.get_webview_window("confirm") {
        let _ = win.hide();
    }

    if result.is_err() {
        let msg = result.as_ref().unwrap_err();
        if msg == actions::CANCELED {
            // 用户取消：不重弹菜单/列表，只显示取消提示，3 秒后自动关闭。
            crate::debug_log!("run_recipe: user cancelled");
            show_cancel_toast(&app, msg);
            return Ok(());
        }
        // 执行失败：重新弹出用户正在使用的弹出层，前端在 status 里展示原因。
        let kind = *state.last_popup.lock().unwrap();
        match kind {
            PopupKind::Menu => {
                let _ = show_menu(&app);
            }
            PopupKind::List => {
                let _ = show_recipe_list(&app);
            }
        }
    }
    result
}

/// 取消执行后的轻提示：复用提示窗口显示一条消息，3 秒后自动关闭，不抢占焦点。
fn show_cancel_toast(app: &AppHandle, message: &str) {
    show_toast(app, message)
}

/// 通用轻提示：用独立提示窗口在「屏幕顶部居中」显示一条消息（无阴影，不抢占焦点），
/// 3 秒后自动关闭。用于取消执行提示，以及圆盘菜单/配方列表执行时被快捷键唤起的警告。
/// 使用独立 toast 窗口，避免与确认流程共享窗口造成的「位置残留/闪现」。
fn show_toast(app: &AppHandle, message: &str) {
    let Some(win) = app.get_webview_window("toast") else {
        return;
    };
    // 先在隐藏状态下定位到顶部居中，再显示：避免先显示旧位置再跳转造成的闪烁。
    position_toast_top(&win);
    let _ = win.show();
    let _ = app.emit_to("toast", "toast-message", message);
    // 只有最新一次提示的计时线程才有权隐藏窗口，避免连续提示时较早线程把较新的提示提前隐藏。
    let gen = app.state::<AppState>().toast_gen.fetch_add(1, Ordering::Relaxed) + 1;
    let app2 = app.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(Duration::from_secs(3));
        if app2.state::<AppState>().toast_gen.load(Ordering::Relaxed) != gen {
            return;
        }
        if let Some(w) = app2.get_webview_window("toast") {
            let _ = w.hide();
        }
    });
}

/// 菜单/列表窗口唤起警告：后端发出警告提示（顶部居中，3 秒自动消失）。
#[tauri::command]
fn show_warning(app: AppHandle, message: String) {
    show_toast(&app, &message);
}

/// 把提示窗口定位到「光标所在显示器工作区的顶部居中」。
fn position_toast_top(win: &tauri::WebviewWindow) {
    let Some(pos) = win::cursor_pos() else {
        return;
    };
    let Ok(size) = win.inner_size() else {
        return;
    };
    let (w, _h) = (size.width as i32, size.height as i32);
    let mut x = pos.x - w / 2;
    if let Some(area) = win::work_area_at(pos.x, pos.y) {
        x = x.clamp(area.left, (area.right - w).max(area.left));
        let _ = win.set_position(PhysicalPosition::new(x, area.top + 24));
        return;
    }
    let _ = win.set_position(PhysicalPosition::new(x, 24));
}

/// 判断窗口是否是本应用的窗口（主窗口/圆盘/列表/确认窗口）。
fn is_own_window(app: &AppHandle, hwnd: isize) -> bool {
    if hwnd == 0 {
        return false;
    }
    let pid = win::window_pid(hwnd);
    ["main", "menu", "list", "confirm", "toast"].iter().any(|label| {
        app.get_webview_window(label)
            .and_then(|w| w.hwnd().ok())
            .map(|h| win::window_pid(h.0 as isize) == pid)
            .unwrap_or(false)
    })
}

/// 确认窗口按键回调（前端兜底路径）：choice 为 "confirm" / "confirmAll" / "cancel"。
#[tauri::command]
fn confirm_step(app: AppHandle, state: State<AppState>, choice: String) {
    let choice = match choice.as_str() {
        "confirmAll" => ConfirmChoice::ConfirmAll,
        "cancel" => ConfirmChoice::Cancel,
        _ => ConfirmChoice::Confirm,
    };
    if let Some(tx) = state.confirm_tx.lock().unwrap().take() {
        let _ = tx.send(choice);
    }
    *state.pending_confirm.lock().unwrap() = None;
    if let Some(win) = app.get_webview_window("confirm") {
        let _ = win.hide();
    }
}

/// 取消当前等待中的确认（确认窗口因失焦被隐藏时调用，视为用户取消）。
fn cancel_confirm(app: &AppHandle) {
    if let Some(tx) = app.state::<AppState>().confirm_tx.lock().unwrap().take() {
        let _ = tx.send(ConfirmChoice::Cancel);
    }
    *app.state::<AppState>().pending_confirm.lock().unwrap() = None;
}

/// 确认窗口主动拉取当前待确认的请求（兜底事件先于监听注册的时序问题）。
#[tauri::command]
fn get_confirm_request(state: State<AppState>) -> Option<ConfirmRequest> {
    state.pending_confirm.lock().unwrap().clone()
}

/// 在配方执行线程中展示确认窗口并等待用户在「确认」窗口按键。
/// 返回 Confirm 继续执行、ConfirmAll 继续执行且后续不再询问、Cancel 中止整个配方。
/// 确认前记录当前前台窗口（步骤注入的目标），确认后把焦点还给它再继续执行。
fn ask_confirm(
    app: &AppHandle,
    recipe_name: &str,
    seq: usize,
    step: &Step,
    fallback_foreground: Option<isize>,
) -> Result<ConfirmChoice, String> {
    let confirm_win = app.get_webview_window("confirm").ok_or("确认窗口不存在")?;

    // 确认窗口弹出前的前台窗口：步骤注入的目标。若前台落在自身窗口上
    // （例如连续确认间隙未恢复），用配方启动时的前台窗口兜底。
    let target = {
        let fg = win::current_foreground();
        if is_own_window(app, fg) {
            fallback_foreground.unwrap_or(0)
        } else {
            fg
        }
    };

    let (tx, rx) = std::sync::mpsc::channel::<ConfirmChoice>();
    let req = ConfirmRequest {
        recipe_name: recipe_name.to_string(),
        step_seq: seq as u32,
        step_desc: describe_step(step),
    };
    *app.state::<AppState>().confirm_tx.lock().unwrap() = Some(tx.clone());
    *app.state::<AppState>().pending_confirm.lock().unwrap() = Some(req.clone());

    // 窗口定位：优先居中于「下一步要操作的目标窗口」——用户正看着的地方。
    // 目标窗口取不到（例如窗口最小化/已关闭）时，回退为光标所在显示器的中心。
    let size = confirm_win.inner_size().map_err(|e| e.to_string())?;
    let (w, h) = (size.width as i32, size.height as i32);
    let mut x: i32 = 0;
    let mut y: i32 = 0;
    let mut fallback = false;
    if target != 0 {
        match win::window_rect(target) {
            Some(rect) if rect.width() > 0 && rect.height() > 0 => {
                let cx = rect.left + rect.width() / 2;
                let cy = rect.top + rect.height() / 2;
                x = cx - w / 2;
                y = cy - h / 2;
                // 夹取到目标窗口所在显示器的工作区，保证完全可见（跨屏窗口取中心所在的屏）。
                if let Some(area) = win::work_area_at(cx, cy) {
                    x = x.clamp(area.left, (area.right - w).max(area.left));
                    y = y.clamp(area.top, (area.bottom - h).max(area.top));
                }
            }
            _ => fallback = true,
        }
    } else {
        fallback = true;
    }
    if fallback {
        // 回退：光标所在显示器中心。
        let pos = win::cursor_pos().ok_or("无法获取鼠标位置")?;
        x = pos.x - w / 2;
        y = pos.y - h / 2;
        if let Some(area) = win::work_area_at(pos.x, pos.y) {
            x = x.clamp(area.left, (area.right - w).max(area.left));
            y = y.clamp(area.top, (area.bottom - h).max(area.top));
        }
    }
    // 先安装全局键盘钩子（同步等待装好）再显示窗口：从窗口出现的第一刻起，
    // Enter/Esc 就必定能被捕获（不依赖窗口焦点时序），且被吞掉不会漏进目标应用。
    crate::debug_log!(
        "ask_confirm: seq={seq} desc={} installing hook",
        describe_step(step)
    );
    let _hook = win::ConfirmKeyHook::install(tx);
    crate::debug_log!("ask_confirm: hook installed, showing window");
    confirm_win
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    confirm_win.show().map_err(|e| e.to_string())?;
    let _ = confirm_win.set_focus();
    // 立即下发提示内容（在焦点重试之前），确保窗口一出现就显示确认信息。
    let _ = app.emit_to("confirm", "confirm-request", &req);
    // 透明置顶窗口的焦点不总是立刻生效（钩子已兜底输入，这里只强化前端按键路径）。
    for _ in 0..10 {
        if confirm_win.is_focused().unwrap_or(false) {
            break;
        }
        std::thread::sleep(Duration::from_millis(40));
        let _ = confirm_win.set_focus();
    }

    // 等待用户按键。超时兜底，避免确认窗口异常导致配方永久挂起。
    let choice = match rx.recv_timeout(Duration::from_secs(300)) {
        Ok(v) => v,
        Err(_) => {
            crate::debug_log!("ask_confirm: TIMEOUT waiting for key");
            let _ = confirm_win.hide();
            *app.state::<AppState>().confirm_tx.lock().unwrap() = None;
            *app.state::<AppState>().pending_confirm.lock().unwrap() = None;
            return Err("等待确认超时，已中止执行".to_string());
        }
    };
    crate::debug_log!("ask_confirm: got response {choice:?}");

    // 用户按过 Enter/Shift+Enter。Enter 不是修饰键，不会污染后续注入；
    // 只有 Shift+Enter 残留的 Shift 才需要等其松开，避免 Alt+L 复制被当成 Alt+Shift+L 而失败。
    // 普通 Enter 瞬间返回（零延迟），Shift+Enter 也只需很短的松开等待，不会造成明显卡顿。
    win::wait_for_shift_released();

    // 窗口本身已在按键回调里隐藏，这里兜底隐藏。
    let _ = confirm_win.hide();
    *app.state::<AppState>().pending_confirm.lock().unwrap() = None;
    // 只有「普通确认」才把焦点还给注入目标（供用户阅读下一个确认、并让下一个确认
    // 窗口定位在目标窗口上）。
    // 「确认全部」时配方会立刻接着执行后续步骤（其中通常有 focusApp/click 等自会
    // 建立焦点），此时若再强行把焦点抢回上一个窗口，会与配方的聚焦步骤竞争，导致
    // 后面的键盘/鼠标注入落到错误窗口，表现为步骤“乱序”。故 ConfirmAll 不恢复焦点。
    if choice == ConfirmChoice::Confirm && target != 0 {
        let _ = win::set_foreground(target);
    }
    Ok(choice)
}

#[tauri::command]
fn show_main_window(app: AppHandle) {
    show_main(&app);
}

#[tauri::command]
fn hide_menu_window(app: AppHandle) {
    hide_menu(&app);
}

#[tauri::command]
fn hide_list_window(app: AppHandle) {
    hide_list(&app);
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
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            get_recipes,
            get_menu,
            run_recipe,
            confirm_step,
            get_confirm_request,
            show_warning,
            show_main_window,
            hide_menu_window,
            hide_list_window,
            list_apps,
            pick_click_coords,
            test_click,
            get_window_info,
        ])
        .setup(|app| {
            let app_dir = app.path().app_config_dir()?;
            let cfg = config::load(app_dir);
            // 尽早注册全局状态：tauri.conf.json 里声明的窗口（main/menu/list）会在 setup
            // 执行的同时加载 JS，若这些窗口在 app.manage() 之前调用任何带 State 的命令
            // （get_config / get_menu / get_recipes），会报 "state not managed"。所以这里
            // 必须放在 setup 最前面，且前端侧用 invokeWithRetry 兜底竞态（见 src/utils/invokeRetry.ts）。
            app.manage(AppState {
                config: Mutex::new(cfg.clone()),
                prev_foreground: Mutex::new(None),
                last_popup: Mutex::new(PopupKind::Menu),
                exec_lock: Mutex::new(()),
                confirm_tx: Mutex::new(None),
                pending_confirm: Mutex::new(None),
                toast_gen: AtomicU32::new(0),
            });

            apply_hotkeys(app.handle()).map_err(|e| e.to_string())?;

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

            // 弹出层窗口（圆盘 / 配方列表）失去焦点即自动收起：
            // 在 OS 层监听 Focused 事件，比前端 window blur 更可靠（透明置顶窗口的
            // WebView2 不一定派发 DOM blur/keydown）。
            for label in ["menu", "list"] {
                if let Some(win) = app.get_webview_window(label) {
                    let w = win.clone();
                    win.on_window_event(move |event| {
                        if let WindowEvent::Focused(false) = event {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            }
                        }
                    });
                }
            }

            // 确认窗口失去焦点视为取消当前确认（用户切走/点别处），
            // 由后端直接回写应答通道，比等前端 blur 事件更可靠。
            if let Some(win) = app.get_webview_window("confirm") {
                let w = win.clone();
                let app2 = app.handle().clone();
                win.on_window_event(move |event| {
                    if let WindowEvent::Focused(false) = event {
                        if w.is_visible().unwrap_or(false) {
                            cancel_confirm(&app2);
                            let _ = w.hide();
                        }
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
