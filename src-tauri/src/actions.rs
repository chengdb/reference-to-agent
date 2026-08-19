//! 动作序列引擎：按顺序执行一组 Step 原语。
//!
//! 新增一个步骤类型需要改动本文件的三处：`StepKind` 枚举、`describe_step`、
//! `run_steps_inner` 的 match（编译器会强制穷尽）；前端同步点见 src/types/index.ts
//! 头部注释。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

use crate::win;

/// 条件分叉中产生的变量名：聚焦/激活步骤成功后写入的真实窗口标题。
/// 前端 VAR_PRODUCERS（src/components/config/steps/IfEditor.vue）镜像此约定。
pub const VAR_TITLE: &str = "title";

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

/// 单个执行步骤：公共标记（confirm）+ 具体动作（kind）。
/// confirm 经 serde flatten 平铺在步骤对象顶层，JSON 形态与旧版
/// （各变体自带 confirm 字段）完全一致，存量配置无需迁移。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// 执行前人工确认（需配方启用「人工确认」）。
    #[serde(default)]
    pub confirm: bool,
    #[serde(flatten)]
    pub kind: StepKind,
}

/// 步骤动作本体（不含公共标记）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StepKind {
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

    /// 条件分叉：按 op 比较 value 与 expected，命中则执行 then，否则依次尝试 elseIf，最终 else。
    If(IfBranch),

    /// 把剪贴板恢复为配方执行前的原始内容（配合复制/粘贴类步骤使用）。
    RollbackClipboard,
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

/// 配方执行的失败原因：区分「用户取消」与「执行失败」，
/// 调用方（run_recipe）据此决定是轻提示取消还是重弹菜单报错。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunError {
    /// 用户在人工确认中取消（Esc / 确认窗口失焦）。
    Canceled,
    /// 步骤执行失败。
    Failed(String),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::Canceled => write!(f, "已取消执行"),
            RunError::Failed(msg) => write!(f, "{msg}"),
        }
    }
}

impl From<String> for RunError {
    fn from(msg: String) -> Self {
        RunError::Failed(msg)
    }
}

/// 一次配方运行的全部执行期状态，递归执行（if 分支）时整体传递。
/// 新增执行期状态（新变量、缓存等）只需扩展本结构，不必改动各递归函数签名。
struct RunContext<'a> {
    /// 配方级人工确认开关（配置里配方的「人工确认」）。
    recipe_confirm: bool,
    /// Shift+Enter 后置真：后续所有已勾选确认的步骤直接执行、不再询问。
    confirm_all: bool,
    /// 配方级变量表：聚焦步骤成功后写入真实标题，供后续文本字段以 ${title} 引用。
    /// 整棵步骤树共享（分支内写入的变量向外冒泡）。
    vars: HashMap<String, String>,
    /// 配方含回滚步骤时，开始执行前备份的剪贴板（None 表示原剪贴板为空或非文本）。
    clipboard_snapshot: Option<String>,
    /// 全流程递增的步骤序号（含嵌套分支），用于确认弹窗的「第 N 步」。
    seq: usize,
    /// 人工确认回调：返回 Confirm 继续、ConfirmAll 后续不再询问、Cancel 中止整个配方。
    ask: &'a mut dyn FnMut(usize, &Step) -> Result<ConfirmChoice, String>,
    /// matches 操作符的正则缓存：同一模式在一次运行中只编译一次（编译失败缓存为 None）。
    regex_cache: HashMap<String, Option<regex::Regex>>,
}

impl RunContext<'_> {
    /// 计算比较结果。
    /// 数值比较（Gt/Ge/Lt/Le）两侧都能解析为 f64 时按数值比较，否则降级为字符串字典序。
    /// 字符串比较（Eq/Ne/StartsWith/EndsWith/Contains）区分大小写。
    /// Matches 把 expected 当作正则；编译失败或未命中均视为不命中（返回 false）。
    fn eval_compare(&mut self, op: CompareOp, lhs: &str, rhs: &str) -> bool {
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
            CompareOp::Matches => self
                .regex_cache
                .entry(rhs.to_string())
                .or_insert_with(|| regex::Regex::new(rhs).ok())
                .as_ref()
                .map(|re| re.is_match(lhs))
                .unwrap_or(false),
        }
    }
}

