export type Step =
  | { type: "wait"; ms: number }
  | { type: "hotkey"; keys: string }
  | { type: "activateApp"; title: string; exe?: string | null }
  | { type: "focusApp"; title: string; exe?: string | null }
  | { type: "setClipboard"; text: string }
  | { type: "typeText"; text: string }
  | { type: "pasteText"; text: string }
  | { type: "runCommand"; cmd: string; args: string[] };

export interface Recipe {
  name: string;
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
  recipes: Recipe[];
  menu: MenuConfig;
}

export interface AppEntry {
  name: string;
  exe: string;
}
