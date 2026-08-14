//! Windows UI 自动化原语：窗口激活、鼠标位置、按键注入。
//! 窗口句柄统一用 `isize` 表示，避免 windows-sys / windows 两套 HWND 类型耦合。

use std::collections::HashSet;
use std::ffi::c_void;
use std::ptr::null_mut;
use std::thread;
use std::time::Duration;

use windows_sys::Win32::Foundation::{BOOL, CloseHandle, HWND, LPARAM, POINT, RECT};
use windows_sys::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEINPUT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetCursorPos, GetForegroundWindow, GetWindowLongPtrW,
    GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, SetCursorPos, SetForegroundWindow, ShowWindow, GWL_EXSTYLE, SW_RESTORE,
    SW_SHOW, SW_SHOWNORMAL, WS_EX_TOOLWINDOW,
};

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

/// 是否为工具窗口（浮动小面板、无任务栏按钮等），聚焦时应跳过。
fn is_tool_window(hwnd: isize) -> bool {
    unsafe {
        let ex = GetWindowLongPtrW(hwnd as HWND, GWL_EXSTYLE);
        ex & (WS_EX_TOOLWINDOW as isize) != 0
    }
}

/// 是否为置顶窗口（WS_EX_TOPMOST）。置顶窗通常不是常规主窗口，不应抢占“最近激活”的判定。
fn is_topmost(hwnd: isize) -> bool {
    const WS_EX_TOPMOST: isize = 0x0000_0008;
    unsafe {
        let ex = GetWindowLongPtrW(hwnd as HWND, GWL_EXSTYLE);
        ex & WS_EX_TOPMOST != 0
    }
}

/// 从按枚举（Z 序，顶层在前）顺序排列的候选里，挑出“最近激活”的一个：
/// 优先当前前台窗口（精确命中）；其次取第一个非置顶窗口（普通主窗口即最近被带上前台的）；
/// 都无则退而取第一个候选。空切片时各级返回 None，自然安全。
fn pick_front_window(cands: &[isize]) -> Option<isize> {
    let fg = current_foreground();
    cands
        .iter()
        .copied()
        .find(|&h| h == fg)
        .or_else(|| cands.iter().copied().find(|&h| !is_topmost(h)))
        .or_else(|| cands.first().copied())
}

/// 按标题查找目标窗口。优先级：完全相等 > 前缀匹配 > 包含匹配；忽略大小写；
/// 跳过不可见窗口与工具窗口。同一匹配级别有多个窗口时，优先「最近激活」的那一个
/// （当前前台窗口 > Z 序最前的非置顶窗口），避免同标题多实例误中陈旧窗口。
pub fn find_window_by_title(title: &str) -> Option<isize> {
    let needle = title.trim().to_lowercase();
    if needle.is_empty() {
        return None;
    }

    // 收集所有「包含」匹配的候选（排除不可见/工具窗口）。
    let mut collect = CollectData { out: Vec::new() };
    unsafe {
        // 复用 collect_proc：先取全量可见带标题窗口，再过滤。
        EnumWindows(Some(collect_proc), &mut collect as *mut _ as LPARAM);
    }

    // 读取每个候选的真实标题，按匹配质量分桶；同一桶内保持枚举（Z 序）顺序。
    let mut exact: Vec<isize> = Vec::new();
    let mut prefix: Vec<isize> = Vec::new();
    let mut contains: Vec<isize> = Vec::new();
    for &h in &collect.out {
        if is_tool_window(h) {
            continue;
        }
        let mut buf = [0u16; 512];
        let len = unsafe { GetWindowTextW(h as HWND, buf.as_mut_ptr(), buf.len() as i32) };
        let t = String::from_utf16_lossy(&buf[..len.max(0) as usize]);
        let tl = t.to_lowercase();
        if !tl.contains(&needle) {
            continue;
        }
        if tl == needle {
            exact.push(h);
        } else if tl.starts_with(&needle) {
            prefix.push(h);
        } else {
            contains.push(h);
        }
    }
    pick_front_window(&exact)
        .or_else(|| pick_front_window(&prefix))
        .or_else(|| pick_front_window(&contains))
}

/// 返回窗口所属进程 PID。
pub fn window_pid(hwnd: isize) -> u32 {
    let mut pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd as HWND, &mut pid);
    }
    pid
}

