import type { MenuSlot } from "../types";

/** 圆盘上一个扇区最终的显示项：绑定的配方 + 生效的外观。 */
export interface MenuItem {
  recipe: { name: string };
  /** 生效的显示名称（优先取扇区自定义 label，回退配方名）。 */
  label: string;
  /** 生效的颜色。 */
  color?: string;
  /** 生效的图标。 */
  icon?: string;
  /** 是否显示图标。 */
  showIcon?: boolean;
  /** 是否显示名称。 */
  showLabel?: boolean;
  /** 生效的显示名称字号。 */
  labelSize?: number;
  /** 生效的显示名称颜色。 */
  labelColor?: string;
}

/**
 * 按扇区绑定配置把配方排布到圆盘上：
 * 只显示显式绑定的扇区，不做任何自动填充；空扇区保持空。
 */
export function buildMenuItems(
  recipes: { name: string }[],
  sectors: number,
  slots: (MenuSlot | null)[]
): (MenuItem | null)[] {
  const out: (MenuItem | null)[] = Array(sectors).fill(null);
  (slots ?? []).forEach((s, i) => {
    if (i >= sectors || !s || s.recipe == null) return;
    const r = recipes[s.recipe];
    if (!r) return;
    out[i] = {
      recipe: r,
      label: s.label?.trim() || r.name,
      color: s.color,
      icon: s.icon,
      showIcon: s.showIcon,
      showLabel: s.showLabel,
      labelSize: s.labelSize,
      labelColor: s.labelColor,
    };
  });
  return out;
}

/** 把 slots 对齐到 sectors 长度，过滤失效绑定（越界索引），并拷贝每项。 */
export function normalizeSlots(
  slots: (MenuSlot | null)[] | undefined,
  sectors: number
): (MenuSlot | null)[] {
  const out: (MenuSlot | null)[] = Array(sectors).fill(null);
  (slots ?? []).forEach((s, i) => {
    if (i >= sectors || !s || s.recipe == null) return;
    out[i] = { ...s };
  });
  return out;
}
