//! Windows UI 自动化原语：窗口激活、鼠标位置、按键注入。
//! 窗口句柄统一用 `isize` 表示，避免 windows-sys / windows 两套 HWND 类型耦合。
//!
//! 按职责拆分为子模块：
//! - `window`：窗口查找、标题/PID 读取、前台切换、显示器工作区；
//! - `input`：键盘/鼠标注入、坐标换算、修饰键等待；
//! - `clipboard`：剪贴板读写；
//! - `apps`：已安装应用枚举（开始菜单快捷方式 + 商店应用）与应用启动；
//! - `hook`：人工确认用的低层键盘钩子与调试日志。

pub mod apps;
pub mod clipboard;
pub mod hook;
pub mod input;
pub mod window;

pub use apps::*;
pub use clipboard::*;
pub use hook::ConfirmKeyHook;
pub use input::*;
pub use window::*;

// debug_log! 宏（#[macro_export] 于 crate 根部）按 `$crate::win::debug_log` 路径引用。
pub(crate) use hook::debug_log;