/// 返回窗口所属进程的可执行文件完整路径。
fn window_process_path(hwnd: isize) -> Option<String> {
    unsafe {
        let pid = window_pid(hwnd);
        if pid == 0 {
            return None;
        }
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h == 0 {
            return None;
        }
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(h);
        if ok == 0 {
            return None;
        }
        let s = String::from_utf16_lossy(&buf[..len as usize]);
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

struct CollectData {
    out: Vec<isize>,
}

unsafe extern "system" fn collect_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let data = &mut *(lparam as *mut CollectData);
    if IsWindowVisible(hwnd) != 0 && GetWindowTextLengthW(hwnd) > 0 {
        data.out.push(hwnd as isize);
    }
    1
}

/// 按进程 exe 路径精确匹配已打开的窗口；跳过工具窗口，多实例时优先选「最近激活」的那个：
/// 当前前台窗口 > Z 序最前（最近带上前面）的非工具窗口；工具窗口仅作兜底。
fn find_window_by_exe(exe: &str) -> Option<isize> {
    let needle = exe.trim();
    if needle.is_empty() {
        return None;
    }
    let needle = needle.to_lowercase();
    let mut data = CollectData { out: Vec::new() };
    unsafe {
        EnumWindows(Some(collect_proc), &mut data as *mut _ as LPARAM);
    }

    // 收集「可见、exe 精确相等」的候选，保持枚举（Z 序）顺序；工具窗口单独兜底。
    let mut primary: Vec<isize> = Vec::new();
    let mut fallback: Vec<isize> = Vec::new();
    for &h in &data.out {
        let matches = window_process_path(h)
            .map(|p| p.to_lowercase() == needle)
            .unwrap_or(false);
        if !matches {
            continue;
        }
        if is_tool_window(h) {
            fallback.push(h);
        } else {
            primary.push(h);
        }
    }
    pick_front_window(&primary).or_else(|| pick_front_window(&fallback))
}

/* ---------- 已安装应用扫描（开始菜单快捷方式） ---------- */

use windows_sys::core::GUID;
use windows_sys::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_INPROC_SERVER,
};
use windows_sys::Win32::UI::Shell::{
    SHCreateItemFromParsingName, ShellExecuteW, SIGDN_DESKTOPABSOLUTEPARSING, SIGDN_NORMALDISPLAY,
};

// CLSID_ShellLink / IID_IShellLinkW / IID_IPersistFile
const CLSID_SHELL_LINK: GUID = GUID {
    data1: 0x00021401,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xC0, 0, 0, 0, 0, 0, 0, 0x46],
};
const IID_ISHELL_LINK: GUID = GUID {
    data1: 0x000214F9,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xC0, 0, 0, 0, 0, 0, 0, 0x46],
};
const IID_IPERSIST_FILE: GUID = GUID {
    data1: 0x0000010B,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xC0, 0, 0, 0, 0, 0, 0, 0x46],
};

// IID_IShellItem / IID_IEnumShellItems / BHID_EnumItems（枚举 shell:AppsFolder 用）
const IID_ISHELL_ITEM: GUID = GUID {
    data1: 0x43826d1e,
    data2: 0xe718,
    data3: 0x42ee,
    data4: [0xbc, 0x55, 0xa1, 0xe2, 0x61, 0xc3, 0x7b, 0xfe],
};
const IID_IENUM_SHELL_ITEMS: GUID = GUID {
    data1: 0x70629033,
    data2: 0xe363,
    data3: 0x4a28,
    data4: [0xa5, 0x67, 0x0d, 0xb7, 0x80, 0x06, 0xe6, 0xd7],
};
const BHID_ENUM_ITEMS: GUID = GUID {
    data1: 0x94f60519,
    data2: 0x2850,
    data3: 0x4924,
    data4: [0xaa, 0x5a, 0xd1, 0x5e, 0x84, 0x86, 0x80, 0x39],
};

type HRESULT = i32;

#[repr(C)]
struct IShellLinkVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    get_path: unsafe extern "system" fn(*mut c_void, *mut u16, i32, *mut c_void, u32) -> HRESULT,
    _rest: [usize; 18],
}

