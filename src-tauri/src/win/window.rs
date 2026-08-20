//! 窗口查找、标题/PID 读取、前台切换与显示器工作区。

use std::ffi::c_void;
use std::ptr::null_mut;
use std::thread;
use std::time::Duration;

use windows_sys::core::GUID;
use windows_sys::Win32::Foundation::{BOOL, CloseHandle, HWND, LPARAM, POINT, RECT};
use windows_sys::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};
use windows_sys::Win32::UI::Shell::VirtualDesktopManager;
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

/// IVirtualDesktopManager 的 IID（{A5CD92FF-29BE-454C-8D04-D82879FB3F1B}）。
/// windows-sys 0.52 未生成该 COM 接口的方法表，这里手工定义 vtable。
const IID_I_VIRTUAL_DESKTOP_MANAGER: GUID =
    GUID::from_u128(0xa5cd92ff_29be_454c_8d04_d82879fb3f1b);

type VdRelease = unsafe extern "system" fn(*mut c_void) -> u32;
type VdIsOnCurrentVirtualDesktop = unsafe extern "system" fn(*mut c_void, isize, *mut i32) -> i32;

/// IVirtualDesktopManager 的 vtable 布局（IUnknown 三方法 + 三个接口方法）。
#[repr(C)]
struct IVirtualDesktopManagerVtbl {
    _query_interface: usize,
    _add_ref: usize,
    release: VdRelease,
    is_window_on_current_virtual_desktop: VdIsOnCurrentVirtualDesktop,
    _get_window_desktop_id: usize,
    _move_window_to_desktop: usize,
}

/// 「窗口是否在当前虚拟桌面」判定器（IVirtualDesktopManager，Win10+）。
/// EnumWindows 会枚举到所有虚拟桌面的窗口，多桌面场景需要用它过滤。
/// 创建失败（旧系统、COM 初始化失败等）时 mgr 为 None，is_current 一律返回 None，
/// 调用方应视为「无法区分、不过滤」。Drop 时释放接口并按需 CoUninitialize。
struct VirtualDesktopFilter {
    mgr: Option<*mut c_void>,
    own_com_init: bool,
}

impl VirtualDesktopFilter {
    fn new() -> Self {
        unsafe {
            // S_OK(0) 表示本次由我们完成初始化，之后要配对 CoUninitialize；
            // RPC_E_CHANGED_MODE（线程已被初始化为 MTA）等也照常尝试创建，
            // VirtualDesktopManager 可跨套间使用，失败则降级为 None。
            let hr_init = CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED as u32);
            let own_com_init = hr_init == 0;
            let mut mgr: *mut c_void = null_mut();
            let hr = CoCreateInstance(
                &VirtualDesktopManager,
                null_mut(),
                CLSCTX_ALL,
                &IID_I_VIRTUAL_DESKTOP_MANAGER,
                &mut mgr,
            );
            let mgr = if hr == 0 && !mgr.is_null() { Some(mgr) } else { None };
            Self { mgr, own_com_init }
        }
    }

    /// 窗口是否在当前虚拟桌面；查询失败返回 None（调用方按「不过滤」处理）。
    fn is_current(&self, hwnd: isize) -> Option<bool> {
        unsafe {
            let mgr = self.mgr?;
            let vtbl = *(mgr as *const *const IVirtualDesktopManagerVtbl);
            let mut on: i32 = 0;
            let hr = ((*vtbl).is_window_on_current_virtual_desktop)(mgr, hwnd, &mut on);
            if hr == 0 {
                Some(on != 0)
            } else {
                None
            }
        }
    }
}

impl Drop for VirtualDesktopFilter {
    fn drop(&mut self) {
        unsafe {
            if let Some(mgr) = self.mgr.take() {
                let vtbl = *(mgr as *const *const IVirtualDesktopManagerVtbl);
                ((*vtbl).release)(mgr);
            }
            if self.own_com_init {
                CoUninitialize();
            }
        }
    }
}

