<script setup lang="ts">
import { computed } from "vue";
import ListSelect from "../../ui/ListSelect.vue";
import { useStepTools } from "../../../composables/useStepTools";
import type { EditableClickStep, EditableStep } from "../../../types/editable";

const props = defineProps<{ step: EditableStep }>();
const s = computed(() => props.step as EditableClickStep);

const { coordPicking, startCoordPicking, testClick } = useStepTools();
const isPicking = computed(() => coordPicking.value === props.step);

/** 点击坐标轴基准/单位选项。 */
const BASE_X: { value: "left" | "right"; label: string }[] = [
  { value: "left", label: "距左边" },
  { value: "right", label: "距右边" },
];
const BASE_Y: { value: "top" | "bottom"; label: string }[] = [
  { value: "top", label: "距上边" },
  { value: "bottom", label: "距下边" },
];
const UNIT: { value: "percent" | "px"; label: string }[] = [
  { value: "percent", label: "%" },
  { value: "px", label: "px" },
];
</script>

<template>
  <input
    v-model="s.title"
    class="config-input step-click-title"
    placeholder="目标窗口标题（模糊匹配）"
  />
  <div class="step-axis">
    <span class="step-axis-label">X</span>
    <ListSelect v-model="s.xBase" :options="BASE_X" width="108px" />
    <input
      v-model.number="s.xValue"
      type="number"
      step="0.01"
      min="0"
      class="config-input step-axis-num"
      placeholder="偏移"
    />
    <ListSelect v-model="s.xUnit" :options="UNIT" width="72px" />
  </div>
  <div class="step-axis">
    <span class="step-axis-label">Y</span>
    <ListSelect v-model="s.yBase" :options="BASE_Y" width="108px" />
    <input
      v-model.number="s.yValue"
      type="number"
      step="0.01"
      min="0"
      class="config-input step-axis-num"
      placeholder="偏移"
    />
    <ListSelect v-model="s.yUnit" :options="UNIT" width="72px" />
  </div>
  <button
    class="pick-btn small"
    :class="{ recording: isPicking }"
    @click="startCoordPicking(step)"
  >
    {{ isPicking ? "移到输入框后按 Enter…" : "拾取坐标" }}
  </button>
  <button class="pick-btn small" @click="testClick(step)">测试点击</button>
</template>

<style>
.step-click-title {
  width: 200px;
  flex-shrink: 0;
}

.step-axis {
  display: flex;
  align-items: center;
  gap: 6px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 4px 8px;
  background: rgba(255, 255, 255, 0.03);
}

.step-axis-label {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-3);
  width: 14px;
}

.step-axis-num {
  width: 96px;
  flex-shrink: 0;
}

.pick-btn.recording {
  border-color: var(--accent);
  color: var(--accent);
}
</style>
