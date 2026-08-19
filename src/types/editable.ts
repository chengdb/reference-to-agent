/**
 * 配置编辑器使用的「编辑用」类型：字段宽松（全部可选），保存时经
 * src/utils/steps.ts 的 toStep 归一化为 src/types/index.ts 的 Step。
 *
 * 与 Step 不同的是：EditableStep 按 type 判别成联合，编辑器组件可按类型
 * 精确取字段；新增步骤类型时同步扩展此联合（编译器会强制处理各 switch）。
 */
import type { CompareOp } from "./index";

/** 编辑用分支结构：else-if 可多个，字段宽松。 */
export interface EditableCompareBranch {
  op: CompareOp;
  value: string;
  expected: string;
  then: EditableStep[];
}

interface EditableFlags {
  /** 执行前人工确认（需配方启用「人工确认」）。 */
  confirm?: boolean;
}

/** click 步骤的 x/y 轴编辑字段。 */
interface EditableClickFields {
  title?: string;
  xBase?: "left" | "right";
  xValue?: number;
  xUnit?: "percent" | "px";
  yBase?: "top" | "bottom";
  yValue?: number;
  yUnit?: "percent" | "px";
}

export type EditableStep =
  | (EditableFlags & { type: "wait"; ms?: number })
  | (EditableFlags & { type: "hotkey"; keys?: string })
  | (EditableFlags & { type: "activateApp"; title?: string; exe?: string | null })
  | (EditableFlags & { type: "focusApp"; title?: string; exe?: string | null })
  | (EditableFlags & { type: "setClipboard"; text?: string })
  | (EditableFlags & { type: "typeText"; text?: string })
  | (EditableFlags & { type: "pasteText"; text?: string })
  | (EditableFlags & { type: "runCommand"; cmd?: string; argsText?: string })
  | (EditableFlags & { type: "click" } & EditableClickFields)
  | (EditableFlags & {
      type: "if";
      op?: CompareOp;
      value?: string;
      expected?: string;
      then?: EditableStep[];
      elseIf?: EditableCompareBranch[];
      else?: EditableStep[];
    })
  | (EditableFlags & { type: "rollbackClipboard" });

export interface EditableRecipe {
  name: string;
  /** 配方级人工确认开关：启用后，勾选了 confirm 的步骤执行前会询问。 */
  confirm?: boolean;
  steps: EditableStep[];
}

/* 常用判别结果别名，供编辑器组件 props 使用。 */
export type EditableWaitStep = Extract<EditableStep, { type: "wait" }>;
export type EditableHotkeyStep = Extract<EditableStep, { type: "hotkey" }>;
export type EditableAppTargetStep = Extract<EditableStep, { exe?: string | null }>;
export type EditableTextStep = Extract<EditableStep, { text?: string }>;
export type EditableRunCommandStep = Extract<EditableStep, { type: "runCommand" }>;
export type EditableClickStep = Extract<EditableStep, { type: "click" }>;
export type EditableIfStep = Extract<EditableStep, { type: "if" }>;
