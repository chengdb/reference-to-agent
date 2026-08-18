//! 动作序列引擎：按顺序执行一组 Step 原语。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    Wait {
        ms: u64,
        /// 执行前人工确认（需配方启用「人工确认」）。
        #[serde(default)]
        confirm: bool,
    },
    /// 注入组合键，如 "Ctrl+Shift+C"。
    Hotkey {
        keys: String,
        #[serde(default)]
        confirm: bool,
    },
    /// 激活目标应用窗口；按 title 模糊匹配，未找到且配置了 exe 则启动它。
    ActivateApp {
        title: String,
        #[serde(default)]
        exe: Option<String>,
        #[serde(default)]
        confirm: bool,
    },
    /// 聚焦已打开的应用窗口（优先按进程 exe 匹配，其次按标题；不启动新进程）。
    FocusApp {
        title: String,
        #[serde(default)]
        exe: Option<String>,
        #[serde(default)]
        confirm: bool,
    },
    /// 设置剪贴板文本。
    SetClipboard {
        text: String,
        #[serde(default)]
        confirm: bool,
    },
    /// 直接注入文本（受当前输入法影响，一般用于少量字符）。
    TypeText {
        text: String,
        #[serde(default)]
        confirm: bool,
    },
    /// 输入文本：写入剪贴板并粘贴（对中文/长文本可靠）。
    PasteText {
        text: String,
        #[serde(default)]
        confirm: bool,
    },
    /// 运行外部命令。
    RunCommand {
        cmd: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        confirm: bool,
    },
    /// 在目标窗口内按「x/y 轴各自独立定位」模拟鼠标左键点击，用于聚焦输入框等控件。
    /// x 轴基准可选 left/right，y 轴基准可选 top/bottom，偏移量单位可选 percent/px。
    /// 例如 x={base:left,value:50%,unit:percent}、y={base:bottom,value:8%,unit:percent}
    /// 表示水平居中、垂直沉底偏上，适合点击聊天输入框。
    Click(Click),

    /// 条件分叉：按 op 比较 value 与 expected，命中则执行 then，否则依次尝试 elseIf，最终 else。
    If(IfBranch),

    /// 把剪贴板恢复为配方执行前的原始内容（配合复制/粘贴类步骤使用）。
    RollbackClipboard {
        #[serde(default)]
        confirm: bool,
    },
}

/// 比较操作符。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompareOp {
    /// 等于（区分大小写）。
    Eq,
    /// 不等于（区分大小写）。
    Ne,
    /// 大于（数值比较，无法解析为数值时降级为字符串比较）。
    Gt,
    /// 大于等于。
    Ge,
    /// 小于。
    Lt,
    /// 小于等于。
    Le,
    /// 前缀匹配（区分大小写）。
    StartsWith,
    /// 后缀匹配（区分大小写）。
    EndsWith,
    /// 包含（区分大小写）。
    Contains,
    /// 正则匹配（expected 为模式；编译失败视为不命中）。
    Matches,
}

/// 单个条件分支（if / else-if 各一条）：value 经变量展开后与 expected 比较。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareBranch {
    pub op: CompareOp,
    /// 左侧值：支持 `${name}` 与 `${name:默认值}` 变量引用，未定义且无默认值时保留原样。
    pub value: String,
    /// 右侧期望值：同样支持 `${name}` 与 `${name:默认值}` 变量引用。
    pub expected: String,
    /// 命中时执行的步骤序列。
    #[serde(default)]
    pub then: Vec<Step>,
}

/// if 分叉步骤：value op expected ? then : (else_if... | else)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IfBranch {
    pub op: CompareOp,
    pub value: String,
    pub expected: String,
    #[serde(default)]
    pub then: Vec<Step>,
    #[serde(default)]
    pub else_if: Vec<CompareBranch>,
    #[serde(default, rename = "else")]
    pub else_branch: Vec<Step>,
    /// 执行前人工确认（需配方启用「人工确认」）。
    #[serde(default)]
    pub confirm: bool,
}

/// click 步骤数据。反序列化时兼容旧格式（corner/rx/ry），自动迁移为 x/y 轴模型。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Click {
    pub title: String,
    pub x: AxisPos,
    pub y: AxisPos,
    /// 执行前人工确认（需配方启用「人工确认」）。
    pub confirm: bool,
}

