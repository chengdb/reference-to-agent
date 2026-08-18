export type Base = "left" | "right" | "top" | "bottom";
export type Unit = "percent" | "px";

/** 单轴定位：从基准边向内偏移 value（单位 unit）。 */
export interface AxisPos {
  base: Base;
  value: number;
  unit: Unit;
}

/** 比较操作符。 */
export type CompareOp =
  | "eq"
  | "ne"
  | "gt"
  | "ge"
  | "lt"
  | "le"
  | "startsWith"
  | "endsWith"
  | "contains"
  | "matches";

/** 单个条件分支（if / else-if）：value op expected 时执行 then。 */
export interface CompareBranch {
  op: CompareOp;
  value: string;
  expected: string;
  then: Step[];
}

/** if 分叉步骤：value op expected ? then : (elseIf... | else)。 */
export interface IfBranch {
  op: CompareOp;
  value: string;
  expected: string;
  then: Step[];
  elseIf: CompareBranch[];
  else: Step[];
}

/** 步骤公共可选字段：执行前人工确认（需配方启用「人工确认」，Enter 确认 / Esc 取消）。 */
type StepFlags = { confirm?: boolean };

export type Step =
  | (StepFlags & { type: "wait"; ms: number })
  | (StepFlags & { type: "hotkey"; keys: string })
  | (StepFlags & { type: "activateApp"; title: string; exe?: string | null })
  | (StepFlags & { type: "focusApp"; title: string; exe?: string | null })
  | (StepFlags & { type: "setClipboard"; text: string })
  | (StepFlags & { type: "typeText"; text: string })
  | (StepFlags & { type: "pasteText"; text: string })
  | (StepFlags & { type: "runCommand"; cmd: string; args: string[] })
  | (StepFlags & { type: "click"; title: string; x: AxisPos; y: AxisPos })
  | (StepFlags & { type: "if"; op: CompareOp; value: string; expected: string; then: Step[]; elseIf: CompareBranch[]; else: Step[] })
  | (StepFlags & { type: "rollbackClipboard" });

export interface Recipe {
  name: string;
  /** 是否启用人工确认：启用后，勾选了 confirm 的步骤执行前会询问（Enter 确认 / Esc 取消）。 */
  confirm?: boolean;
  steps: Step[];
}

/** 圆盘菜单单个扇区（菜单按钮）的配置：绑定配方 + 外观。 */
export interface MenuSlot {
  /** 绑定的配方索引（0 起）。缺省表示未绑定。 */
  recipe?: number;
  /** 菜单显示名称，覆盖配方名。 */
  label?: string;
  /** 扇区自定义颜色（CSS 颜色值）。 */
  color?: string;
  /** 扇区图标（emoji 或短文本）。 */
  icon?: string;
  /** 是否显示图标（缺省显示）。 */
  showIcon?: boolean;
  /** 是否显示名称（缺省显示）。 */
  showLabel?: boolean;
  /** 显示名称字号（px）。 */
  labelSize?: number;
  /** 显示名称颜色（CSS 颜色值）。 */
  labelColor?: string;
}

export interface MenuConfig {
  /** 圆盘直径（px）。 */
  size: number;
  /** 扇区数量。 */
  sectors: number;
  /** 是否显示配方名称标签。 */
  showLabels: boolean;
  /** 按扇区索引排布的绑定配置，长度不超过 sectors。 */
  slots: (MenuSlot | null)[];
}

export interface Config {
  globalHotkey: string;
  listHotkey: string;
  recipes: Recipe[];
  menu: MenuConfig;
}

export interface AppEntry {
  name: string;
  exe: string;
}
