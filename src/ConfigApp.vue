<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import StepTypeSelect from "./StepTypeSelect.vue";
import Select from "./Select.vue";
import Radial from "./Radial.vue";
import { buildMenuItems, normalizeSlots, type MenuItem } from "./menu";
import type { AppEntry, Config, MenuConfig, MenuSlot, Step } from "./types";

type StepType = Step["type"];

type Section = "basic" | "recipes" | "menu";

/** 编辑用步骤结构（宽松字段，保存时按 type 归一化）。 */
interface EditableStep {
  type: StepType;
  ms?: number;
  keys?: string;
  title?: string;
  exe?: string | null;
  text?: string;
  cmd?: string;
  argsText?: string;
}

interface EditableRecipe {
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

const nameColorHistory = ref<string[]>(loadColorHistory(COLOR_HISTORY_KEYS.label));
const sectorColorHistory = ref<string[]>(loadColorHistory(COLOR_HISTORY_KEYS.sector));

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
const selectedStep = ref<number | null>(null);

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

watch(
  () => cfg.menu.sectors,
  (n) => {
    cfg.menu.slots = normalizeSlots(cfg.menu.slots, n);
    if (selectedSlot.value != null && selectedSlot.value >= n) selectedSlot.value = null;
  }
);

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

onMounted(async () => {
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
});
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

onMounted(() => window.addEventListener("keydown", onKeydown, true));
onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown, true);
  window.clearTimeout(toastTimer);
});
</script>