/// 带人工确认的执行入口：
/// - recipe_confirm：配方级开关（配置里配方的「人工确认」）；
/// - 当 recipe_confirm 为 true 且该步骤自身 confirm 为 true 时，在步骤执行前调用 ask；
/// - ask 返回 Confirm 继续执行、ConfirmAll 继续执行且后续不再询问、Cancel 中止整个配方、
///   Err 作为失败向上传播。
pub fn run_steps_confirmed(
    steps: &[Step],
    recipe_confirm: bool,
    ask: &mut dyn FnMut(usize, &Step) -> Result<ConfirmChoice, String>,
) -> Result<(), RunError> {
    // 配方含回滚步骤时，开始执行前备份一次剪贴板，供后续 RollbackClipboard 恢复。
    // 快照为 Option<String>：None 表示原剪贴板为空或非文本，回滚时保持不动。
    // 回滚步骤可能嵌套在 if 分支内，故递归检测。
    let needs_rollback = steps.iter().any(contains_rollback);
    let clipboard_snapshot = if needs_rollback {
        win::read_clipboard().map_err(|e| format!("备份剪贴板失败: {e}"))?
    } else {
        None
    };
    let mut ctx = RunContext {
        recipe_confirm,
        confirm_all: false,
        vars: HashMap::new(),
        clipboard_snapshot,
        seq: 0,
        ask,
        regex_cache: HashMap::new(),
    };
    run_steps_inner(steps, &mut ctx)
}

/// 递归判断步骤树中是否含回滚步骤。
fn contains_rollback(s: &Step) -> bool {
    match &s.kind {
        StepKind::RollbackClipboard => true,
        StepKind::If(b) => {
            b.then.iter().any(contains_rollback)
                || b.else_if
                    .iter()
                    .any(|ei| ei.then.iter().any(contains_rollback))
                || b.else_branch.iter().any(contains_rollback)
        }
        _ => false,
    }
}

/// 步骤是否需要人工确认。
/// 等待步骤不注入任何操作，恒为 false（编辑器已隐藏其复选框，这里兜底忽略旧配置）。
fn step_confirm(step: &Step) -> bool {
    step.confirm && !matches!(step.kind, StepKind::Wait { .. })
}

/// 生成步骤的人类可读描述，用于人工确认弹窗。
pub fn describe_step(step: &Step) -> String {
    match &step.kind {
        StepKind::Wait { ms } => format!("等待 {ms}ms"),
        StepKind::Hotkey { keys } => format!("发送快捷键 {keys}"),
        StepKind::ActivateApp { title, .. } => format!("激活窗口「{title}」"),
        StepKind::FocusApp { title, .. } => format!("聚焦窗口「{title}」"),
        StepKind::SetClipboard { text } => format!("设置剪贴板：{text}"),
        StepKind::TypeText { text } => format!("输入文本：{text}"),
        StepKind::PasteText { text } => format!("粘贴文本：{text}"),
        StepKind::RunCommand { cmd, .. } => format!("运行命令 {cmd}"),
        StepKind::Click(click) => format!("点击窗口「{}」", click.title),
        StepKind::If(_) => "条件判断（if）".to_string(),
        StepKind::RollbackClipboard => "恢复剪贴板".to_string(),
    }
}

