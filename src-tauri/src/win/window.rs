//! 窗口查找、标题/PID 读取、前台切换与显示器工作区。

use std::ptr::null_mut;
use std::thread;
use std::time::Duration;

use windows_sys::Win32::Foundation::{BOOL, CloseHandle, HWND, LPARAM, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    SetForegroundWindow, ShowWindow, GWL_EXSTYLE, SW_RESTORE, SW_SHOW, WS_EX_TOOLWINDOW,
};

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
        let t = match window_title(h) {
            Some(t) => t,
            None => continue,
        };
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

/// 读取窗口标题（GetWindowTextW 的 UTF-16 解码）。访问某些进程的窗口可能失败，返回 None。
pub fn window_title(hwnd: isize) -> Option<String> {
    unsafe {
        let len = GetWindowTextLengthW(hwnd as HWND);
        if len <= 0 {
            return None;
        }
        let mut buf = vec![0u16; len as usize + 1];
        let read = GetWindowTextW(hwnd as HWND, buf.as_mut_ptr(), buf.len() as i32);
        if read <= 0 {
            return None;
        }
        let title = String::from_utf16_lossy(&buf[..read as usize]);
        if title.is_empty() {
            None
        } else {
            Some(title)
        }
    }
}

/// 返回窗口所属进程 PID。
pub fn window_pid(hwnd: isize) -> u32 {
    let mut pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut pid);
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
pub fn find_window_by_exe(exe: &str) -> Option<isize> {
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

/// 返回包含指定屏幕点的显示器的工作区（排除任务栏，物理像素）。
/// 用于把弹出窗口夹取到可见区域，覆盖多显示器负坐标场景。
pub fn work_area_at(x: i32, y: i32) -> Option<WindowRect> {
    unsafe {
        let pt = POINT { x, y };
        let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        if monitor == 0 {
            return None;
        }
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT { left: 0, top: 0, right: 0, bottom: 0 },
            rcWork: RECT { left: 0, top: 0, right: 0, bottom: 0 },
            dwFlags: 0,
        };
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return None;
        }
        Some(WindowRect {
            left: info.rcWork.left,
            top: info.rcWork.top,
            right: info.rcWork.right,
            bottom: info.rcWork.bottom,
        })
    }
}
