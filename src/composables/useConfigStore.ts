import { computed, effectScope, reactive, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { buildMenuItems, normalizeSlots, type MenuItem } from "../utils/menu";
import type {
  AppEntry,
  AxisPos,
  CompareOp,
  Config,
  MenuConfig,
  MenuSlot,
  Step,
} from "../types";

export type StepType = Step["type"];

export type Section = "basic" | "recipes" | "menu";

/** 编辑用分支结构：else-if 可多个，字段宽松。 */
export interface EditableCompareBranch {
  op: CompareOp;
  value: string;
  expected: string;
  then: EditableStep[];
}

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
  /** click 步骤：x/y 轴各自独立定位。 */
  xBase?: "left" | "right";
  xValue?: number;
  xUnit?: "percent" | "px";
  yBase?: "top" | "bottom";
  yValue?: number;
  yUnit?: "percent" | "px";
  /** if 分叉步骤。 */
  op?: CompareOp;
  value?: string;
  expected?: string;
  then?: EditableStep[];
  elseIf?: EditableCompareBranch[];
  else?: EditableStep[];
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
/** 当前选中的步骤（对象引用，支持嵌套分支）。 */
const selectedStep = ref<EditableStep | null>(null);
const toast = ref<{ type: "success" | "error"; msg: string } | null>(null);
let toastTimer: number | undefined;
/** 热键录制目标：'global' 或 'step'（配合 recordingStep）。 */
const recording = ref<"global" | "step" | null>(null);
/** 正在录制热键的步骤（recording === 'step' 时有效）。 */
const recordingStep = ref<EditableStep | null>(null);
const picker = ref<{ step: EditableStep | null; list: AppEntry[] } | null>(null);
const pickerSearch = ref("");
/** 正在拾取点击坐标的步骤（对象引用）。 */
const coordPicking = ref<EditableStep | null>(null);

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
  // 步骤类型切换为 click / if 时补齐默认字段，避免相关字段未定义（含嵌套分支）。
  watch(
    () => JSON.stringify(cfg.recipes.map((r) => stepsShape(r.steps))),
    () => {
      for (const r of cfg.recipes) {
        normalizeStepFields(r.steps);
      }
    },
    { immediate: true }
  );
});

/** 序列化步骤树的“形状”（仅类型与分支结构），作为 watch 触发源。 */
function stepsShape(steps: EditableStep[]): unknown {
  return steps.map((s) =>
    s.type === "if"
      ? {
          type: s.type,
          then: stepsShape(s.then ?? []),
          elseIf: (s.elseIf ?? []).map((b) => b.then && stepsShape(b.then)),
          else: stepsShape(s.else ?? []),
        }
      : s.type
  );
}

