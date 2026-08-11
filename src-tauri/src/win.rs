//! Windows UI 自动化原语：窗口激活、鼠标位置、按键注入。
//! 窗口句柄统一用 `isize` 表示，避免 windows-sys / windows 两套 HWND 类型耦合。

use std::collections::HashSet;
use std::ffi::c_void;
use std::ptr::null_mut;
use std::thread;
use std::time::Duration;

use windows_sys::Win32::Foundation::{BOOL, CloseHandle, HWND, LPARAM, POINT};
use windows_sys::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetCursorPos, GetForegroundWindow, GetWindowTextLengthW,
    GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible, SetCursorPos,
    SetForegroundWindow, ShowWindow, SW_RESTORE,
};

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let data = &mut *(lparam as *mut EnumerateData);
    if IsWindowVisible(hwnd) == 0 || GetWindowTextLengthW(hwnd) == 0 {
        return 1;
    }
    let mut buf = [0u16; 512];
    let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    let title = String::from_utf16_lossy(&buf[..len.max(0) as usize]);
    let matched = (data.matcher)(&title);
    if matched {
        data.result = Some(hwnd as isize);
        return 0;
    }
    1
}

struct EnumerateData {
    matcher: Box<dyn Fn(&str) -> bool>,
    result: Option<isize>,
}

/// 按标题（模糊、忽略大小写）查找第一个可见顶层窗口，返回句柄。
pub fn find_window_by_title(title: &str) -> Option<isize> {
    let needle = title.trim().to_lowercase();
    if needle.is_empty() {
        return None;
    }
    let mut data = EnumerateData {
        matcher: Box::new(move |t| t.to_lowercase().contains(&needle)),
        result: None,
    };
    unsafe {
        EnumWindows(Some(enum_proc), &mut data as *mut _ as LPARAM);
    }
    data.result
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

/// 按进程 exe 路径精确匹配已打开的窗口。
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
    data.out.into_iter().find(|&h| {
        window_process_path(h)
            .map(|p| p.to_lowercase() == needle)
            .unwrap_or(false)
    })
}

/* ---------- 已安装应用扫描（开始菜单快捷方式） ---------- */

use windows_sys::core::GUID;
use windows_sys::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER};

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

/// 列出本机已安装的应用（名称, exe 路径），来源于开始菜单快捷方式。
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
    apps.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    apps
}

/// 将窗口设为前台（绕过 Windows 前台锁的通用技巧）。
pub fn set_foreground(hwnd: isize) {
    unsafe {
        let hwnd = hwnd as HWND;
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
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

/// 聚焦已打开的窗口（优先按进程 exe 匹配，其次按标题；不启动新进程）。
pub fn focus_app(title: &str, exe: Option<&str>) -> Result<(), String> {
    if let Some(exe) = exe {
        if !exe.trim().is_empty() {
            if let Some(hwnd) = find_window_by_exe(exe) {
                set_foreground(hwnd);
                return Ok(());
            }
        }
    }
    match find_window_by_title(title) {
        Some(hwnd) => {
            set_foreground(hwnd);
            Ok(())
        }
        None => Err(format!("未找到“{title}”对应的已打开窗口")),
    }
}

/// 激活目标应用：优先按进程 exe 匹配已开窗口，其次按标题；都没有则尝试启动 exe 并等待。
pub fn activate_app(title: &str, exe: Option<&str>) -> Result<(), String> {
    if let Some(exe) = exe {
        if !exe.trim().is_empty() {
            if let Some(hwnd) = find_window_by_exe(exe) {
                set_foreground(hwnd);
                return Ok(());
            }
        }
    }
    if !title.trim().is_empty() {
        if let Some(hwnd) = find_window_by_title(title) {
            set_foreground(hwnd);
            return Ok(());
        }
    }
    if let Some(exe) = exe {
        launch_app(exe)?;
        for _ in 0..40 {
            thread::sleep(Duration::from_millis(150));
            if let Some(hwnd) = find_window_by_exe(exe) {
                set_foreground(hwnd);
                return Ok(());
            }
            if !title.trim().is_empty() {
                if let Some(hwnd) = find_window_by_title(title) {
                    set_foreground(hwnd);
                    return Ok(());
                }
            }
        }
        Err(format!("已启动 {exe}，但窗口未出现"))
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
