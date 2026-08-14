//! 动作序列引擎：按顺序执行一组 Step 原语。

use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;

use crate::win;

/// 坐标轴基准边（x 轴用 Left/Right，y 轴用 Top/Bottom）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Base {
    Left,
    Right,
    Top,
    Bottom,
}

/// 偏移单位：百分比（相对窗口宽/高）或固定像素。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Unit {
    Percent,
    Px,
}

impl Default for Unit {
    fn default() -> Self {
        Unit::Percent
    }
}

/// 单轴定位：从 `base` 基准边向内偏移 `value`（单位由 `unit` 决定）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AxisPos {
    pub base: Base,
    pub value: f64,
    #[serde(default)]
    pub unit: Unit,
}

impl AxisPos {
    pub fn percent(base: Base, value: f64) -> Self {
        Self { base, value, unit: Unit::Percent }
    }
}

impl Default for AxisPos {
    fn default() -> Self {
        Self { base: Base::Left, value: 0.0, unit: Unit::Percent }
    }
}

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
    /// 在目标窗口内按「x/y 轴各自独立定位」模拟鼠标左键点击，用于聚焦输入框等控件。
    /// x 轴基准可选 left/right，y 轴基准可选 top/bottom，偏移量单位可选 percent/px。
    /// 例如 x={base:left,value:50%,unit:percent}、y={base:bottom,value:8%,unit:percent}
    /// 表示水平居中、垂直沉底偏上，适合点击聊天输入框。
    Click(Click),

    /// 把剪贴板恢复为配方执行前的原始内容（配合复制/粘贴类步骤使用）。
    RollbackClipboard,
}

/// click 步骤数据。反序列化时兼容旧格式（corner/rx/ry），自动迁移为 x/y 轴模型。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Click {
    pub title: String,
    pub x: AxisPos,
    pub y: AxisPos,
}

/// 反序列化辅助：同时接受新格式（x/y）与旧格式（corner/rx/ry）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawClick {
    title: Option<String>,
    x: Option<AxisPos>,
    y: Option<AxisPos>,
    // 旧字段
    corner: Option<Corner>,
    rx: Option<f64>,
    ry: Option<f64>,
}

/// 旧版四角锚点枚举（仅用于反序列化迁移）。
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl<'de> Deserialize<'de> for Click {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawClick::deserialize(deserializer)?;
        let title = raw.title.unwrap_or_default();

        // 新格式优先。
        if let (Some(x), Some(y)) = (raw.x, raw.y) {
            return Ok(Click { title, x, y });
        }

        // 旧格式 corner/rx/ry → 拆解成 x/y 轴。
        let rx = raw.rx.unwrap_or(0.0).clamp(0.0, 1.0);
        let ry = raw.ry.unwrap_or(0.0).clamp(0.0, 1.0);
        let corner = raw.corner.unwrap_or(Corner::TopLeft);
        let (x, y) = match corner {
            Corner::TopLeft => (
                AxisPos::percent(Base::Left, rx),
                AxisPos::percent(Base::Top, ry),
            ),
            Corner::TopRight => (
                AxisPos::percent(Base::Right, rx),
                AxisPos::percent(Base::Top, ry),
            ),
            Corner::BottomLeft => (
                AxisPos::percent(Base::Left, rx),
                AxisPos::percent(Base::Bottom, ry),
            ),
            Corner::BottomRight => (
                AxisPos::percent(Base::Right, rx),
                AxisPos::percent(Base::Bottom, ry),
            ),
        };
        Ok(Click { title, x, y })
    }
}

pub fn run_steps(steps: &[Step]) -> Result<(), String> {
    // 配方含回滚步骤时，开始执行前备份一次剪贴板，供后续 RollbackClipboard 恢复。
    // 快照为 Option<String>：None 表示原剪贴板为空或非文本，回滚时保持不动。
    let needs_rollback = steps.iter().any(|s| matches!(s, Step::RollbackClipboard));
    let clipboard_snapshot = if needs_rollback {
        win::read_clipboard().map_err(|e| format!("备份剪贴板失败: {e}"))?
    } else {
        None
    };
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
            Step::Click(click) => {
                win::click_in_window(&click.title, click.x, click.y)
                    .map_err(|e| format!("点击窗口 {} 失败: {e}", click.title))?;
            }
            Step::RollbackClipboard => {
                // 快照为 None（原剪贴板为空或非文本）时保持不动，避免用空字符串覆盖。
                if let Some(text) = clipboard_snapshot.as_deref() {
                    win::write_clipboard(text).map_err(|e| format!("回滚剪贴板失败: {e}"))?;
                }
            }
        }
    }
    Ok(())
}