/// 反序列化辅助：同时接受新格式（x/y）与旧格式（corner/rx/ry）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawClick {
    title: Option<String>,
    x: Option<AxisPos>,
    y: Option<AxisPos>,
    confirm: Option<bool>,
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
        let confirm = raw.confirm.unwrap_or(false);

        // 新格式优先。
        if let (Some(x), Some(y)) = (raw.x, raw.y) {
            return Ok(Click { title, x, y, confirm });
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
        Ok(Click { title, x, y, confirm })
    }
}

/// 把文本里的 `${name}` 与 `${name:默认值}` 展开为变量值。
/// 变量名缺失时：有默认值用默认值，否则保留原样。变量值本身不再递归展开。
pub fn expand_vars(text: &str, vars: &HashMap<String, String>) -> String {
    if !text.contains('$') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(idx) = rest.find("${") {
        out.push_str(&rest[..idx]);
        let after = &rest[idx + 2..];
        match after.find('}') {
            None => {
                // 未闭合的 `${`，原样保留并结束。
                out.push_str(&rest[idx..]);
                return out;
            }
            Some(end) => {
                let inner = &after[..end];
                let (name, default) = match inner.find(':') {
                    Some(ci) => (&inner[..ci], Some(&inner[ci + 1..])),
                    None => (inner, None),
                };
                match vars.get(name) {
                    Some(v) => out.push_str(v),
                    None => {
                        if let Some(d) = default {
                            out.push_str(d);
                        } else {
                            // 未定义的变量，原样保留占位符。
                            out.push_str(&rest[idx..idx + 2 + end + 1]);
                        }
                    }
                }
                rest = &after[end + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// 人工确认的结果选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmChoice {
    /// 确认执行当前步骤。
    Confirm,
    /// Shift+Enter：确认当前步骤及后续所有步骤（本次配方运行不再询问）。
    ConfirmAll,
    /// 取消执行整个配方。
    Cancel,
}

/// 用户取消执行时返回的错误信息（调用方据此区分“用户取消”与“执行失败”）。
pub const CANCELED: &str = "已取消执行";

/// 带人工确认的执行入口：
/// - recipe_confirm：配方级开关（配置里配方的「人工确认」）；
/// - 当 recipe_confirm 为 true 且该步骤自身 confirm 为 true 时，在步骤执行前调用 ask；
/// - ask 返回 Confirm 继续执行、ConfirmAll 继续执行且后续不再询问、Cancel 中止整个配方、
///   Err 作为失败向上传播。
pub fn run_steps_confirmed(
    steps: &[Step],
    recipe_confirm: bool,
    ask: &mut dyn FnMut(usize, &Step) -> Result<ConfirmChoice, String>,
) -> Result<(), String> {
    // 配方级变量表：聚焦步骤成功后写入真实标题，供后续文本字段以 ${title} 引用。
    let mut vars: HashMap<String, String> = HashMap::new();

    // 配方含回滚步骤时，开始执行前备份一次剪贴板，供后续 RollbackClipboard 恢复。
    // 快照为 Option<String>：None 表示原剪贴板为空或非文本，回滚时保持不动。
    // 回滚步骤可能嵌套在 if 分支内，故递归检测。
    let needs_rollback = steps.iter().any(contains_rollback);
    let clipboard_snapshot = if needs_rollback {
        win::read_clipboard().map_err(|e| format!("备份剪贴板失败: {e}"))?
    } else {
        None
    };
    let mut seq = 0usize;
    let mut confirm_all = false;
    run_steps_inner(
        steps,
        recipe_confirm,
        &mut confirm_all,
        &mut vars,
        clipboard_snapshot.as_deref(),
        &mut seq,
        ask,
    )
}

/// 递归判断步骤树中是否含回滚步骤。
fn contains_rollback(s: &Step) -> bool {
    match s {
        Step::RollbackClipboard { .. } => true,
        Step::If(b) => {
            b.then.iter().any(contains_rollback)
                || b.else_if
                    .iter()
                    .any(|ei| ei.then.iter().any(contains_rollback))
                || b.else_branch.iter().any(contains_rollback)
        }
        _ => false,
    }
}

/// 读取步骤自身的「人工确认」开关。
/// 等待步骤不注入任何操作，恒为 false（编辑器已隐藏其复选框，这里兜底忽略旧配置）。
fn step_confirm(step: &Step) -> bool {
    match step {
        Step::Wait { .. } => false,
        Step::Hotkey { confirm, .. } => *confirm,
        Step::ActivateApp { confirm, .. } => *confirm,
        Step::FocusApp { confirm, .. } => *confirm,
        Step::SetClipboard { confirm, .. } => *confirm,
        Step::TypeText { confirm, .. } => *confirm,
        Step::PasteText { confirm, .. } => *confirm,
        Step::RunCommand { confirm, .. } => *confirm,
        Step::Click(c) => c.confirm,
        Step::If(b) => b.confirm,
        Step::RollbackClipboard { confirm } => *confirm,
    }
}

/// 生成步骤的人类可读描述，用于人工确认弹窗。
pub fn describe_step(step: &Step) -> String {
    match step {
        Step::Wait { ms, .. } => format!("等待 {ms}ms"),
        Step::Hotkey { keys, .. } => format!("发送快捷键 {keys}"),
        Step::ActivateApp { title, .. } => format!("激活窗口「{title}」"),
        Step::FocusApp { title, .. } => format!("聚焦窗口「{title}」"),
        Step::SetClipboard { text, .. } => format!("设置剪贴板：{text}"),
        Step::TypeText { text, .. } => format!("输入文本：{text}"),
        Step::PasteText { text, .. } => format!("粘贴文本：{text}"),
        Step::RunCommand { cmd, .. } => format!("运行命令 {cmd}"),
        Step::Click(click) => format!("点击窗口「{}」", click.title),
        Step::If(_) => "条件判断（if）".to_string(),
        Step::RollbackClipboard { .. } => "恢复剪贴板".to_string(),
    }
}

/// 递归执行步骤序列。vars 在整棵树内共享（分支内写入的变量向外冒泡）。
/// seq 为全流程递增的步骤序号（含嵌套分支），用于确认弹窗的「第 N 步」。
/// confirm_all 为 true 后，后续所有已勾选确认的步骤直接执行、不再询问。
fn run_steps_inner(
    steps: &[Step],
    recipe_confirm: bool,
    confirm_all: &mut bool,
    vars: &mut HashMap<String, String>,
    clipboard_snapshot: Option<&str>,
    seq: &mut usize,
    ask: &mut dyn FnMut(usize, &Step) -> Result<ConfirmChoice, String>,
) -> Result<(), String> {
    for step in steps {
        *seq += 1;
        // 调试：记录每一步的执行时机（用于排查“Shift+Enter 后步骤乱序/回滚提前”）。
        crate::debug_log!("run: seq={} type={}", *seq, describe_step(step));
        // 配方级人工确认开关 + 步骤级 confirm 同时满足、且用户未选“全部确认”时才询问。
        if recipe_confirm && !*confirm_all && step_confirm(step) {
            match ask(*seq, step)? {
                ConfirmChoice::Confirm => {}
                ConfirmChoice::ConfirmAll => *confirm_all = true,
                ConfirmChoice::Cancel => return Err(CANCELED.to_string()),
            }
        }
        match step {
            Step::Wait { ms, .. } => thread::sleep(Duration::from_millis(*ms)),
            Step::Hotkey { keys, .. } => {
                let keys = expand_vars(keys, vars);
                win::press_hotkey(&keys).map_err(|e| format!("发送快捷键 {keys} 失败: {e}"))?
            }
            Step::ActivateApp { title, exe, .. } => {
                let title = expand_vars(title, vars);
                match win::activate_app(&title, exe.as_deref()) {
                    Ok(info) => {
                        if let Some(t) = info {
                            vars.insert("title".to_string(), t);
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
            Step::FocusApp { title, exe, .. } => {
                let title = expand_vars(title, vars);
                match win::focus_app(&title, exe.as_deref()) {
                    Ok(info) => {
                        if let Some(t) = info {
                            vars.insert("title".to_string(), t);
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
            Step::SetClipboard { text, .. } => {
                let text = expand_vars(text, vars);
                win::write_clipboard(&text).map_err(|e| format!("设置剪贴板失败: {e}"))?
            }
            Step::TypeText { text, .. } => {
                let text = expand_vars(text, vars);
                win::type_text(&text).map_err(|e| format!("输入文本失败: {e}"))?
            }
            Step::PasteText { text, .. } => {
                let text = expand_vars(text, vars);
                win::write_clipboard(&text).map_err(|e| format!("设置剪贴板失败: {e}"))?;
                win::press_hotkey("Ctrl+V").map_err(|e| format!("粘贴失败: {e}"))?;
            }
            Step::RunCommand { cmd, args, .. } => {
                let cmd = expand_vars(cmd, vars);
                let args: Vec<String> = args.iter().map(|a| expand_vars(a, vars)).collect();
                // 严格顺序：等待命令进程完全退出后再执行下一步，避免后续步骤抢跑。
                let status = std::process::Command::new(&cmd)
                    .args(&args)
                    .status()
                    .map_err(|e| format!("运行命令 {cmd} 失败: {e}"))?;
                if !status.success() {
                    return Err(format!(
                        "命令 {cmd} 执行失败，退出码 {}",
                        status.code().unwrap_or(-1)
                    ));
                }
            }
            Step::Click(click) => {
                let title = expand_vars(&click.title, vars);
                win::click_in_window(&title, click.x, click.y)
                    .map_err(|e| format!("点击窗口 {title} 失败: {e}"))?;
            }
            Step::If(b) => {
                run_if(b, recipe_confirm, confirm_all, vars, clipboard_snapshot, seq, ask)?;
            }
            Step::RollbackClipboard { .. } => {
                // 快照为 None（原剪贴板为空或非文本）时保持不动，避免用空字符串覆盖。
                if let Some(text) = clipboard_snapshot {
                    win::write_clipboard(text).map_err(|e| format!("回滚剪贴板失败: {e}"))?;
                }
            }
        }
    }
    Ok(())
}

/// 执行一次 if 分叉：命中 then / else_if / else 中的首个分支。
fn run_if(
    b: &IfBranch,
    recipe_confirm: bool,
    confirm_all: &mut bool,
    vars: &mut HashMap<String, String>,
    clipboard_snapshot: Option<&str>,
    seq: &mut usize,
    ask: &mut dyn FnMut(usize, &Step) -> Result<ConfirmChoice, String>,
) -> Result<(), String> {
    let lhs = expand_vars(&b.value, vars);
    let rhs = expand_vars(&b.expected, vars);
    if eval_compare(b.op, &lhs, &rhs) {
        return run_steps_inner(
            &b.then,
            recipe_confirm,
            confirm_all,
            vars,
            clipboard_snapshot,
            seq,
            ask,
        );
    }
    for ei in &b.else_if {
        let lhs = expand_vars(&ei.value, vars);
        let rhs = expand_vars(&ei.expected, vars);
        if eval_compare(ei.op, &lhs, &rhs) {
            return run_steps_inner(
                &ei.then,
                recipe_confirm,
                confirm_all,
                vars,
                clipboard_snapshot,
                seq,
                ask,
            );
        }
    }
    run_steps_inner(
        &b.else_branch,
        recipe_confirm,
        confirm_all,
        vars,
        clipboard_snapshot,
        seq,
        ask,
    )
}

/// 计算比较结果。
/// 数值比较（Gt/Ge/Lt/Le）两侧都能解析为 f64 时按数值比较，否则降级为字符串字典序。
/// 字符串比较（Eq/Ne/StartsWith/EndsWith/Contains）区分大小写。
/// Matches 把 expected 当作正则；编译失败或未命中均视为不命中（返回 false）。
fn eval_compare(op: CompareOp, lhs: &str, rhs: &str) -> bool {
    match op {
        CompareOp::Eq => lhs == rhs,
        CompareOp::Ne => lhs != rhs,
        CompareOp::Gt => compare_ord(lhs, rhs) == std::cmp::Ordering::Greater,
        CompareOp::Ge => matches!(
            compare_ord(lhs, rhs),
            std::cmp::Ordering::Greater | std::cmp::Ordering::Equal
        ),
        CompareOp::Lt => compare_ord(lhs, rhs) == std::cmp::Ordering::Less,
        CompareOp::Le => matches!(
            compare_ord(lhs, rhs),
            std::cmp::Ordering::Less | std::cmp::Ordering::Equal
        ),
        CompareOp::StartsWith => lhs.starts_with(rhs),
        CompareOp::EndsWith => lhs.ends_with(rhs),
        CompareOp::Contains => lhs.contains(rhs),
        CompareOp::Matches => {
            // 编译失败或未命中都返回 false（视为不命中，不中断执行）。
            regex::Regex::new(rhs).map(|re| re.is_match(lhs)).unwrap_or(false)
        }
    }
}

/// 数值优先比较：两侧都能解析为 f64 时按数值比较，否则按字符串字典序比较。
fn compare_ord(lhs: &str, rhs: &str) -> std::cmp::Ordering {
    match (lhs.trim().parse::<f64>(), rhs.trim().parse::<f64>()) {
        (Ok(a), Ok(b)) => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),
        _ => lhs.cmp(rhs),
    }
}
