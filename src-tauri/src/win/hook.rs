//! 人工确认用的低层键盘钩子与调试日志。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_RETURN, VK_SHIFT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, HC_ACTION, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
};

use crate::actions::ConfirmChoice;

/// 钩子回调与安装线程之间的发送端（同一时刻最多一个确认在等待，故用静态）。
/// 首次应答后即被 take 走（置 None），起到“本提示只应答一次”的作用；
/// 下一次提示安装钩子时重新放入。
static HOOK_TX: std::sync::Mutex<Option<mpsc::Sender<ConfirmChoice>>> =
    std::sync::Mutex::new(None);

/// 调试日志（写入 %TEMP%/confirm_hook_debug.log），用于排查“确认回车偶发不生效”。
/// 写失败静默忽略。仅 debug 构建（tauri dev）下生效；release 构建由宏编译为空，见下方 debug_log!。
#[cfg(debug_assertions)]
pub(crate) fn debug_log(msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(std::env::temp_dir().join("confirm_hook_debug.log"))
    {
        let _ = writeln!(
            f,
            "[{}] {}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
            msg
        );
    }
}

/// 调试日志宏：参数按 `format!` 语义展开。
/// - debug 构建：转发到 `win::debug_log`；
/// - release 构建：编译为空实现，调用点不产生 `format!` 分配，也不做任何磁盘写入，
///   彻底消除时序敏感配方热路径上的开销。
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        $crate::win::debug_log(&format!($($arg)*))
    };
}
#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => { () };
}

/// 低层键盘钩子：人工确认等待期间截获 Enter/Esc。
/// 不依赖确认窗口是否已拿到系统键盘焦点（解决“首次回车落空需要按两次”的焦点时序问题），
/// 并且会吞掉这两个键，防止漏掉的回车误注入到目标应用。
/// 每个提示接受「第一次按下」即应答（take 发送端保证只应答一次），
/// 不依赖物理按键状态、不设时间窗口——手快连续确认每个提示各按一次即可生效。
/// Drop 时自动卸载。
pub struct ConfirmKeyHook {
    tid: u32,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl ConfirmKeyHook {
    /// 安装钩子。`sender` 会收到 ConfirmChoice：Enter 确认、Shift+Enter 确认全部、Esc 取消。
    /// 返回前会等待钩子真正装好（钩子线程已进入消息循环、可接收按键），
    /// 或确认安装失败，调用方无需再 sleep 兜底。
    /// release 构建里 `Err(e)` 的 `e` 只被 debug_log! 使用（宏为空），故按发布忽略未用告警。
    #[cfg_attr(not(debug_assertions), allow(unused_variables))]
    pub fn install(sender: mpsc::Sender<ConfirmChoice>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = Arc::clone(&stop);
        let (tid_tx, tid_rx) = mpsc::channel::<u32>();
        let (ready_tx, ready_rx) = mpsc::channel::<()>();
        let thread = thread::spawn(move || {
            let tid = unsafe { GetCurrentThreadId() };
            let _ = tid_tx.send(tid);
            run_hook_thread(stop2, sender, ready_tx);
        });
        let tid = tid_rx.recv_timeout(Duration::from_secs(5)).unwrap_or(0);
        // 等待钩子已装好、线程即将进入消息循环（可接收按键）或安装失败；超时则记录并继续。
        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(()) => crate::debug_log!("hook install: ready"),
            Err(e) => crate::debug_log!("hook install: ready timeout/err {e:?}"),
        }
        Self {
            tid,
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for ConfirmKeyHook {
    fn drop(&mut self) {
        // 投递 WM_QUIT 唤醒消息泵，让钩子线程退出并卸载钩子。
        if !self.stop.swap(true, Ordering::Relaxed) && self.tid != 0 {
            unsafe { PostThreadMessageW(self.tid, WM_QUIT, 0, 0) };
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        *HOOK_TX.lock().unwrap() = None;
    }
}

fn run_hook_thread(
    stop: Arc<AtomicBool>,
    sender: mpsc::Sender<ConfirmChoice>,
    ready: mpsc::Sender<()>,
) {
    *HOOK_TX.lock().unwrap() = Some(sender);
    let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), 0, 0) };
    if hook == 0 {
        // 安装失败（极少见）：清空发送端，前端按键路径仍可兜底。
        crate::debug_log!("hook install: FAILED");
        *HOOK_TX.lock().unwrap() = None;
        let _ = ready.send(());
        return;
    }
    crate::debug_log!("hook install: OK");
    // 钩子已装好、线程即将进入消息循环：立即通知就绪。
    // 注意：不能在“等到第一条消息后”再发——正常提示期间本线程的消息队列是空的，
    // GetMessageW 会一直阻塞，那样 ready 永远等不到，会导致弹窗延迟数秒。
    // 低层钩子的回调只要求安装线程处于消息检索状态（阻塞在 GetMessageW 中）即可。
    let _ = ready.send(());
    // 低层钩子的回调运行在安装线程上下文，必须由该线程泵消息。
    let mut msg = unsafe { std::mem::zeroed::<MSG>() };
    while unsafe { GetMessageW(&mut msg, 0, 0, 0) } > 0 {
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if stop.load(Ordering::Relaxed) {
            break;
        }
    }
    unsafe { UnhookWindowsHookEx(hook) };
    crate::debug_log!("hook uninstalled");
    *HOOK_TX.lock().unwrap() = None;
}

/// 钩子回调：Enter 确认、Shift+Enter 确认全部、Esc 取消；都被吞掉，不继续分发给任何窗口。
/// 每个提示接受第一次按下（take 发送端，之后不再应答）。
unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let msg = wparam as u32;
        let kbd = &*(lparam as *const KBDLLHOOKSTRUCT);
        let is_hooked = kbd.vkCode == VK_RETURN as u32 || kbd.vkCode == VK_ESCAPE as u32;
        if is_hooked {
            if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                let choice = if kbd.vkCode == VK_RETURN as u32 {
                    // Shift 仍按住 → 确认剩余所有步骤。
                    let shift_down =
                        (GetAsyncKeyState(VK_SHIFT as i32) as i32 & 0x8000) != 0;
                    if shift_down {
                        ConfirmChoice::ConfirmAll
                    } else {
                        ConfirmChoice::Confirm
                    }
                } else {
                    ConfirmChoice::Cancel
                };
                let mut guard = HOOK_TX.lock().unwrap();
                crate::debug_log!(
                    "keydown vk={} choice={:?} sender_present={}",
                    kbd.vkCode,
                    choice,
                    guard.is_some()
                );
                // take 置 None：本提示只应答一次，之后的重复按下一律吞掉。
                if let Some(tx) = guard.take() {
                    let _ = tx.send(choice);
                }
                drop(guard);
            }
            // 吞掉按键（含 keyup），避免它落入目标应用。
            return 1;
        }
    }
    CallNextHookEx(0, code, wparam, lparam)
}