#[repr(C)]
struct IPersistFileVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    get_class_id: unsafe extern "system" fn(*mut c_void, *mut GUID) -> HRESULT,
    is_dirty: unsafe extern "system" fn(*mut c_void) -> HRESULT,
    load: unsafe extern "system" fn(*mut c_void, *const u16, u32) -> HRESULT,
    _rest: [usize; 4],
}

#[repr(C)]
struct IShellItemVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    bind_to_handler: unsafe extern "system" fn(
        *mut c_void,
        *mut c_void,
        *const GUID,
        *const GUID,
        *mut *mut c_void,
    ) -> HRESULT,
    get_parent: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
    get_display_name: unsafe extern "system" fn(*mut c_void, i32, *mut *mut u16) -> HRESULT,
    get_attributes: unsafe extern "system" fn(*mut c_void, u32, *mut u32) -> HRESULT,
    compare: unsafe extern "system" fn(*mut c_void, *mut c_void, u32, *mut i32) -> HRESULT,
}

#[repr(C)]
struct IEnumShellItemsVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    next: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void, *mut u32) -> HRESULT,
    skip: unsafe extern "system" fn(*mut c_void, u32) -> HRESULT,
    reset: unsafe extern "system" fn(*mut c_void) -> HRESULT,
    clone: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
}

/// 解析 .lnk 快捷方式，用系统 COM（IShellLink）取目标路径。
fn parse_lnk(path: &str) -> Option<String> {
    unsafe {
        let _ = CoInitializeEx(null_mut(), 0); // COINIT_MULTITHREADED

        let mut obj: *mut c_void = null_mut();
        let hr = CoCreateInstance(
            &CLSID_SHELL_LINK,
            null_mut(),
            CLSCTX_INPROC_SERVER,
            &IID_ISHELL_LINK,
            &mut obj,
        );
        if hr < 0 || obj.is_null() {
            return None;
        }

        let mut pf: *mut c_void = null_mut();
        let link_vtbl = *(obj as *const *const IShellLinkVtbl);
        let hr = ((*link_vtbl).query_interface)(obj, &IID_IPERSIST_FILE, &mut pf);
        if hr < 0 || pf.is_null() {
            ((*link_vtbl).release)(obj);
            return None;
        }

        let wpath: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let pf_vtbl = *(pf as *const *const IPersistFileVtbl);
        let hr = ((*pf_vtbl).load)(pf, wpath.as_ptr(), 0);
        ((*pf_vtbl).release)(pf);
        if hr < 0 {
            ((*link_vtbl).release)(obj);
            return None;
        }

        let mut buf = [0u16; 1024];
        let hr = ((*link_vtbl).get_path)(obj, buf.as_mut_ptr(), buf.len() as i32, null_mut(), 0);
        ((*link_vtbl).release)(obj);
        if hr < 0 {
            return None;
        }

        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let s = String::from_utf16_lossy(&buf[..len]);
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

fn collect_lnks(dir: &str, apps: &mut Vec<(String, String)>, seen: &mut HashSet<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_lnks(&path.to_string_lossy(), apps, seen);
            continue;
        }
        if path.extension().map(|e| e.to_string_lossy().to_lowercase()) != Some("lnk".into()) {
            continue;
        }
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        if let Some(exe) = parse_lnk(&path.to_string_lossy()) {
            if !std::path::Path::new(&exe).is_file() {
                continue;
            }
            if seen.insert(exe.to_lowercase()) {
                apps.push((name, exe));
            }
        }
    }
}

/// 读取 IShellItem 指定 SIGDN 的显示名；失败返回 None。
fn shell_item_display_name(item: *mut c_void, sigdn: i32) -> Option<String> {
    unsafe {
        let vtbl = *(item as *const *const IShellItemVtbl);
        let mut buf: *mut u16 = null_mut();
        let hr = ((*vtbl).get_display_name)(item, sigdn, &mut buf);
        if hr < 0 || buf.is_null() {
            return None;
        }
        let mut len = 0;
        while *buf.add(len) != 0 {
            len += 1;
        }
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(buf, len));
        CoTaskMemFree(buf as *const c_void);
        Some(s)
    }
}

