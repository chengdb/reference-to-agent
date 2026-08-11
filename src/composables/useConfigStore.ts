import { computed, effectScope, reactive, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { buildMenuItems, normalizeSlots, type MenuItem } from "../utils/menu";
import type { AppEntry, Config, MenuConfig, MenuSlot, Step } from "../types";

export type StepType = Step["type"];

export type Section = "basic" | "recipes" | "menu";

/** 编辑用步骤结构（宽松字段，保存时按 type 归一化）。 */
export interface EditableStep {
  type: StepType;
  ms?: number;
  keys?: string;
  title?: string;
  exe?: string | null;
  text?: string;
  cmd?: string;
  argsText?: string;
}

export interface EditableRecipe {
  name: string;
  steps: EditableStep[];
}

const DEFAULT_MENU: MenuConfig = {
  size: 400,
  sectors: 8,
  showLabels: true,
  slots: [{ recipe: 0 }, { recipe: 1 }],
};

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

const FONT_SIZES = [10, 11, 12, 13, 14, 15, 16, 18, 20];

const DEFAULT_CONFIG: Config = {
  globalHotkey: "Ctrl+Alt+R",
  menu: DEFAULT_MENU,
  recipes: [
    {
      name: "发送选中代码到 Claude",
      steps: [
        { type: "wait", ms: 50 },
        { type: "hotkey", keys: "Ctrl+C" },
        { type: "wait", ms: 150 },
        { type: "activateApp", title: "Claude", exe: null },
        { type: "wait", ms: 600 },
        { type: "hotkey", keys: "Ctrl+V" },
        { type: "wait", ms: 400 },
        { type: "hotkey", keys: "Enter" },
      ],
    },
    {
      name: "发送文件路径到 Claude",
      steps: [
        { type: "wait", ms: 50 },
        { type: "hotkey", keys: "Ctrl+Shift+C" },
        { type: "wait", ms: 150 },
        { type: "activateApp", title: "Claude", exe: null },
        { type: "wait", ms: 600 },
        { type: "hotkey", keys: "Ctrl+V" },
        { type: "wait", ms: 400 },
        { type: "hotkey", keys: "Enter" },
      ],
    },
  ],
};

const cfg = reactive<{ globalHotkey: string; recipes: EditableRecipe[]; menu: MenuConfig }>({
  globalHotkey: "",
  recipes: [],
  menu: { ...DEFAULT_MENU, slots: DEFAULT_MENU.slots.map((s) => ({ ...s })) },
});
const selectedIndex = ref(0);
const activeSection = ref<Section>("menu");
const selectedSlot = ref<number | null>(null);
const selectedStep = ref<number | null>(null);
const toast = ref<{ type: "success" | "error"; msg: string } | null>(null);
let toastTimer: number | undefined;
const recording = ref<string | null>(null);
const picker = ref<{ ri: number; si: number; list: AppEntry[] } | null>(null);
const pickerSearch = ref("");

const filteredApps = computed(() => {
  if (!picker.value) return [];
  const q = pickerSearch.value.trim().toLowerCase();
  if (!q) return picker.value.list;
  return picker.value.list.filter(
    (a) => a.name.toLowerCase().includes(q) || a.exe.toLowerCase().includes(q)
  );
});

function showToast(msg: string, type: "success" | "error" = "success") {
  toast.value = { type, msg };
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => (toast.value = null), 2500);
}

const current = computed(() => cfg.recipes[selectedIndex.value]);

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

