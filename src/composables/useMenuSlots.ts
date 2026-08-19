import { computed, ref, watch } from "vue";
import { buildMenuItems, normalizeSlots, type MenuItem } from "../utils/menu";
import { useConfigState } from "./useConfigState";
import type { MenuSlot } from "../types";

const { cfg } = useConfigState();

const selectedSlot = ref<number | null>(null);

const FONT_SIZES = [10, 11, 12, 13, 14, 15, 16, 18, 20];

const COLOR_HISTORY_KEYS = {
  label: "menu-color-history:name",
  sector: "menu-color-history:sector",
} as const;

function loadColorHistory(key: string): string[] {
  try {
    const raw = localStorage.getItem(key);
    const arr = raw ? JSON.parse(raw) : [];
    return Array.isArray(arr)
      ? arr.filter((c): c is string => typeof c === "string").slice(0, 6)
      : [];
  } catch {
    return [];
  }
}

const nameColorHistory = ref<string[]>(loadColorHistory(COLOR_HISTORY_KEYS.label));
const sectorColorHistory = ref<string[]>(loadColorHistory(COLOR_HISTORY_KEYS.sector));

function pushColorHistory(key: string, color: string) {
  const list = key === COLOR_HISTORY_KEYS.label ? nameColorHistory : sectorColorHistory;
  const next = [color, ...list.value.filter((c) => c !== color)];
  list.value = next.slice(0, 6);
  try {
    localStorage.setItem(key, JSON.stringify(list.value));
  } catch {
    /* ignore */
  }
}

// 扇区数量变化时：slots 对齐到新数量，失效的选中态清除。
// 模块级单例 watch，随应用存活（无需 effectScope 管理）。
watch(
  () => cfg.menu.sectors,
  (n) => {
    cfg.menu.slots = normalizeSlots(cfg.menu.slots, n);
    if (selectedSlot.value != null && selectedSlot.value >= n) selectedSlot.value = null;
  }
);

const orderedPreview = computed<(MenuItem | null)[]>(() =>
  buildMenuItems(cfg.recipes, cfg.menu.sectors, cfg.menu.slots)
);

/** 选中扇区当前的绑定配置对象引用（未绑定时为 null）。 */
const boundSlot = computed<MenuSlot | null>(() => {
  if (selectedSlot.value == null) return null;
  return cfg.menu.slots[selectedSlot.value] ?? null;
});

/** 选中扇区绑定后的显示项（含生效外观），用于绑定面板预览。 */
const selectedItem = computed<MenuItem | null>(() =>
  selectedSlot.value == null ? null : (orderedPreview.value[selectedSlot.value] ?? null)
);

const sectorOptions = computed(() =>
  Array.from({ length: 10 }, (_, i) => ({ value: i + 4, label: String(i + 4) }))
);
const fontSizeOptions = computed(() =>
  FONT_SIZES.map((s) => ({ value: s, label: s + "px" }))
);
const recipeOptions = computed(() => [
  { value: "", label: "（不绑定）" },
  ...cfg.recipes.map((r, i) => ({ value: i, label: r.name })),
]);

function onPreviewSelect(i: number) {
  selectedSlot.value = i;
}

/** 下拉框把选中扇区绑定/解绑配方；扇区可复用同一配方（单向绑定）。 */
function onBindUpdate(raw: string | number | null) {
  if (selectedSlot.value == null) return;
  if (raw === "" || raw == null) {
    cfg.menu.slots[selectedSlot.value] = null;
    return;
  }
  const ri = Number(raw);
  if (!Number.isInteger(ri)) return;
  const old = cfg.menu.slots[selectedSlot.value];
  cfg.menu.slots[selectedSlot.value] = {
    recipe: ri,
    label: old?.label,
    color: old?.color,
    icon: old?.icon,
    labelSize: old?.labelSize,
    labelColor: old?.labelColor,
  };
}

function setSlotField<K extends keyof MenuSlot>(key: K, value: MenuSlot[K] | undefined) {
  if (boundSlot.value) boundSlot.value[key] = value as MenuSlot[K];
}

function setSlotLabel(v: string) {
  setSlotField("label", v.trim() ? v : undefined);
}

function setSlotIcon(v: string) {
  setSlotField("icon", v.trim() ? v : undefined);
}

function setSlotLabelSize(v: number) {
  setSlotField("labelSize", v);
}

function setSlotLabelColor(v: string) {
  setSlotField("labelColor", v || undefined);
  if (v) pushColorHistory(COLOR_HISTORY_KEYS.label, v);
}

function clearSlotLabelColor() {
  setSlotField("labelColor", undefined);
}

function setSlotColor(v: string) {
  setSlotField("color", v || undefined);
  if (v) pushColorHistory(COLOR_HISTORY_KEYS.sector, v);
}

function clearSlotColor() {
  setSlotField("color", undefined);
}

function setSlotShowIcon(v: boolean) {
  setSlotField("showIcon", v ? undefined : false);
}

function setSlotShowLabel(v: boolean) {
  setSlotField("showLabel", v ? undefined : false);
}

/** 圆盘菜单设置：扇区选择、配方绑定与按钮外观（含颜色历史）。 */
export function useMenuSlots() {
  return {
    selectedSlot,
    orderedPreview,
    boundSlot,
    selectedItem,
    sectorOptions,
    fontSizeOptions,
    recipeOptions,
    nameColorHistory,
    sectorColorHistory,
    onPreviewSelect,
    onBindUpdate,
    setSlotLabel,
    setSlotIcon,
    setSlotLabelSize,
    setSlotLabelColor,
    clearSlotLabelColor,
    setSlotColor,
    clearSlotColor,
    setSlotShowIcon,
    setSlotShowLabel,
  };
}