<template>
  <div class="config">
    <div class="config-head">
      <div class="config-brand">
        <div class="config-logo"></div>
        <div>
          <div class="config-title">Reference to Agent</div>
          <div class="config-sub">把代码 / 文件路径一键发送给 AI Agent</div>
        </div>
      </div>
      <div class="config-actions">
        <button class="btn btn-primary" @click="save">保存并应用</button>
      </div>
    </div>

    <div class="config-body">
      <aside class="sidebar">
        <button
          class="sidebar-item"
          :class="{ active: activeSection === 'menu' }"
          @click="setSection('menu')"
        >
          <svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v3M12 18v3M3 12h3M18 12h3" /><circle cx="12" cy="12" r="4.5" /></svg>
          <span>菜单设置</span>
        </button>
        <button
          class="sidebar-item"
          :class="{ active: activeSection === 'recipes' }"
          @click="setSection('recipes')"
        >
          <svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 3h6M10 3v5.5L5.5 18a2 2 0 0 0 1.8 3h9.4a2 2 0 0 0 1.8-3L14 8.5V3" /><path d="M7.5 15h9" /></svg>
          <span>配方设置</span>
        </button>
        <button
          class="sidebar-item"
          :class="{ active: activeSection === 'basic' }"
          @click="setSection('basic')"
        >
          <svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.3 3.3a1.8 1.8 0 0 1 3.4 0l.7 2a1.8 1.8 0 0 0 1.3 1.2l2.1.4a1.8 1.8 0 0 1 1 2.9l-1.4 1.6a1.8 1.8 0 0 0-.4 1.7l.6 2a1.8 1.8 0 0 1-2.5 2.2l-1.9-1a1.8 1.8 0 0 0-1.7 0l-1.9 1a1.8 1.8 0 0 1-2.5-2.2l.6-2a1.8 1.8 0 0 0-.4-1.7L5.1 10a1.8 1.8 0 0 1 1-2.9l2.1-.4a1.8 1.8 0 0 0 1.3-1.2z" /><circle cx="12" cy="12" r="3" /></svg>
          <span>全局设置</span>
        </button>
      </aside>

      <div class="config-panel">
        <section v-show="activeSection === 'basic'" class="card config-section basic-panel">
          <div class="config-label">全局快捷键 · 弹出菜单</div>
          <div class="hotkey-row">
            <input
              class="config-input hotkey-input"
              :value="cfg.globalHotkey"
              readonly
              :placeholder="recording === 'global' ? '请按下组合键…' : '点击右侧「录制」设置'"
            />
            <button
              class="rec-btn"
              :class="{ recording: recording === 'global' }"
              @click="startRecording('global')"
            >
              {{ recording === "global" ? "录制中 · Esc 取消" : "录制" }}
            </button>
          </div>
        </section>

        <section v-show="activeSection === 'recipes'" class="card config-section recipe-editor">
      <div class="recipe-list">
        <div class="recipe-list-title">配方</div>
        <div
          v-for="(r, i) in cfg.recipes"
          :key="i"
          class="recipe-item"
          :class="{ active: i === selectedIndex }"
          @click="selectRecipe(i)"
        >
          <span class="recipe-item-name" :title="r.name">{{ r.name }}</span>
          <button class="recipe-del" title="删除配方" @click.stop="removeRecipe(i)">
            ×
          </button>
        </div>
        <button class="recipe-add" @click="addRecipe">+ 添加配方</button>
      </div>

      <div v-if="current" class="recipe-edit">
        <div class="recipe-name-row">
          <input v-model="current.name" class="config-input recipe-name" placeholder="配方名称" />
          <button class="btn btn-primary recipe-save" @click="save">保存配方</button>
        </div>
        <div class="step-list">
          <button class="step-insert" title="在开头插入步骤" @click="addStepAt(0)">
            <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14M5 12h14" /></svg>
          </button>
          <template v-for="(step, si) in current.steps" :key="si">
            <div class="step-row" :class="{ active: selectedStep === si }" @click="selectStep(si)">
            <span class="step-index">{{ si + 1 }}</span>
            <div class="step-body">
              <StepTypeSelect v-model="step.type" class="step-type" />

              <template v-if="step.type === 'wait'">
                <input v-model.number="step.ms" type="number" min="0" class="config-input step-ms" placeholder="毫秒" />
                <span class="step-unit">ms</span>
              </template>

              <template v-else-if="step.type === 'hotkey'">
                <input
                  class="config-input step-keys"
                  :value="step.keys"
                  readonly
                  :placeholder="recording === 'step:' + si ? '请按下组合键…' : '录制或手填'"
                />
                <button
                  class="rec-btn small"
                  :class="{ recording: recording === 'step:' + si }"
                  @click="startRecording('step:' + si)"
                >
                  {{ recording === "step:" + si ? "…" : "录制" }}
                </button>
              </template>

              <template v-else-if="step.type === 'activateApp'">
                <input
                  v-model="step.title"
                  class="config-input step-title"
                  placeholder="窗口标题（模糊匹配）"
                />
                <button class="pick-btn" @click="openPicker(si)">选择应用…</button>
                <span v-if="step.exe" class="step-exe-display" :title="step.exe">{{ step.exe }}</span>
              </template>

              <template v-else-if="step.type === 'focusApp'">
                <input
                  v-model="step.title"
                  class="config-input step-title"
                  placeholder="窗口标题（模糊匹配）"
                />
                <button class="pick-btn" @click="openPicker(si)">选择应用…</button>
                <span v-if="step.exe" class="step-exe-display" :title="step.exe">{{ step.exe }}</span>
              </template>

              <template v-else-if="step.type === 'setClipboard' || step.type === 'typeText' || step.type === 'pasteText'">
                <input v-model="step.text" class="config-input step-text" placeholder="内容" />
              </template>

              <template v-else-if="step.type === 'runCommand'">
                <input v-model="step.cmd" class="config-input step-cmd" placeholder="命令" />
                <input v-model="step.argsText" class="config-input step-args" placeholder="参数，逗号分隔" />
              </template>
            </div>
            <div class="step-controls">
              <button title="上移" :disabled="si === 0" @click="moveStep(si, -1)">
                <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M18 15l-6-6-6 6" /></svg>
              </button>
              <button
                title="下移"
                :disabled="si === current.steps.length - 1"
                @click="moveStep(si, 1)"
              >
                <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M6 9l6 6 6-6" /></svg>
              </button>
              <button class="step-del" title="删除步骤" @click="removeStep(si)">
                <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6L6 18M6 6l12 12" /></svg>
              </button>
            </div>
            </div>
            <button class="step-insert" title="在此处插入步骤" @click="addStepAt(si + 1)">
              <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14M5 12h14" /></svg>
            </button>
          </template>
        </div>
      </div>
      <div v-else class="recipe-edit empty">选择一个配方进行编辑，或点击「+ 添加配方」</div>
        </section>

        <section v-show="activeSection === 'menu'" class="card config-section menu-panel">
          <div class="menu-settings-main">
            <div class="ring-area">
              <div class="ring-controls">
                <div class="setting-card">
                  <label for="menu-size">菜单大小</label>
                  <div class="setting-card-control">
                    <input
                      id="menu-size"
                      class="range-input"
                      type="range"
                      min="320"
                      max="600"
                      step="10"
                      v-model.number="cfg.menu.size"
                    />
                    <span class="setting-value">{{ cfg.menu.size }}px</span>
                  </div>
                </div>

                <div class="setting-card">
                  <label for="menu-sectors">扇区数量</label>
                  <Select
                    id="menu-sectors"
                    class="setting-select"
                    v-model.number="cfg.menu.sectors"
                    :options="sectorOptions"
                  />
                </div>
              </div>

              <div class="radial-preview">
                <Radial
                  :items="orderedPreview"
                  :size="cfg.menu.size"
                  :sectors="cfg.menu.sectors"
                  :show-labels="cfg.menu.showLabels"
                  :selected="selectedSlot"
                  interactive
                  allow-empty-select
                  show-indices
                  @select="onPreviewSelect"
                />
              </div>
            </div>

            <div class="bind-panel">
              <div class="bind-head">
                <div class="bind-title">
                  <template v-if="selectedSlot != null">第 {{ selectedSlot + 1 }} 个扇区</template>
                  <template v-else>未选择扇区</template>
                </div>
                <div class="bind-current">
                  <template v-if="selectedItem">
                    {{ selectedItem.label }}
                  </template>
                  <template v-else-if="selectedSlot != null">尚未绑定配方</template>
                  <template v-else>点击左侧圆环选择一个扇区</template>
                </div>
              </div>

              <div class="config-label bind-label">绑定配方</div>
              <Select
                class="bind-select"
                :model-value="boundSlot?.recipe ?? ''"
                :disabled="selectedSlot == null"
                :options="recipeOptions"
                placeholder="点击左侧圆环选择一个扇区"
                @update:model-value="onBindUpdate"
              />

              <div
                v-if="boundSlot && boundSlot.recipe != null"
                class="bind-appearance"
              >
                <div class="config-label">菜单按钮外观</div>
                <div class="setting-row">
                  <label for="slot-label">显示名称</label>
                  <input
                    id="slot-label"
                    class="config-input"
                    :value="boundSlot.label ?? ''"
                    placeholder="默认显示配方名"
                    :disabled="boundSlot.showLabel === false"
                    @input="setSlotLabel(($event.target as HTMLInputElement).value)"
                  />
                  <label class="show-check" title="是否显示名称">
                    <input
                      type="checkbox"
                      :checked="boundSlot.showLabel !== false"
                      @change="setSlotShowLabel(($event.target as HTMLInputElement).checked)"
                    />
                    <span>显示</span>
                  </label>
                </div>
                <div class="setting-row">
                  <label for="slot-icon">图标</label>
                  <input
                    id="slot-icon"
                    class="config-input"
                    :value="boundSlot.icon ?? ''"
                    placeholder="emoji，如 🧰"
                    :disabled="boundSlot.showIcon === false"
                    @input="setSlotIcon(($event.target as HTMLInputElement).value)"
                  />
                  <label class="show-check" title="是否显示图标">
                    <input
                      type="checkbox"
                      :checked="boundSlot.showIcon !== false"
                      @change="setSlotShowIcon(($event.target as HTMLInputElement).checked)"
                    />
                    <span>显示</span>
                  </label>
                </div>
                <div class="setting-row">
                  <label for="slot-fontsize">名称字号</label>
                  <Select
                    id="slot-fontsize"
                    class="setting-select"
                    :model-value="boundSlot.labelSize ?? 12"
                    :options="fontSizeOptions"
                    @update:model-value="setSlotLabelSize(Number($event))"
                  />
                </div>
                <div class="setting-row">
                  <label for="slot-labelcolor">名称颜色</label>
                  <input
                    type="color"
                    class="color-input"
                    :value="boundSlot.labelColor ?? '#a6aec4'"
                    @change="setSlotLabelColor(($event.target as HTMLInputElement).value)"
                  />
                  <button v-if="boundSlot.labelColor" class="pick-btn" @click="clearSlotLabelColor">
                    恢复默认
                  </button>
                  <div v-if="nameColorHistory.length" class="color-swatches">
                    <button
                      v-for="c in nameColorHistory"
                      :key="'l' + c"
                      class="color-swatch"
                      :style="{ background: c }"
                      :title="c"
                      @click="setSlotLabelColor(c)"
                    ></button>
                  </div>
                </div>
                <div class="setting-row">
                  <label>扇区颜色</label>
                  <input
                    type="color"
                    class="color-input"
                    :value="boundSlot.color ?? '#6d7cff'"
                    @change="setSlotColor(($event.target as HTMLInputElement).value)"
                  />
                  <button v-if="boundSlot.color" class="pick-btn" @click="clearSlotColor">
                    恢复默认
                  </button>
                  <div v-if="sectorColorHistory.length" class="color-swatches">
                    <button
                      v-for="c in sectorColorHistory"
                      :key="'s' + c"
                      class="color-swatch"
                      :style="{ background: c }"
                      :title="c"
                      @click="setSlotColor(c)"
                    ></button>
                  </div>
                </div>
                <p class="bind-hint">外观仅作用于当前扇区；「显示名称」留空时显示配方名。</p>
              </div>
            </div>
          </div>
        </section>
      </div>
    </div>

    <!-- 应用选择弹层 -->
    <div v-if="picker" class="picker-overlay" @click="picker = null">
      <div class="picker-box" @click.stop>
        <div class="picker-title">选择应用</div>
        <div class="picker-search">
          <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="7" /><path d="M21 21l-4.35-4.35" /></svg>
          <input
            v-model="pickerSearch"
            class="config-input"
            placeholder="搜索应用名称或路径…"
            autofocus
          />
        </div>
        <div class="picker-list">
          <div
            v-for="(a, i) in filteredApps"
            :key="i"
            class="picker-item"
            @click="pickApp(a)"
          >
            <span class="picker-name">{{ a.name }}</span>
            <span class="picker-proc">{{ a.exe }}</span>
          </div>
          <div v-if="filteredApps.length === 0" class="picker-empty">
            未找到匹配的应用
          </div>
        </div>
      </div>
    </div>

    <!-- 保存提示弹窗 -->
    <Transition name="toast">
      <div v-if="toast" class="toast" :class="toast.type">
        <svg
          v-if="toast.type === 'success'"
          viewBox="0 0 24 24"
          width="16"
          height="16"
          fill="none"
          stroke="currentColor"
          stroke-width="2.6"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M20 6L9 17l-5-5" />
        </svg>
        <svg
          v-else
          viewBox="0 0 24 24"
          width="16"
          height="16"
          fill="none"
          stroke="currentColor"
          stroke-width="2.6"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M12 8v5M12 17h.01" />
          <circle cx="12" cy="12" r="9" />
        </svg>
        <span>{{ toast.msg }}</span>
      </div>
    </Transition>
  </div>
</template>