/// 枚举商店（MSIX/UWP）应用：来源 shell:AppsFolder 虚拟文件夹。
/// 每个应用记录为 (显示名称, "shell:AppsFolder\<AUMID>")，用于 ShellExecuteW 启动。
fn list_store_apps(apps: &mut Vec<(String, String)>, seen: &mut HashSet<String>) {
    unsafe {
        let _ = CoInitializeEx(null_mut(), 0); // COINIT_MULTITHREADED

        let mut folder: *mut c_void = null_mut();
        let parsing: Vec<u16> = "shell:AppsFolder"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let hr = SHCreateItemFromParsingName(parsing.as_ptr(), null_mut(), &IID_ISHELL_ITEM, &mut folder);
        if hr < 0 || folder.is_null() {
            return;
        }

        let folder_vtbl = *(folder as *const *const IShellItemVtbl);
        let mut enumerator: *mut c_void = null_mut();
        let hr = ((*folder_vtbl).bind_to_handler)(
            folder,
            null_mut(),
            &BHID_ENUM_ITEMS,
            &IID_IENUM_SHELL_ITEMS,
            &mut enumerator,
        );
        ((*folder_vtbl).release)(folder);
        if hr < 0 || enumerator.is_null() {
            return;
        }

        let enum_vtbl = *(enumerator as *const *const IEnumShellItemsVtbl);
        loop {
            let mut item: *mut c_void = null_mut();
            let mut fetched: u32 = 0;
            if ((*enum_vtbl).next)(enumerator, 1, &mut item, &mut fetched) < 0
                || fetched == 0
                || item.is_null()
            {
                break;
            }
            let alias = shell_item_display_name(item, SIGDN_DESKTOPABSOLUTEPARSING)
                .and_then(|s| {
                    let lower = s.to_ascii_lowercase();
                    if lower.starts_with("shell:appsfolder\\") {
                        Some(s)
                    } else if !lower.contains('\\')
                        && !lower.contains('/')
                        && lower.contains('!')
                    {
                        // 裸 AUMID（PackageFamilyName!AppId），补上前缀供 ShellExecuteW 启动。
                        Some(format!("shell:AppsFolder\\{s}"))
                    } else {
                        None
                    }
                });
            if let Some(alias) = alias {
                if let Some(name) = shell_item_display_name(item, SIGDN_NORMALDISPLAY)
                    .filter(|s| !s.trim().is_empty())
                {
                    if seen.insert(alias.to_lowercase()) {
                        apps.push((name, alias));
                    }
                }
            }
            let item_vtbl = *(item as *const *const IShellItemVtbl);
            ((*item_vtbl).release)(item);
        }
        ((*enum_vtbl).release)(enumerator);
    }
}

/// 列出本机已安装的应用（名称, exe 路径），来源于开始菜单快捷方式与商店应用。
/// 商店应用以 "shell:AppsFolder\<AUMID>" 作为启动标识。
/// 应用可能未运行。
pub fn list_apps() -> Vec<(String, String)> {
    let mut apps: Vec<(String, String)> = Vec::new();
    let mut seen = HashSet::new();
    for var in ["APPDATA", "ProgramData"] {
        if let Some(root) = std::env::var(var).ok() {
            let dir = format!("{root}\\Microsoft\\Windows\\Start Menu\\Programs");
            collect_lnks(&dir, &mut apps, &mut seen);
        }
    }
    list_store_apps(&mut apps, &mut seen);
    apps.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    apps
}

/// 将窗口设为前台（绕过 Windows 前台锁的通用技巧）。
/// 返回 Err 表示最终未能把目标窗口置为前台（调用方据此可重试或报错）。
pub fn set_foreground(hwnd: isize) -> Result<(), String> {
    // 最多重试几次，缓解前台锁竞态。
    for attempt in 0..3 {
        unsafe {
            let hwnd = hwnd as HWND;
            // 最小化则还原；不可见则先显示，确保聚焦的对象是可见窗口。
            if IsIconic(hwnd) != 0 {
                ShowWindow(hwnd, SW_RESTORE);
            } else if IsWindowVisible(hwnd) == 0 {
                ShowWindow(hwnd, SW_SHOW);
            }

            // 经典前台激活技巧：把自身线程与目标/前台线程输入队列绑定。
            let fg = GetForegroundWindow();
            let target_thread = GetWindowThreadProcessId(hwnd, null_mut());
            let fg_thread = GetWindowThreadProcessId(fg, null_mut());
            let cur_thread = GetCurrentThreadId();
            let mut attached_target = false;
            let mut attached_fg = false;
            if target_thread != 0 && target_thread != cur_thread {
                AttachThreadInput(cur_thread, target_thread, 1);
                attached_target = true;
            }
            if fg_thread != 0 && fg_thread != cur_thread {
                AttachThreadInput(cur_thread, fg_thread, 1);
                attached_fg = true;
            }

            SetForegroundWindow(hwnd);
            BringWindowToTop(hwnd);

            if attached_target {
                AttachThreadInput(cur_thread, target_thread, 0);
            }
            if attached_fg {
                AttachThreadInput(cur_thread, fg_thread, 0);
            }
        }

        // 验证是否真的成为前台；是则成功返回。
        thread::sleep(Duration::from_millis(20 * (attempt + 1)));
        if current_foreground() == hwnd {
            return Ok(());
        }
    }
    Err(format!("无法将窗口设为前台（句柄 {}）", hwnd))
}

