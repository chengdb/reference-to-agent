//! 动作序列引擎：按顺序执行一组 Step 原语。

use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;

use crate::win;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Step {
    /// 等待指定毫秒。
    Wait { ms: u64 },
    /// 注入组合键，如 "Ctrl+Shift+C"。
    Hotkey { keys: String },
    /// 激活目标应用窗口；按 title 模糊匹配，未找到且配置了 exe 则启动它。
    ActivateApp {
        title: String,
        #[serde(default)]
        exe: Option<String>,
    },
    /// 聚焦已打开的应用窗口（优先按进程 exe 匹配，其次按标题；不启动新进程）。
    FocusApp {
        title: String,
        #[serde(default)]
        exe: Option<String>,
    },
    /// 设置剪贴板文本。
    SetClipboard { text: String },
    /// 直接注入文本（受当前输入法影响，一般用于少量字符）。
    TypeText { text: String },
    /// 输入文本：写入剪贴板并粘贴（对中文/长文本可靠）。
    PasteText { text: String },
    /// 运行外部命令。
    RunCommand {
        cmd: String,
        #[serde(default)]
        args: Vec<String>,
    },
}

pub fn run_steps(steps: &[Step]) -> Result<(), String> {
    for step in steps {
        match step {
            Step::Wait { ms } => thread::sleep(Duration::from_millis(*ms)),
            Step::Hotkey { keys } => {
                win::press_hotkey(keys).map_err(|e| format!("发送快捷键 {keys} 失败: {e}"))?
            }
            Step::ActivateApp { title, exe } => {
                win::activate_app(title, exe.as_deref()).map_err(|e| e.to_string())?;
            }
            Step::FocusApp { title, exe } => {
                win::focus_app(title, exe.as_deref()).map_err(|e| e.to_string())?;
            }
            Step::SetClipboard { text } => {
                win::write_clipboard(text).map_err(|e| format!("设置剪贴板失败: {e}"))?
            }
            Step::TypeText { text } => {
                win::type_text(text).map_err(|e| format!("输入文本失败: {e}"))?
            }
            Step::PasteText { text } => {
                win::write_clipboard(text).map_err(|e| format!("设置剪贴板失败: {e}"))?;
                win::press_hotkey("Ctrl+V").map_err(|e| format!("粘贴失败: {e}"))?;
            }
            Step::RunCommand { cmd, args } => {
                let out = std::process::Command::new(cmd)
                    .args(args)
                    .spawn()
                    .map_err(|e| format!("运行命令 {cmd} 失败: {e}"))?;
                let _ = out;
            }
        }
    }
    Ok(())
}
