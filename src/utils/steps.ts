/**
 * 步骤的编辑态 ⇄ 持久化态转换与字段归一化。
 * 新增步骤类型时：同步 toEditable / toStep / normalizeStepFields 三处
 * （判别联合会让漏改的分支直接编译失败）。
 */
import type { AxisPos, Step } from "../types";
import type { EditableClickStep, EditableRecipe, EditableStep } from "../types/editable";

export function toEditable(s: Step): EditableStep {
  const confirm = s.confirm ?? undefined;
  switch (s.type) {
    case "wait":
      return { type: "wait", ms: s.ms, confirm };
    case "hotkey":
      return { type: "hotkey", keys: s.keys, confirm };
    case "activateApp":
      return { type: "activateApp", title: s.title, exe: s.exe ?? null, confirm };
    case "focusApp":
      return { type: "focusApp", title: s.title, exe: s.exe ?? null, confirm };
    case "setClipboard":
      return { type: "setClipboard", text: s.text, confirm };
    case "typeText":
      return { type: "typeText", text: s.text, confirm };
    case "pasteText":
      return { type: "pasteText", text: s.text, confirm };
    case "runCommand":
      return { type: "runCommand", cmd: s.cmd, argsText: (s.args ?? []).join(", "), confirm };
    case "click":
      return {
        type: "click",
        title: s.title,
        xBase: s.x?.base === "right" ? "right" : "left",
        xValue: s.x?.value,
        xUnit: s.x?.unit,
        yBase: s.y?.base === "bottom" ? "bottom" : "top",
        yValue: s.y?.value,
        yUnit: s.y?.unit,
        confirm,
      };
    case "if":
      return {
        type: "if",
        op: s.op,
        value: s.value,
        expected: s.expected,
        then: (s.then ?? []).map(toEditable),
        elseIf: (s.elseIf ?? []).map((b) => ({
          op: b.op,
          value: b.value,
          expected: b.expected,
          then: (b.then ?? []).map(toEditable),
        })),
        else: (s.else ?? []).map(toEditable),
        confirm,
      };
    case "rollbackClipboard":
      return { type: "rollbackClipboard", confirm };
  }
}

/** 把 click 步骤的 x/y 轴编辑字段收敛为 AxisPos（供序列化与测试点击复用）。 */
export function clickAxisPos(e: EditableClickStep): { x: AxisPos; y: AxisPos } {
  return {
    x: {
      base: e.xBase === "right" ? "right" : "left",
      value: Number(e.xValue) || 0,
      unit: e.xUnit === "px" ? "px" : "percent",
    },
    y: {
      base: e.yBase === "bottom" ? "bottom" : "top",
      value: Number(e.yValue) || 0,
      unit: e.yUnit === "px" ? "px" : "percent",
    },
  };
}

export function toStep(e: EditableStep): Step {
  const confirm = e.confirm === true;
  switch (e.type) {
    case "wait":
      // 等待步骤不注入任何操作，人工确认恒为 false。
      return { type: "wait", ms: Math.max(0, Math.round(Number(e.ms) || 0)), confirm: false };
    case "hotkey":
      return { type: "hotkey", keys: (e.keys ?? "").trim(), confirm };
    case "activateApp":
      return { type: "activateApp", title: (e.title ?? "").trim(), exe: e.exe?.trim() || null, confirm };
    case "focusApp":
      return { type: "focusApp", title: (e.title ?? "").trim(), exe: e.exe?.trim() || null, confirm };
    case "setClipboard":
      return { type: "setClipboard", text: e.text ?? "", confirm };
    case "typeText":
      return { type: "typeText", text: e.text ?? "", confirm };
    case "pasteText":
      return { type: "pasteText", text: e.text ?? "", confirm };
    case "runCommand":
      return {
        type: "runCommand",
        cmd: (e.cmd ?? "").trim(),
        args: (e.argsText ?? "")
          .split(",")
          .map((s) => s.trim())
          .filter(Boolean),
        confirm,
      };
    case "click":
      return { type: "click", title: (e.title ?? "").trim(), ...clickAxisPos(e), confirm };
    case "if":
      return {
        type: "if",
        op: e.op ?? "eq",
        value: e.value ?? "",
        expected: e.expected ?? "",
        then: (e.then ?? []).map(toStep),
        elseIf: (e.elseIf ?? []).map((b) => ({
          op: b.op,
          value: b.value,
          expected: b.expected,
          then: b.then.map(toStep),
        })),
        else: (e.else ?? []).map(toStep),
        confirm,
      };
    case "rollbackClipboard":
      return { type: "rollbackClipboard", confirm };
  }
}

/**
 * 递归补齐 click / if 步骤的默认字段（切换到该类型时调用）。
 * 等待步骤不注入任何操作，无需人工确认：切换到 wait 时自动清掉该标记。
 */
export function normalizeStepFields(steps: EditableStep[]) {
  for (const s of steps) {
    if (s.type === "wait") {
      s.confirm = false;
    } else if (s.type === "click") {
      if (s.xBase == null) s.xBase = "left";
      if (s.xValue == null) s.xValue = 0.5;
      if (s.xUnit == null) s.xUnit = "percent";
      if (s.yBase == null) s.yBase = "bottom";
      if (s.yValue == null) s.yValue = 0.08;
      if (s.yUnit == null) s.yUnit = "percent";
    } else if (s.type === "if") {
      if (s.op == null) s.op = "eq";
      if (s.value == null) s.value = "";
      if (s.expected == null) s.expected = "";
      if (s.then == null) s.then = [];
      if (s.elseIf == null) s.elseIf = [];
      if (s.else == null) s.else = [];
      normalizeStepFields(s.then);
      for (const b of s.elseIf) {
        if (b.then == null) b.then = [];
        normalizeStepFields(b.then);
      }
      normalizeStepFields(s.else);
    }
  }
}

/**
 * 新建配方时的初始步骤模板（前端唯一一份），仅服务于「+ 添加配方」。
 * 注意：首次安装的默认配置（src-tauri/src/config.rs 的 Config::default()）
 * 不含任何配方与菜单绑定，不再与本模板对应。
 */
export function newRecipeTemplateSteps(): EditableStep[] {
  return [
    { type: "wait", ms: 50 },
    { type: "hotkey", keys: "Ctrl+C" },
    { type: "wait", ms: 150 },
    { type: "activateApp", title: "Claude", exe: null },
    { type: "wait", ms: 600 },
    { type: "hotkey", keys: "Ctrl+V" },
    { type: "wait", ms: 400 },
    { type: "hotkey", keys: "Enter" },
  ];
}

export type { EditableRecipe };