pub struct CursorPos {
    pub x: i32,
    pub y: i32,
}

pub fn cursor_pos() -> Option<CursorPos> {
    let mut pt = POINT { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut pt) };
    if ok == 0 {
        None
    } else {
        Some(CursorPos { x: pt.x, y: pt.y })
    }
}

/// 将鼠标指针移动到屏幕指定坐标（用于把光标置于圆盘中心）。
pub fn set_cursor_pos(x: i32, y: i32) -> Result<(), String> {
    let ok = unsafe { SetCursorPos(x, y) };
    if ok == 0 {
        Err("设置鼠标位置失败".into())
    } else {
        Ok(())
    }
}

/// 当前前台窗口句柄；无前台窗口时返回 0。
pub fn current_foreground() -> isize {
    unsafe { GetForegroundWindow() as isize }
}

/// 窗口在屏幕上的外接矩形（含边框/标题栏，物理像素）。
#[derive(Debug, Clone, Copy)]
pub struct WindowRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl WindowRect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

/// 获取指定窗口的屏幕外接矩形（GetWindowRect）。
pub fn window_rect(hwnd: isize) -> Option<WindowRect> {
    let rect = unsafe {
        let mut r = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if GetWindowRect(hwnd as HWND, &mut r) == 0 {
            return None;
        }
        r
    };
    Some(WindowRect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    })
}

/// 在屏幕坐标 (x, y) 处点击一次（优先保证点击准确）。
/// 流程：记录原光标位置 → 真实移动到目标点 → 左键按下→抬起 → 延迟 → 移回原位置。
/// 这样点击发生时 GetCursorPos 即为目标点，任何依赖「当前光标位置」判定点击的应用
/// 都能正确识别；用户仅看到光标极短暂地闪到目标再回来（点击准确优先）。
pub fn click_at(x: i32, y: i32) -> Result<(), String> {
    // 记录点击前的光标位置，点击后恢复。
    let prev = cursor_pos().ok_or("无法获取当前鼠标位置")?;

    // 真实移动到目标点，保证点击时系统光标就在目标坐标。
    set_cursor_pos(x, y)?;
    thread::sleep(Duration::from_millis(5));

    // 用 SendInput 注入纯按下/抬起（不带 MOVE），点击发生在当前光标位置（即目标点）。
    let click = |flags: u32| INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    let down = click(MOUSEEVENTF_LEFTDOWN);
    let up = click(MOUSEEVENTF_LEFTUP);
    let mut inputs = [down, up];
    let sent = unsafe { SendInput(inputs.len() as u32, inputs.as_mut_ptr(), std::mem::size_of::<INPUT>() as i32) };

    // 点击事件注入后，稍等片刻让目标处理 down/up，再移回原位。
    thread::sleep(Duration::from_millis(10));

    // 无论成败都恢复光标原位。
    let _ = set_cursor_pos(prev.x, prev.y);

    if sent != inputs.len() as u32 {
        return Err(format!("发送鼠标点击失败（仅成功 {sent}/{}）", inputs.len()));
    }
    Ok(())
}