/** 递归补齐 click / if 步骤的默认字段。 */
function normalizeStepFields(steps: EditableStep[]) {
  for (const s of steps) {
    if (s.type === "click") {
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

function selectStep(step: EditableStep | null) {
  selectedStep.value = step;
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
    case "click":
      e.title = s.title;
      e.xBase = s.x.base === "right" ? "right" : "left";
      e.xValue = s.x.value;
      e.xUnit = s.x.unit;
      e.yBase = s.y.base === "bottom" ? "bottom" : "top";
      e.yValue = s.y.value;
      e.yUnit = s.y.unit;
      break;
    case "if":
      e.op = s.op;
      e.value = s.value;
      e.expected = s.expected;
      e.then = s.then.map(toEditable);
      e.elseIf = s.elseIf.map((b) => ({
        op: b.op,
        value: b.value,
        expected: b.expected,
        then: b.then.map(toEditable),
      }));
      e.else = s.else.map(toEditable);
      break;
    case "rollbackClipboard":
      break;
  }
  return e;
}

/** 把 click 步骤的 x/y 轴编辑字段收敛为 AxisPos（供序列化与测试点击复用）。 */
function clickAxisPos(e: EditableStep): { x: AxisPos; y: AxisPos } {
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
    case "click":
      return { type: "click", title: (e.title ?? "").trim(), ...clickAxisPos(e) };
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
      };
    case "rollbackClipboard":
      return { type: "rollbackClipboard" };
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

function uniqueRecipeName(base: string) {
  const names = new Set(cfg.recipes.map((r) => r.name));
  if (!names.has(base)) return base;
  let n = 2;
  while (names.has(`${base} ${n}`)) n++;
  return `${base} ${n}`;
}

/** 复制配方：拷贝步骤（EditableStep 字段均为值类型，展开即充分隔离），取一个不重名的新名字，并选中副本。 */
function duplicateRecipe(i: number) {
  const src = cfg.recipes[i];
  if (!src) return;
  const copy: EditableRecipe = {
    name: uniqueRecipeName(`${src.name} 副本`),
    steps: src.steps.map((s) => ({ ...s })),
  };
  cfg.recipes.splice(i + 1, 0, copy);
  selectedIndex.value = i + 1;
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

function addStepAt(list: EditableStep[], i: number) {
  list.splice(i, 0, { type: "wait", ms: 100 });
  selectedStep.value = list[i] ?? null;
}

function removeStep(list: EditableStep[], i: number) {
  list.splice(i, 1);
  selectedStep.value = null;
}

function moveStep(list: EditableStep[], i: number, dir: number) {
  const ni = i + dir;
  if (ni < 0 || ni >= list.length) return;
  const tmp = list[i];
  list[i] = list[ni];
  list[ni] = tmp;
  selectedStep.value = list[ni];
}

/* ---------- 应用选择 ---------- */

async function openPicker(step: EditableStep) {
  try {
    const list = await invoke<AppEntry[]>("list_apps");
    picker.value = { step, list };
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
  const step = picker.value.step;
  if (step && (step.type === "activateApp" || step.type === "focusApp")) {
    step.title = app.name;
    step.exe = app.exe;
  }
  picker.value = null;
}

/* ---------- 点击坐标拾取 ---------- */

/**
 * 开启坐标拾取：让用户把鼠标移到目标窗口输入框上并停留，然后按下 Enter。
 * 后端记录鼠标位置并换算为「相对目标窗口」的百分比坐标，回填到当前 click 步骤。
 */
function startCoordPicking(step: EditableStep) {
  coordPicking.value = coordPicking.value === step ? null : step;
  // 拾取与热键录制互斥：避免 Enter 同时触发坐标拾取与热键录制。
  if (coordPicking.value != null) {
    recording.value = null;
    recordingStep.value = null;
  }
}

function onCoordPickKeydown(e: KeyboardEvent) {
  const step = coordPicking.value;
  if (step == null) return;
  if (e.key === "Escape") {
    coordPicking.value = null;
    return;
  }
  if (e.key !== "Enter") return;
  e.preventDefault();
  if (step.type !== "click") {
    coordPicking.value = null;
    return;
  }
  pickClickCoords(step).finally(() => {
    coordPicking.value = null;
  });
}

async function pickClickCoords(step: EditableStep) {
  try {
    const { x, y } = await invoke<{ x: AxisPos; y: AxisPos }>("pick_click_coords", {
      title: step.title ?? "",
      xUnit: step.xUnit === "px" ? "px" : "percent",
      yUnit: step.yUnit === "px" ? "px" : "percent",
    });
    step.xBase = x.base === "right" ? "right" : "left";
    step.xValue = x.value;
    step.xUnit = x.unit;
    step.yBase = y.base === "bottom" ? "bottom" : "top";
    step.yValue = y.value;
    step.yUnit = y.unit;
    showToast(
      `已拾取坐标 x=${x.base}+${(x.value * (x.unit === "percent" ? 100 : 1)).toFixed(1)}${x.unit === "percent" ? "%" : "px"}, y=${y.base}+${(y.value * (y.unit === "percent" ? 100 : 1)).toFixed(1)}${y.unit === "percent" ? "%" : "px"}`
    );
  } catch (e) {
    showToast(String(e), "error");
  }
}

/** 按当前 x/y 轴配置测试点击一次，验证坐标是否落在输入框上。 */
async function testClick(step: EditableStep) {
  if (!step || step.type !== "click") return;
  try {
    await invoke("test_click", {
      title: step.title ?? "",
      ...clickAxisPos(step),
    });
    showToast("已发送测试点击");
  } catch (e) {
    showToast(String(e), "error");
  }
}

/** 查询标题匹配窗口的真实标题，用于验证聚焦/激活步骤的目标是否匹配正确。 */
async function getWindowInfo(step: EditableStep) {
  if (!step || (step.type !== "activateApp" && step.type !== "focusApp")) return;
  const title = (step.title ?? "").trim();
  if (!title) {
    showToast("请先填写窗口标题", "error");
    return;
  }
  try {
    const realTitle = await invoke<string>("get_window_info", { title });
    showToast(`已匹配窗口标题：${realTitle}`);
  } catch (e) {
    showToast(String(e), "error");
  }
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

/** 开启热键录制：target 传 'global' 录制全局热键，传步骤对象录制该步骤的 keys。 */
function startRecording(target: "global" | EditableStep) {
  const currently = recording.value;
  if (target === "global") {
    recording.value = currently === "global" ? null : "global";
    recordingStep.value = null;
  } else {
    const isSame = recording.value === "step" && recordingStep.value === target;
    recording.value = isSame ? null : "step";
    recordingStep.value = isSame ? null : target;
  }
  // 录制与坐标拾取互斥（见 startCoordPicking）。
  if (recording.value != null) coordPicking.value = null;
}

function onKeydown(e: KeyboardEvent) {
  if (!recording.value) return;
  e.preventDefault();
  e.stopPropagation();
  if (e.key === "Escape") {
    recording.value = null;
    recordingStep.value = null;
    return;
  }
  const combo = comboFromEvent(e);
  if (!combo) return;
  if (recording.value === "global") {
    cfg.globalHotkey = combo;
  } else if (recording.value === "step") {
    const step = recordingStep.value;
    if (step && step.type === "hotkey") step.keys = combo;
  }
  recording.value = null;
  recordingStep.value = null;
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
    recordingStep,
    picker,
    pickerSearch,
    filteredApps,
    coordPicking,
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
    duplicateRecipe,
    removeRecipe,
    selectStep,
    addStepAt,
    removeStep,
    moveStep,
    openPicker,
    closePicker,
    pickApp,
    startCoordPicking,
    onCoordPickKeydown,
    testClick,
    getWindowInfo,
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
