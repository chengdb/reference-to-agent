import { useCaptureMode } from "./useCaptureMode";
import { useConfigState } from "./useConfigState";
import type { EditableStep } from "../types/editable";

const { cfg } = useConfigState();
const { recording, recordingStep, clearCoordPicking } = useCaptureMode();

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

/** 开启热键录制：target 传 'global'/'list' 录制对应全局快捷键，传步骤对象录制该步骤的 keys。 */
function startRecording(target: "global" | "list" | EditableStep) {
  const currently = recording.value;
  if (target === "global" || target === "list") {
    recording.value = currently === target ? null : target;
    recordingStep.value = null;
  } else {
    const isSame = recording.value === "step" && recordingStep.value === target;
    recording.value = isSame ? null : "step";
    recordingStep.value = isSame ? null : target;
  }
  // 录制与坐标拾取互斥（见 startCoordPicking）。
  if (recording.value != null) clearCoordPicking();
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
  } else if (recording.value === "list") {
    cfg.listHotkey = combo;
  } else if (recording.value === "step") {
    const step = recordingStep.value;
    if (step && step.type === "hotkey") step.keys = combo;
  }
  recording.value = null;
  recordingStep.value = null;
}

/** 热键录制：全局快捷键与 hotkey 步骤的按键捕获。 */
export function useHotkeyRecording() {
  return {
    recording,
    recordingStep,
    startRecording,
    onKeydown,
  };
}