/// 递归执行步骤序列。全部执行期状态集中在 ctx 中传递。
fn run_steps_inner(steps: &[Step], ctx: &mut RunContext) -> Result<(), RunError> {
    for step in steps {
        ctx.seq += 1;
        // 调试：记录每一步的执行时机（用于排查“Shift+Enter 后步骤乱序/回滚提前”）。
        crate::debug_log!("run: seq={} type={}", ctx.seq, describe_step(step));
        // 配方级人工确认开关 + 步骤级 confirm 同时满足、且用户未选“全部确认”时才询问。
        if ctx.recipe_confirm && !ctx.confirm_all && step_confirm(step) {
            match (ctx.ask)(ctx.seq, step).map_err(RunError::Failed)? {
                ConfirmChoice::Confirm => {}
                ConfirmChoice::ConfirmAll => ctx.confirm_all = true,
                ConfirmChoice::Cancel => return Err(RunError::Canceled),
            }
        }
        match &step.kind {
            StepKind::Wait { ms } => thread::sleep(Duration::from_millis(*ms)),
            StepKind::Hotkey { keys } => {
                let keys = expand_vars(keys, &ctx.vars);
                win::press_hotkey(&keys).map_err(|e| format!("发送快捷键 {keys} 失败: {e}"))?
            }
            StepKind::ActivateApp { title, exe } => {
                let title = expand_vars(title, &ctx.vars);
                match win::activate_app(&title, exe.as_deref()) {
                    Ok(info) => {
                        if let Some(t) = info {
                            ctx.vars.insert(VAR_TITLE.to_string(), t);
                        }
                    }
                    Err(e) => return Err(RunError::Failed(e)),
                }
            }
            StepKind::FocusApp { title, exe } => {
                let title = expand_vars(title, &ctx.vars);
                match win::focus_app(&title, exe.as_deref()) {
                    Ok(info) => {
                        if let Some(t) = info {
                            ctx.vars.insert(VAR_TITLE.to_string(), t);
                        }
                    }
                    Err(e) => return Err(RunError::Failed(e)),
                }
            }
            StepKind::SetClipboard { text } => {
                let text = expand_vars(text, &ctx.vars);
                win::write_clipboard(&text).map_err(|e| format!("设置剪贴板失败: {e}"))?
            }
            StepKind::TypeText { text } => {
                let text = expand_vars(text, &ctx.vars);
                win::type_text(&text).map_err(|e| format!("输入文本失败: {e}"))?
            }
            StepKind::PasteText { text } => {
                let text = expand_vars(text, &ctx.vars);
                win::write_clipboard(&text).map_err(|e| format!("设置剪贴板失败: {e}"))?;
                win::press_hotkey("Ctrl+V").map_err(|e| format!("粘贴失败: {e}"))?;
            }
            StepKind::RunCommand { cmd, args } => {
                let cmd = expand_vars(cmd, &ctx.vars);
                let args: Vec<String> = args.iter().map(|a| expand_vars(a, &ctx.vars)).collect();
                // 严格顺序：等待命令进程完全退出后再执行下一步，避免后续步骤抢跑。
                let status = std::process::Command::new(&cmd)
                    .args(&args)
                    .status()
                    .map_err(|e| format!("运行命令 {cmd} 失败: {e}"))?;
                if !status.success() {
                    return Err(RunError::Failed(format!(
                        "命令 {cmd} 执行失败，退出码 {}",
                        status.code().unwrap_or(-1)
                    )));
                }
            }
            StepKind::Click(click) => {
                let title = expand_vars(&click.title, &ctx.vars);
                win::click_in_window(&title, click.x, click.y)
                    .map_err(|e| format!("点击窗口 {title} 失败: {e}"))?;
            }
            StepKind::If(b) => {
                run_if(b, ctx)?;
            }
            StepKind::RollbackClipboard => {
                // 快照为 None（原剪贴板为空或非文本）时保持不动，避免用空字符串覆盖。
                if let Some(text) = ctx.clipboard_snapshot.as_deref() {
                    win::write_clipboard(text).map_err(|e| format!("回滚剪贴板失败: {e}"))?;
                }
            }
        }
    }
    Ok(())
}