function onPreviewSelect(i: number) {
  selectedSlot.value = i;
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

const scope = effectScope();
scope.run(() => {
  watch(
    () => cfg.menu.sectors,
    (n) => {
      cfg.menu.slots = normalizeSlots(cfg.menu.slots, n);
      if (selectedSlot.value != null && selectedSlot.value >= n) selectedSlot.value = null;
    }
  );
});

function selectStep(si: number) {
  selectedStep.value = si;
}

function toEditable(s: Step): EditableStep {
  const e: EditableStep = { type: s.type };
  switch (s.type) {
    case "wait":
      e.ms = s.ms;
      break;
    case "hotkey":
      e.keys = s.keys;
      break;
    case "activateApp":
      e.title = s.title;
      e.exe = s.exe ?? null;
      break;
    case "focusApp":
      e.title = s.title;
      e.exe = s.exe ?? null;
      break;
    case "setClipboard":
    case "typeText":
    case "pasteText":
      e.text = s.text;
      break;
    case "runCommand":
      e.cmd = s.cmd;
      e.argsText = s.args.join(", ");
      break;
  }
  return e;
}

function toStep(e: EditableStep): Step {
  switch (e.type) {
    case "wait":
      return { type: "wait", ms: Math.max(0, Math.round(Number(e.ms) || 0)) };
    case "hotkey":
      return { type: "hotkey", keys: (e.keys ?? "").trim() };
    case "activateApp":
      return {
        type: "activateApp",
        title: (e.title ?? "").trim(),
        exe: e.exe?.trim() || null,
      };
    case "focusApp":
      return {
        type: "focusApp",
        title: (e.title ?? "").trim(),
        exe: e.exe?.trim() || null,
      };
    case "setClipboard":
      return { type: "setClipboard", text: e.text ?? "" };
    case "typeText":
      return { type: "typeText", text: e.text ?? "" };
    case "pasteText":
      return { type: "pasteText", text: e.text ?? "" };
    case "runCommand":
      return {
        type: "runCommand",
        cmd: (e.cmd ?? "").trim(),
        args: (e.argsText ?? "")
          .split(",")
          .map((s) => s.trim())
          .filter(Boolean),
      };
  }
}

function buildConfig(): Config {
  return {
    globalHotkey: cfg.globalHotkey.trim() || "Ctrl+Alt+R",
    menu: {
      size: Math.round(cfg.menu.size),
      sectors: cfg.menu.sectors,
      showLabels: cfg.menu.showLabels,
      slots: Array.from({ length: cfg.menu.sectors }, (_, i) => {
        const s = cfg.menu.slots[i];
        if (!s || s.recipe == null) return null;
        return {
          recipe: s.recipe,
          ...(s.label?.trim() ? { label: s.label.trim() } : {}),
          ...(s.color ? { color: s.color } : {}),
          ...(s.icon?.trim() ? { icon: s.icon.trim() } : {}),
          ...(s.showIcon === false ? { showIcon: false } : {}),
          ...(s.showLabel === false ? { showLabel: false } : {}),
          ...(s.labelSize ? { labelSize: s.labelSize } : {}),
          ...(s.labelColor ? { labelColor: s.labelColor } : {}),
        };
      }),
    },
    recipes: cfg.recipes.map((r) => ({
      name: r.name.trim() || "未命名配方",
      steps: r.steps.map(toStep),
    })),
  };
}

async function load() {
  const c = await invoke<Config>("get_config");
  const base = c ?? DEFAULT_CONFIG;
  cfg.globalHotkey = base.globalHotkey;
  cfg.menu = { ...DEFAULT_MENU, ...(base.menu ?? {}) };
  cfg.menu.slots = normalizeSlots(cfg.menu.slots, cfg.menu.sectors);
  cfg.recipes = base.recipes.map((r) => ({
    name: r.name,
    steps: r.steps.map(toEditable),
  }));
  if (cfg.recipes.length > 0) selectedIndex.value = 0;
  selectedStep.value = null;
}

async function save() {
  try {
    await invoke("save_config", { cfg: buildConfig() });
    showToast("已保存并应用");
  } catch (e) {
    showToast(String(e), "error");
  }
}

function addRecipe() {
  cfg.recipes.push({
    name: `新配方 ${cfg.recipes.length + 1}`,
    steps: [
      { type: "wait", ms: 50 },
      { type: "hotkey", keys: "Ctrl+C" },
      { type: "wait", ms: 150 },
      { type: "activateApp", title: "Claude", exe: null },
      { type: "wait", ms: 600 },
      { type: "hotkey", keys: "Ctrl+V" },
      { type: "wait", ms: 400 },
      { type: "hotkey", keys: "Enter" },
    ],
  });
  selectedIndex.value = cfg.recipes.length - 1;
  selectedStep.value = null;
}

function removeRecipe(i: number) {
  if (!window.confirm(`删除配方“${cfg.recipes[i].name}”？`)) return;
  cfg.recipes.splice(i, 1);
  // 同步菜单绑定：删除的配方解绑，索引在其后的配方前移。
  cfg.menu.slots.forEach((s, si) => {
    if (!s) return;
    if (s.recipe === i) cfg.menu.slots[si] = null;
    else if (s.recipe != null && s.recipe > i) s.recipe--;
  });
  selectedStep.value = null;
  if (selectedIndex.value >= cfg.recipes.length) {
    selectedIndex.value = Math.max(0, cfg.recipes.length - 1);
  }
}

function selectRecipe(i: number) {
  selectedIndex.value = i;
  selectedStep.value = null;
}

function setSection(s: Section) {
  activeSection.value = s;
  selectedStep.value = null;
}

function addStepAt(i: number) {
  if (!current.value) return;
  current.value.steps.splice(i, 0, { type: "wait", ms: 100 });
  selectedStep.value = i;
}

function removeStep(si: number) {
  if (!current.value) return;
  current.value.steps.splice(si, 1);
  selectedStep.value = null;
}

function moveStep(si: number, dir: number) {
  if (!current.value) return;
  const steps = current.value.steps;
  const ni = si + dir;
  if (ni < 0 || ni >= steps.length) return;
  const tmp = steps[si];
  steps[si] = steps[ni];
  steps[ni] = tmp;
  if (selectedStep.value === si) selectedStep.value = ni;
}

/* ---------- 应用选择 ---------- */

async function openPicker(si: number) {
  try {
    const list = await invoke<AppEntry[]>("list_apps");
    picker.value = { ri: selectedIndex.value, si, list };
    pickerSearch.value = "";
  } catch (e) {
    showToast(String(e), "error");
  }
}

function closePicker() {
  picker.value = null;
}

function pickApp(app: AppEntry) {
  if (!picker.value) return;
  const step = cfg.recipes[picker.value.ri]?.steps[picker.value.si];
  if (step && (step.type === "activateApp" || step.type === "focusApp")) {
    step.title = app.name;
    step.exe = app.exe;
  }
  picker.value = null;
}

/* ---------- 热键录制 ---------- */

function comboFromEvent(e: KeyboardEvent): string | null {
  const mods: string[] = [];
  if (e.ctrlKey) mods.push("Ctrl");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push("Win");
  const map: Record<string, string> = {
    " ": "Space",
    Enter: "Enter",
    Tab: "Tab",
    Backspace: "Backspace",
    Delete: "Delete",
    Home: "Home",
    End: "End",
    PageUp: "PageUp",
    PageDown: "PageDown",
    Insert: "Insert",
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
    "\\": "\\",
  };
  if (map[e.key]) return [...mods, map[e.key]].join("+");
  if (/^F([1-9]|1[0-2])$/.test(e.key)) return [...mods, e.key].join("+");
  if (e.key.length === 1 && /[a-zA-Z0-9]/.test(e.key)) {
    return [...mods, e.key.toUpperCase()].join("+");
  }
  return null;
}

function startRecording(target: string) {
  recording.value = recording.value === target ? null : target;
}

function onKeydown(e: KeyboardEvent) {
  if (!recording.value) return;
  e.preventDefault();
  e.stopPropagation();
  if (e.key === "Escape") {
    recording.value = null;
    return;
  }
  const combo = comboFromEvent(e);
  if (!combo) return;
  if (recording.value === "global") {
    cfg.globalHotkey = combo;
  } else if (recording.value.startsWith("step:")) {
    const si = Number(recording.value.slice(5));
    const step = cfg.recipes[selectedIndex.value]?.steps[si];
    if (step && step.type === "hotkey") step.keys = combo;
  }
  recording.value = null;
}

export function useConfigStore() {
  return {
    cfg,
    selectedIndex,
    activeSection,
    selectedSlot,
    selectedStep,
    toast,
    recording,
    picker,
    pickerSearch,
    filteredApps,
    current,
    orderedPreview,
    boundSlot,
    selectedItem,
    sectorOptions,
    fontSizeOptions,
    recipeOptions,
    nameColorHistory,
    sectorColorHistory,
    load,
    save,
    showToast,
    setSection,
    selectRecipe,
    addRecipe,
    removeRecipe,
    selectStep,
    addStepAt,
    removeStep,
    moveStep,
    openPicker,
    closePicker,
    pickApp,
    startRecording,
    onKeydown,
    onPreviewSelect,
    onBindUpdate,
    setSlotLabel,
    setSlotIcon,
    setSlotLabelSize,
    setSlotLabelColor,
    setSlotColor,
    clearSlotLabelColor,
    clearSlotColor,
    setSlotShowIcon,
    setSlotShowLabel,
  };
}
