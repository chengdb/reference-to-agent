//! 已安装应用枚举（开始菜单快捷方式 + 商店应用）与应用启动/激活。

use std::collections::HashSet;
use std::ffi::c_void;
use std::ptr::null_mut;
use std::thread;
use std::time::Duration;

use windows_sys::core::GUID;
use windows_sys::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};
use windows_sys::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_INPROC_SERVER,
};
use windows_sys::Win32::UI::Shell::{
    SHCreateItemFromParsingName, ShellExecuteW, SIGDN_DESKTOPABSOLUTEPARSING, SIGDN_NORMALDISPLAY,
};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use super::window::{find_window_by_exe, find_window_by_title, set_foreground, window_title};

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
    compare: unsafe extern "system" fn(*mut c_void, *mut c_void, i32, *mut i32) -> HRESULT,
    _rest: [usize; 18],
}

#[repr(C)]
struct IEnumShellItemsVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    next: unsafe extern "system" fn(
        *mut c_void,
        u32,
        *mut *mut c_void,
        *mut u32,
    ) -> HRESULT,
    skip: unsafe extern "system" fn(*mut c_void, u32) -> HRESULT,
    reset: unsafe extern "system" fn(*mut c_void) -> HRESULT,
    clone: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
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

/// 读取 exe 版本资源里的本地化显示名（FileDescription），
/// 例如 mstsc 的“远程桌面连接”。读取失败或没有有效字符串时返回 None，
/// 调用方回退到快捷方式文件名。
fn exe_display_name(exe: &str) -> Option<String> {
    unsafe {
        let wpath: Vec<u16> = exe.encode_utf16().chain(std::iter::once(0)).collect();
        let mut handle = 0u32;
        let size = GetFileVersionInfoSizeW(wpath.as_ptr(), &mut handle);
        if size == 0 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        if GetFileVersionInfoW(wpath.as_ptr(), handle, buf.len() as u32, buf.as_mut_ptr() as *mut c_void) == 0 {
            return None;
        }
        let data = buf.as_ptr() as *const c_void;

        // 翻译表：每项一个 DWORD（低 16 位语言、高 16 位代码页）。
        let trans_key = "\\VarFileInfo\\Translation"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let mut trans: *mut c_void = null_mut();
        let mut trans_len: u32 = 0;
        if VerQueryValueW(data, trans_key.as_ptr(), &mut trans, &mut trans_len) == 0
            || trans.is_null()
            || trans_len < 4
        {
            return None;
        }
        let pairs = std::slice::from_raw_parts(trans as *const u16, trans_len as usize / 2);
        for chunk in pairs.chunks_exact(2) {
            let (lang, cp) = (chunk[0], chunk[1]);
            let key = format!("\\StringFileInfo\\{lang:04X}{cp:04X}\\FileDescription");
            let wkey: Vec<u16> = key.encode_utf16().chain(std::iter::once(0)).collect();
            let mut val: *mut c_void = null_mut();
            let mut val_len: u32 = 0;
            if VerQueryValueW(data, wkey.as_ptr(), &mut val, &mut val_len) == 0
                || val.is_null()
                || val_len < 2
            {
                continue;
            }
            // 长度（字节）含结尾空字符。
            let chars = val_len as usize / 2 - 1;
            let s = String::from_utf16_lossy(std::slice::from_raw_parts(val as *const u16, chars));
            let trimmed = s.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
        None
    }
}

/// 快捷方式显示名：优先用 exe 版本资源的本地化显示名（FileDescription），
/// 只有在该名字缺失、或与快捷方式文件名 / exe 文件名重复（退化）时才回退到快捷方式文件名。
fn pick_display_name(stem: &str, exe: &str) -> String {
    let exe_stem = std::path::Path::new(exe)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    match exe_display_name(exe) {
        Some(n)
            if !n.eq_ignore_ascii_case(stem) && !n.eq_ignore_ascii_case(&exe_stem) =>
        {
            n
        }
        _ => stem.to_string(),
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
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if stem.is_empty() {
            continue;
        }
        if let Some(exe) = parse_lnk(&path.to_string_lossy()) {
            if !std::path::Path::new(&exe).is_file() {
                continue;
            }
            if seen.insert(exe.to_lowercase()) {
                apps.push((pick_display_name(&stem, &exe), exe));
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
        CoTaskMemFree(buf as *mut c_void);
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
/// 成功时返回聚焦窗口的真实标题（供调用方存入变量），读取标题失败则返回 None。
/// 商店应用无真实 exe，按标题匹配即可。
pub fn focus_app(title: &str, exe: Option<&str>) -> Result<Option<String>, String> {
    if let Some(exe) = exe {
        if !exe.trim().is_empty() && !is_store_alias(exe) {
            if let Some(hwnd) = find_window_by_exe(exe) {
                set_foreground(hwnd)?;
                return Ok(window_title(hwnd));
            }
        }
    }
    match find_window_by_title(title) {
        Some(hwnd) => {
            set_foreground(hwnd)?;
            Ok(window_title(hwnd))
        }
        None => Err(format!("未找到“{title}”对应的已打开窗口")),
    }
}

/// 激活目标应用：优先按进程 exe 匹配已开窗口，其次按标题；都没有则尝试启动并等待。
/// 成功时返回聚焦窗口的真实标题（供调用方存入变量），读取标题失败则返回 None。
/// 商店应用按标题匹配窗口，未启动时用 ShellExecuteW 按 AUMID 拉起。
pub fn activate_app(title: &str, exe: Option<&str>) -> Result<Option<String>, String> {
    let store = exe.map(is_store_alias).unwrap_or(false);
    if let Some(exe) = exe {
        if !exe.trim().is_empty() && !is_store_alias(exe) {
            if let Some(hwnd) = find_window_by_exe(exe) {
                set_foreground(hwnd)?;
                return Ok(window_title(hwnd));
            }
        }
    }
    if !title.trim().is_empty() {
        if let Some(hwnd) = find_window_by_title(title) {
            set_foreground(hwnd)?;
            return Ok(window_title(hwnd));
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
                    return Ok(window_title(h));
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