/// 按标题找到窗口后，按「x/y 轴各自独立定位」换算成屏幕坐标并点击一次。
/// 例如 x=Left+50%、y=Bottom+8% → 水平居中、垂直沉底偏上，适合点击聊天输入框。
pub fn click_in_window(
    title: &str,
    x: crate::actions::AxisPos,
    y: crate::actions::AxisPos,
) -> Result<(), String> {
    let hwnd = find_window_by_title(title)
        .ok_or_else(|| format!("未找到“{title}”对应的窗口，无法定位点击位置"))?;
    let rect = window_rect(hwnd).ok_or("无法获取目标窗口矩形")?;
    let (px, py) = axis_to_screen(rect, x, y)?;
    click_at(px, py)
}

/// 把「x/y 轴各自独立定位」换算为屏幕像素坐标。
fn axis_to_screen(
    rect: WindowRect,
    x: crate::actions::AxisPos,
    y: crate::actions::AxisPos,
) -> Result<(i32, i32), String> {
    use crate::actions::{Base, Unit};
    let w = rect.width();
    let h = rect.height();

    // 偏移量：百分比转像素；像素取整并夹取到 [0, span]，避免点出窗口。
    let offset = |v: f64, unit: Unit, span: i32, axis: &str| -> Result<i32, String> {
        match unit {
            Unit::Percent => Ok((span as f64 * v.clamp(0.0, 1.0)).round() as i32),
            Unit::Px => {
                if v < 0.0 {
                    Err(format!("{axis} 轴像素偏移不能为负: {v}"))
                } else {
                    Ok((v.round() as i32).clamp(0, span))
                }
            }
        }
    };

    let sx = match x.base {
        Base::Left => rect.left + offset(x.value, x.unit, w, "x")?,
        Base::Right => rect.right - offset(x.value, x.unit, w, "x")?,
        _ => return Err("x 轴基准必须是 left 或 right".into()),
    };
    let sy = match y.base {
        Base::Top => rect.top + offset(y.value, y.unit, h, "y")?,
        Base::Bottom => rect.bottom - offset(y.value, y.unit, h, "y")?,
        _ => return Err("y 轴基准必须是 top 或 bottom".into()),
    };
    Ok((sx, sy))
}

/// 返回光标位置相对目标窗口的 x/y 轴定位：各轴自动选择「距离较近的基准边」。
/// x_unit/y_unit 指定偏移量单位（percent/px），拾取结果按该单位返回，
/// 使拾取尊重用户在配置里已选择的单位。
pub fn cursor_ratio_in_window(
    title: &str,
    x_unit: crate::actions::Unit,
    y_unit: crate::actions::Unit,
) -> Result<(crate::actions::AxisPos, crate::actions::AxisPos), String> {
    use crate::actions::{AxisPos, Base, Unit};
    let hwnd = find_window_by_title(title)
        .ok_or_else(|| format!("未找到“{title}”对应的窗口"))?;
    let rect = window_rect(hwnd).ok_or("无法获取目标窗口矩形")?;
    let cur = cursor_pos().ok_or("无法获取鼠标位置")?;
    let w = rect.width();
    let h = rect.height();
    if w <= 0 || h <= 0 {
        return Err("目标窗口尺寸异常".into());
    }
    let fx = (cur.x - rect.left) as f64 / w as f64; // 0=左 1=右
    let fy = (cur.y - rect.top) as f64 / h as f64; // 0=上 1=下

    // 基准右/下时用 (1 - 比例) 作为偏移；基准左/上直接用比例。
    // 像素单位则把比例乘上对应跨度。
    let to_val = |ratio: f64, span: i32, unit: Unit| -> f64 {
        match unit {
            Unit::Percent => ratio,
            Unit::Px => ratio * span as f64,
        }
    };

    let x = if fx >= 0.5 {
        AxisPos { base: Base::Right, value: to_val(1.0 - fx, w, x_unit), unit: x_unit }
    } else {
        AxisPos { base: Base::Left, value: to_val(fx, w, x_unit), unit: x_unit }
    };
    let y = if fy >= 0.5 {
        AxisPos { base: Base::Bottom, value: to_val(1.0 - fy, h, y_unit), unit: y_unit }
    } else {
        AxisPos { base: Base::Top, value: to_val(fy, h, y_unit), unit: y_unit }
    };
    Ok((x, y))
}