/// 执行一次 if 分叉：命中 then / else_if / else 中的首个分支。
fn run_if(b: &IfBranch, ctx: &mut RunContext) -> Result<(), RunError> {
    let lhs = expand_vars(&b.value, &ctx.vars);
    let rhs = expand_vars(&b.expected, &ctx.vars);
    if ctx.eval_compare(b.op, &lhs, &rhs) {
        return run_steps_inner(&b.then, ctx);
    }
    for ei in &b.else_if {
        let lhs = expand_vars(&ei.value, &ctx.vars);
        let rhs = expand_vars(&ei.expected, &ctx.vars);
        if ctx.eval_compare(ei.op, &lhs, &rhs) {
            return run_steps_inner(&ei.then, ctx);
        }
    }
    run_steps_inner(&b.else_branch, ctx)
}

/// 数值优先比较：两侧都能解析为 f64 时按数值比较，否则按字符串字典序比较。
fn compare_ord(lhs: &str, rhs: &str) -> std::cmp::Ordering {
    match (lhs.trim().parse::<f64>(), rhs.trim().parse::<f64>()) {
        (Ok(a), Ok(b)) => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),
        _ => lhs.cmp(rhs),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn test_ctx<'a>(
        ask: &'a mut dyn FnMut(usize, &Step) -> Result<ConfirmChoice, String>,
    ) -> RunContext<'a> {
        RunContext {
            recipe_confirm: false,
            confirm_all: false,
            vars: HashMap::new(),
            clipboard_snapshot: None,
            seq: 0,
            ask,
            regex_cache: HashMap::new(),
        }
    }

    /* ---------- expand_vars ---------- */

    #[test]
    fn expand_vars_basic() {
        let v = vars(&[("title", "Claude")]);
        assert_eq!(expand_vars("发送到 ${title}", &v), "发送到 Claude");
        assert_eq!(expand_vars("${title}-${title}", &v), "Claude-Claude");
    }

    #[test]
    fn expand_vars_default_and_missing() {
        let v = vars(&[]);
        // 未定义但有默认值 → 用默认值（默认值不做 trim，按字面取冒号后内容）。
        assert_eq!(expand_vars("${title:fallback}", &v), "fallback");
        assert_eq!(expand_vars("${title: fb}", &v), " fb");
        // 未定义且无默认值 → 原样保留占位符。
        assert_eq!(expand_vars("a ${title} b", &v), "a ${title} b");
        // 已定义时忽略默认值。
        let v2 = vars(&[("title", "X")]);
        assert_eq!(expand_vars("${title:Y}", &v2), "X");
    }

    #[test]
    fn expand_vars_unclosed_kept() {
        let v = vars(&[]);
        assert_eq!(expand_vars("abc ${title", &v), "abc ${title");
        // 变量值本身不再递归展开。
        let v2 = vars(&[("a", "${b}"), ("b", "B")]);
        assert_eq!(expand_vars("${a}", &v2), "${b}");
        // 不含 $ 时原样返回。
        assert_eq!(expand_vars("plain", &v), "plain");
    }

    /* ---------- compare_ord / eval_compare ---------- */

    #[test]
    fn compare_ord_numeric_vs_string() {
        // 两侧都是数值 → 数值比较（"10" > "9"）。
        assert_eq!(compare_ord("10", "9"), std::cmp::Ordering::Greater);
        // 任一侧非数值 → 字符串字典序（"10" < "9"）。
        assert_eq!(compare_ord("10x", "9"), std::cmp::Ordering::Less);
        assert_eq!(compare_ord(" 3 ", "3"), std::cmp::Ordering::Equal);
    }

    #[test]
    fn eval_compare_ops() {
        let mut ask = |_: usize, _: &Step| Ok(ConfirmChoice::Confirm);
        let mut ctx = test_ctx(&mut ask);
        assert!(ctx.eval_compare(CompareOp::Eq, "a", "a"));
        assert!(!ctx.eval_compare(CompareOp::Eq, "a", "A")); // 区分大小写
        assert!(ctx.eval_compare(CompareOp::Ne, "a", "b"));
        assert!(ctx.eval_compare(CompareOp::Gt, "10", "9")); // 数值比较
        assert!(ctx.eval_compare(CompareOp::Ge, "9", "9"));
        assert!(ctx.eval_compare(CompareOp::Lt, "9", "10"));
        assert!(ctx.eval_compare(CompareOp::Le, "9", "9"));
        assert!(ctx.eval_compare(CompareOp::StartsWith, "hello", "he"));
        assert!(ctx.eval_compare(CompareOp::EndsWith, "hello", "lo"));
        assert!(ctx.eval_compare(CompareOp::Contains, "hello", "ell"));
        assert!(ctx.eval_compare(CompareOp::Matches, "abc123", r"\d+"));
        // 正则编译失败视为不命中，不中断执行。
        assert!(!ctx.eval_compare(CompareOp::Matches, "abc", r"("));
    }

    /* ---------- Step JSON 形态（flatten confirm 与旧版兼容） ---------- */

    #[test]
    fn step_json_flat_confirm() {
        let s: Step =
            serde_json::from_str(r#"{"type":"hotkey","keys":"Ctrl+C","confirm":true}"#).unwrap();
        assert!(s.confirm);
        match &s.kind {
            StepKind::Hotkey { keys } => assert_eq!(keys, "Ctrl+C"),
            other => panic!("unexpected kind: {other:?}"),
        }
        // 序列化回 JSON：confirm 仍在顶层，键集合与旧版一致。
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["type"], "hotkey");
        assert_eq!(v["keys"], "Ctrl+C");
        assert_eq!(v["confirm"], true);
    }

    #[test]
    fn step_json_confirm_defaults_false() {
        let s: Step = serde_json::from_str(r#"{"type":"wait","ms":100}"#).unwrap();
        assert!(!s.confirm);
        match &s.kind {
            StepKind::Wait { ms } => assert_eq!(*ms, 100),
            other => panic!("unexpected kind: {other:?}"),
        }
    }

    #[test]
    fn click_legacy_corner_migration() {
        let s: Step = serde_json::from_str(
            r#"{"type":"click","title":"A","corner":"bottomRight","rx":0.5,"ry":0.08,"confirm":true}"#,
        )
        .unwrap();
        assert!(s.confirm);
        match &s.kind {
            StepKind::Click(c) => {
                assert_eq!(c.title, "A");
                assert_eq!(c.x.base, Base::Right);
                assert!((c.x.value - 0.5).abs() < 1e-9);
                assert_eq!(c.y.base, Base::Bottom);
                assert!((c.y.value - 0.08).abs() < 1e-9);
            }
            other => panic!("unexpected kind: {other:?}"),
        }
    }

    #[test]
    fn rollback_step_json_compat() {
        // 旧格式带 confirm 字段，新结构反序列化应忽略多余键并保留 confirm。
        let s: Step =
            serde_json::from_str(r#"{"type":"rollbackClipboard","confirm":true}"#).unwrap();
        assert!(s.confirm);
        assert!(matches!(s.kind, StepKind::RollbackClipboard));
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["type"], "rollbackClipboard");
        assert_eq!(v["confirm"], true);
    }

    /* ---------- 确认开关与回滚检测 ---------- */

    #[test]
    fn wait_step_never_confirms() {
        let s = Step { confirm: true, kind: StepKind::Wait { ms: 1 } };
        assert!(!step_confirm(&s));
        let h = Step { confirm: true, kind: StepKind::Hotkey { keys: "Enter".into() } };
        assert!(step_confirm(&h));
    }

    #[test]
    fn contains_rollback_finds_nested() {
        let nested: Step = serde_json::from_str(
            r#"{"type":"if","op":"eq","value":"a","expected":"b","then":[],
                "else":[{"type":"rollbackClipboard"}]}"#,
        )
        .unwrap();
        assert!(contains_rollback(&nested));
        let plain = Step { confirm: false, kind: StepKind::Wait { ms: 1 } };
        assert!(!contains_rollback(&plain));
    }
}
