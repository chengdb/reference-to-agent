// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 显式声明 Per-Monitor V2 DPI 感知：保证 GetWindowRect / GetSystemMetrics /
    // SendInput / SetCursorPos 全部处于物理像素坐标系，多显示器（含不同 DPI、主屏左侧负坐标）下定位一致。
    #[cfg(target_os = "windows")]
    unsafe {
        use windows_sys::Win32::UI::HiDpi::{
            SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        };
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    reference_to_agent_lib::run()
}