const ALPHA_KEYS: [Key; 26] = [
    Key::A, Key::B, Key::C, Key::D, Key::E, Key::F, Key::G, Key::H, Key::I, Key::J, Key::K,
    Key::L, Key::M, Key::N, Key::O, Key::P, Key::Q, Key::R, Key::S, Key::T, Key::U, Key::V,
    Key::W, Key::X, Key::Y, Key::Z,
];

const DIGIT_KEYS: [Key; 10] = [
    Key::Num0, Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7,
    Key::Num8, Key::Num9,
];

fn key_from_str(s: &str) -> Result<Key, String> {
    let lower = s.to_ascii_lowercase();
    match lower.as_str() {
        "enter" | "return" => Ok(Key::Return),
        "tab" => Ok(Key::Tab),
        "space" => Ok(Key::Space),
        "escape" | "esc" => Ok(Key::Escape),
        "backspace" => Ok(Key::Backspace),
        "delete" | "del" => Ok(Key::Delete),
        "insert" => Ok(Key::Insert),
        "up" | "uparrow" => Ok(Key::UpArrow),
        "down" | "downarrow" => Ok(Key::DownArrow),
        "left" | "leftarrow" => Ok(Key::LeftArrow),
        "right" | "rightarrow" => Ok(Key::RightArrow),
        "home" => Ok(Key::Home),
        "end" => Ok(Key::End),
        "pageup" => Ok(Key::PageUp),
        "pagedown" => Ok(Key::PageDown),
        "f1" => Ok(Key::F1),
        "f2" => Ok(Key::F2),
        "f3" => Ok(Key::F3),
        "f4" => Ok(Key::F4),
        "f5" => Ok(Key::F5),
        "f6" => Ok(Key::F6),
        "f7" => Ok(Key::F7),
        "f8" => Ok(Key::F8),
        "f9" => Ok(Key::F9),
        "f10" => Ok(Key::F10),
        "f11" => Ok(Key::F11),
        "f12" => Ok(Key::F12),
        other => {
            let chars: Vec<char> = other.chars().collect();
            if chars.len() == 1 {
                let c = chars[0];
                if c.is_ascii_alphabetic() {
                    return Ok(ALPHA_KEYS[(c.to_ascii_uppercase() as u8 - b'A') as usize]);
                }
                if c.is_ascii_digit() {
                    return Ok(DIGIT_KEYS[(c as u8 - b'0') as usize]);
                }
                return Ok(Key::Unicode(c));
            }
            Err(format!("不支持的按键: {other}"))
        }
    }
}

/// 解析形如 "Ctrl+Shift+C" 的组合键并注入。
pub fn press_hotkey(combo: &str) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    let mut ctrl = false;
    let mut shift = false;
    let mut alt = false;
    let mut key: Option<Key> = None;

    for part in combo.split('+') {
        let p = part.trim();
        match p.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "cmd" | "win" | "meta" => ctrl = true,
            "shift" => shift = true,
            "alt" => alt = true,
            _ => key = Some(key_from_str(p)?),
        }
    }
    let key = key.ok_or_else(|| format!("组合键缺少主键: {combo}"))?;

    // 依次按下，最后无论成败都释放修饰键，避免键位卡住。
    let r1 = if ctrl {
        enigo.key(Key::Control, Direction::Press)
    } else {
        Ok(())
    };
    let r2 = if alt {
        enigo.key(Key::Alt, Direction::Press)
    } else {
        Ok(())
    };
    let r3 = if shift {
        enigo.key(Key::Shift, Direction::Press)
    } else {
        Ok(())
    };
    let r4 = enigo.key(key, Direction::Press);
    let r5 = enigo.key(key, Direction::Release);
    let _ = enigo.key(Key::Shift, Direction::Release);
    let _ = enigo.key(Key::Alt, Direction::Release);
    let _ = enigo.key(Key::Control, Direction::Release);

    r1.and(r2)
        .and(r3)
        .and(r4)
        .and(r5)
        .map_err(|e| format!("按键注入失败: {e}"))
}

/// 直接注入一段文本（受当前输入法影响）。
pub fn type_text(text: &str) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo.text(text).map_err(|e| e.to_string())
}

/// 启动外部程序（不等待）。
pub fn launch_app(exe: &str) -> Result<(), String> {
    std::process::Command::new(exe)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("启动 {exe} 失败: {e}"))
}

