import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "./useToast";
import type { AppEntry } from "../types";
import type { EditableStep } from "../types/editable";

const { showToast } = useToast();

const picker = ref<{ step: EditableStep | null; list: AppEntry[] } | null>(null);
const pickerSearch = ref("");

const filteredApps = computed(() => {
  if (!picker.value) return [];
  const q = pickerSearch.value.trim().toLowerCase();
  if (!q) return picker.value.list;
  return picker.value.list.filter(
    (a) => a.name.toLowerCase().includes(q) || a.exe.toLowerCase().includes(q)
  );
});

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

/** 「选择应用…」弹层：列出本机已安装应用并回填到聚焦/激活步骤。 */
export function useAppPicker() {
  return {
    picker,
    pickerSearch,
    filteredApps,
    openPicker,
    closePicker,
    pickApp,
  };
}