/// 从按枚举（Z 序，顶层在前）顺序排列的候选里挑一个，优先级：
/// 1. 当前前台窗口（精确命中）；
/// 2. 当前虚拟桌面上的候选：第一个非置顶窗口（普通主窗口即最近被带上前台的），否则第一个；
/// 3. 其他虚拟桌面上的候选：同上规则兜底。
/// 虚拟桌面判定不可用时所有候选都落入「当前桌面」桶，行为与未区分桌面时一致。
/// vd 为惰性槽：只有候选超过 1 个、确实需要区分桌面时才创建 COM 对象，
/// 同一轮查找的多个匹配桶共享同一个实例，避免重复 CoCreateInstance 开销。
fn pick_front_window(cands: &[isize], vd: &mut Option<VirtualDesktopFilter>) -> Option<isize> {
    let fg = current_foreground();
    if let Some(h) = cands.iter().copied().find(|&h| h == fg) {
        return Some(h);
    }
    // 0/1 个候选时桌面分桶不影响结果，直接返回，省掉 COM 初始化。
    if cands.len() <= 1 {
        return pick_from_bucket(cands);
    }
    let vd = vd.get_or_insert_with(VirtualDesktopFilter::new);
    let mut current_desktop: Vec<isize> = Vec::new();
    let mut other_desktop: Vec<isize> = Vec::new();
    for &h in cands {
        // 判定失败按「在当前桌面」处理，避免把窗口误过滤掉。
        if vd.is_current(h).unwrap_or(true) {
            current_desktop.push(h);
        } else {
            other_desktop.push(h);
        }
    }
    pick_from_bucket(&current_desktop).or_else(|| pick_from_bucket(&other_desktop))
}

/// 桶内挑选：优先第一个非置顶窗口，兜底取第一个。空切片自然返回 None。
fn pick_from_bucket(cands: &[isize]) -> Option<isize> {
    cands
        .iter()
        .copied()
        .find(|&h| !is_topmost(h))
        .or_else(|| cands.first().copied())
}

/// 按标题查找目标窗口。优先级：完全相等 > 前缀匹配 > 包含匹配；忽略大小写；
/// 跳过不可见窗口与工具窗口。同一匹配级别有多个窗口时，优先「当前虚拟桌面」
/// 再按「最近激活」（当前前台窗口 > Z 序最前的非置顶窗口），
/// 避免多桌面/多实例场景误中其他桌面或陈旧的窗口。
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
    // 惰性共享同一个虚拟桌面判定器：哪个桶先出现多候选才创建。
    let mut vd: Option<VirtualDesktopFilter> = None;
    pick_front_window(&exact, &mut vd)
        .or_else(|| pick_front_window(&prefix, &mut vd))
        .or_else(|| pick_front_window(&contains, &mut vd))
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

/// 按进程 exe 路径精确匹配已打开的窗口；跳过工具窗口，多实例时优先「当前虚拟桌面」，
/// 再按「最近激活」选择：当前前台窗口 > Z 序最前（最近带上前面）的非工具窗口；
/// 工具窗口仅作兜底。
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

    // 收集「可见、exe 精确相等」的候选，保持枚举（Z 序）顺序。
    // OpenProcess + QueryFullProcessImageNameW 开销较大，工具窗口仅作兜底，
    // 先只查常规窗口；常规窗口一个都不匹配时，才为工具窗口付出进程查询开销。
    let exe_matches = |h: isize| {
        window_process_path(h)
            .map(|p| p.to_lowercase() == needle)
            .unwrap_or(false)
    };
    let mut primary: Vec<isize> = Vec::new();
    let mut tool_windows: Vec<isize> = Vec::new();
    for &h in &data.out {
        if is_tool_window(h) {
            tool_windows.push(h);
        } else if exe_matches(h) {
            primary.push(h);
        }
    }
    let fallback: Vec<isize> = if primary.is_empty() {
        tool_windows.into_iter().filter(|&h| exe_matches(h)).collect()
    } else {
        Vec::new()
    };
    let mut vd: Option<VirtualDesktopFilter> = None;
    pick_front_window(&primary, &mut vd).or_else(|| pick_front_window(&fallback, &mut vd))
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
