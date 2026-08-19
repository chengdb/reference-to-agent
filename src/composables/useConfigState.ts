import { computed, reactive, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { invokeWithRetry } from "../utils/invokeRetry";
import { normalizeSlots } from "../utils/menu";
import { normalizeStepFields, toEditable, toStep } from "../utils/steps";
import { useToast } from "./useToast";
import type { Config, MenuConfig } from "../types";
import type { EditableRecipe, EditableStep } from "../types/editable";

export type Section = "basic" | "recipes" | "menu";

const DEFAULT_MENU: MenuConfig = {
  size: 400,
  sectors: 8,
  showLabels: true,
  slots: [{ recipe: 0 }, { recipe: 1 }],
};

/** 编辑中的整体配置（单一共享响应式对象，各域 composable 围绕它工作）。 */
const cfg = reactive<{
  globalHotkey: string;
  listHotkey: string;
  recipes: EditableRecipe[];
  menu: MenuConfig;
}>({
  globalHotkey: "",
  listHotkey: "",
  recipes: [],
  menu: { ...DEFAULT_MENU, slots: DEFAULT_MENU.slots.map((s) => ({ ...s })) },
});

const selectedIndex = ref(0);
const activeSection = ref<Section>("menu");
/** 当前选中的步骤（对象引用，支持嵌套分支）。 */
const selectedStep = ref<EditableStep | null>(null);
/** 最近的加载失败信息（非空时在配置页顶部以红色条显示，便于定位“配置空/全空”问题）。 */
const loadError = ref<string | null>(null);

const current = computed(() => cfg.recipes[selectedIndex.value]);

function selectStep(step: EditableStep | null) {
  selectedStep.value = step;
}

function setSection(s: Section) {
  activeSection.value = s;
  selectedStep.value = null;
}

function buildConfig(): Config {
  return {
    globalHotkey: cfg.globalHotkey.trim() || "Ctrl+Alt+R",
    listHotkey: cfg.listHotkey.trim() || "Ctrl+Alt+L",
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
      confirm: r.confirm === true,
      steps: r.steps.map(toStep),
    })),
  };
}

async function load() {
  const { showToast } = useToast();
  try {
    // 见 invokeRetry.ts：启动时窗口 JS 可能早于后端 app.manage() 执行，首调 get_config
    // 会报 "state not managed"。这里用带重试的调用等状态就绪；重试耗尽则向上抛，
    // 由下方 catch 记录 loadError 并提示，避免静默变成空配置。
    const c = await invokeWithRetry<Config>("get_config");
    // 后端 load 保证始终返回有效配置（损坏时自动从备份恢复或回退默认），
    // 前端不再自带默认配置副本，避免双源漂移。
    cfg.globalHotkey = c.globalHotkey;
    cfg.listHotkey = c.listHotkey ?? "Ctrl+Alt+L";
    cfg.menu = { ...DEFAULT_MENU, ...(c.menu ?? {}) };
    cfg.menu.slots = normalizeSlots(cfg.menu.slots, cfg.menu.sectors);
    // 逐个配方映射：单个配方/步骤格式异常时只跳过该配方并记录错误，
    // 不能因为一个坏项就让整个配置加载失败、界面变成空列表“像新软件”。
    const mapped: EditableRecipe[] = [];
    for (const r of c.recipes ?? []) {
      try {
        mapped.push({
          name: r.name,
          confirm: r.confirm === true,
          steps: (r.steps ?? []).map(toEditable),
        });
      } catch (e) {
        console.error("[load] skip malformed recipe:", r?.name, e);
      }
    }
    // 补齐 click/if 默认字段（旧配置可能缺 x/y 或分支字段）。
    for (const r of mapped) normalizeStepFields(r.steps);
    cfg.recipes = mapped;
    if (cfg.recipes.length > 0) selectedIndex.value = 0;
    selectedStep.value = null;
    loadError.value = null;
  } catch (e) {
    console.error("[load] FAILED:", e);
    loadError.value = String(e);
    // 加载失败时保留内存中的现有数据，不清空，并给出可见错误提示，
    // 避免出现“配置看起来全丢了/空列表”又默默写回覆盖磁盘配置。
    showToast("读取配置失败：" + String(e), "error");
  }
}

async function save() {
  const { showToast } = useToast();
  try {
    await invoke("save_config", { cfg: buildConfig() });
    showToast("已保存并应用");
  } catch (e) {
    showToast(String(e), "error");
  }
}

/**
 * 核心配置状态：cfg 对象、加载/保存、配方/步骤/面板的选择状态。
 * 其余领域逻辑见 useRecipes / useMenuSlots / useHotkeyRecording /
 * useStepTools / useAppPicker / useToast。
 */
export function useConfigState() {
  return {
    cfg,
    selectedIndex,
    activeSection,
    selectedStep,
    loadError,
    current,
    load,
    save,
    selectStep,
    setSection,
  };
}