/// 是否为商店应用标识（"shell:AppsFolder\AUMID"），不是真实 exe 路径。
fn is_store_alias(s: &str) -> bool {
    s.trim_start().to_ascii_lowercase().starts_with("shell:")
}

/// 通过 ShellExecuteW 启动商店应用（AUMID）。
fn launch_store_app(alias: &str) -> Result<(), String> {
    let w: Vec<u16> = alias.encode_utf16().chain(std::iter::once(0)).collect();
    let r = unsafe { ShellExecuteW(0, null_mut(), w.as_ptr(), null_mut(), null_mut(), SW_SHOWNORMAL) };
    if r <= 32 {
        Err(format!("启动商店应用 {alias} 失败"))
    } else {
        Ok(())
    }
}

/// 聚焦已打开的窗口（优先按进程 exe 匹配，其次按标题；不启动新进程）。
/// 商店应用无真实 exe，按标题匹配即可。
pub fn focus_app(title: &str, exe: Option<&str>) -> Result<(), String> {
    if let Some(exe) = exe {
        if !exe.trim().is_empty() && !is_store_alias(exe) {
            if let Some(hwnd) = find_window_by_exe(exe) {
                return set_foreground(hwnd);
            }
        }
    }
    match find_window_by_title(title) {
        Some(hwnd) => set_foreground(hwnd),
        None => Err(format!("未找到“{title}”对应的已打开窗口")),
    }
}

/// 激活目标应用：优先按进程 exe 匹配已开窗口，其次按标题；都没有则尝试启动并等待。
/// 商店应用按标题匹配窗口，未启动时用 ShellExecuteW 按 AUMID 拉起。
pub fn activate_app(title: &str, exe: Option<&str>) -> Result<(), String> {
    let store = exe.map(is_store_alias).unwrap_or(false);
    if let Some(exe) = exe {
        if !exe.trim().is_empty() && !is_store_alias(exe) {
            if let Some(hwnd) = find_window_by_exe(exe) {
                return set_foreground(hwnd);
            }
        }
    }
    if !title.trim().is_empty() {
        if let Some(hwnd) = find_window_by_title(title) {
            return set_foreground(hwnd);
        }
    }
    if let Some(exe) = exe {
        if store {
            launch_store_app(exe)?;
        } else {
            launch_app(exe)?;
        }
        let mut started: Option<isize> = None;
        for _ in 0..40 {
            thread::sleep(Duration::from_millis(150));
            // 找到窗口后先记下句柄；前台激活失败（前台锁）不中止，继续重试，
            // 避免已启动但尚未稳定时提前报错。
            let hwnd = if !store {
                find_window_by_exe(exe)
            } else {
                None
            }
            .or_else(|| {
                if title.trim().is_empty() {
                    None
                } else {
                    find_window_by_title(title)
                }
            });
            if let Some(h) = hwnd {
                started = Some(h);
                if set_foreground(h).is_ok() {
                    return Ok(());
                }
            }
        }
        if let Some(_h) = started {
            Err(format!("已启动 {exe}，但无法将窗口置于前台"))
        } else {
            Err(format!("已启动 {exe}，但窗口未出现"))
        }
    } else {
        Err(format!("未找到“{title}”对应的窗口，且未配置可启动程序"))
    }
}

/// 写入剪贴板文本。
pub fn write_clipboard(text: &str) -> Result<(), String> {
    arboard::Clipboard::new()
        .map_err(|e| e.to_string())
        .and_then(|mut c| c.set_text(text.to_string()).map_err(|e| e.to_string()))
}

/// 读取剪贴板文本。
/// - `Ok(Some(text))`：当前是文本内容（含空字符串文本）。
/// - `Ok(None)`：剪贴板为空或非文本（图片/文件等），无法还原为文本。
/// - `Err(e)`：剪贴板被占用等，短暂重试仍失败。
pub fn read_clipboard() -> Result<Option<String>, String> {
    let mut clip = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    // 剪贴板可能被其他进程短暂占用，重试几次缓解。
    for _ in 0..3 {
        match clip.get_text() {
            Ok(text) => return Ok(Some(text)),
            Err(arboard::Error::ContentNotAvailable) => return Ok(None),
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }
    // 若最终失败，保守返回 None（视为不可还原），回滚成为 no-op，
    // 避免误写空字符串覆盖用户的非文本剪贴板。
    Ok(None)
}
