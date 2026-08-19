import { invoke } from "@tauri-apps/api/core";
import { useCaptureMode } from "./useCaptureMode";
import { useToast } from "./useToast";
import { clickAxisPos } from "../utils/steps";
import type { AxisPos } from "../types";
import type { EditableClickStep, EditableStep } from "../types/editable";

const { coordPicking, clearRecording } = useCaptureMode();
const { showToast } = useToast();

/**
 * 开启坐标拾取：让用户把鼠标移到目标窗口输入框上并停留，然后按下 Enter。
 * 后端记录鼠标位置并换算为「相对目标窗口」的百分比坐标，回填到当前 click 步骤。
 */
function startCoordPicking(step: EditableStep) {
  coordPicking.value = coordPicking.value === step ? null : step;
  // 拾取与热键录制互斥：避免 Enter 同时触发坐标拾取与热键录制。
  if (coordPicking.value != null) clearRecording();
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

async function pickClickCoords(step: EditableClickStep) {
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

/** 步骤调试工具：点击坐标拾取、测试点击、窗口标题查询。 */
export function useStepTools() {
  return {
    coordPicking,
    startCoordPicking,
    onCoordPickKeydown,
    testClick,
    getWindowInfo,
  };
}
