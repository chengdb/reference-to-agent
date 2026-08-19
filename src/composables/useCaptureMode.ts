import { ref } from "vue";
import type { EditableStep } from "../types/editable";

/**
 * 输入捕获模式（热键录制 / 坐标拾取）的共享状态。
 * 两种模式互斥：各自的操作函数在开启时清空对方（见
 * useHotkeyRecording / useStepTools）。单独抽成模块是为了避免
 * 两个 composable 之间循环依赖。
 */

/** 热键录制目标：'global'（圆盘）、'list'（完整配方列表）或 'step'（配合 recordingStep）。 */
const recording = ref<"global" | "list" | "step" | null>(null);
/** 正在录制热键的步骤（recording === 'step' 时有效）。 */
const recordingStep = ref<EditableStep | null>(null);
/** 正在拾取点击坐标的步骤（对象引用）。 */
const coordPicking = ref<EditableStep | null>(null);

function clearRecording() {
  recording.value = null;
  recordingStep.value = null;
}

function clearCoordPicking() {
  coordPicking.value = null;
}

export function useCaptureMode() {
  return {
    recording,
    recordingStep,
    coordPicking,
    clearRecording,
    clearCoordPicking,
  };
}
