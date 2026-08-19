<script setup lang="ts">
import { computed } from "vue";
import { useAppPicker } from "../../../composables/useAppPicker";
import { useStepTools } from "../../../composables/useStepTools";
import type { EditableAppTargetStep, EditableStep } from "../../../types/editable";

/** activateApp / focusApp 共用：窗口标题 + 应用选择 + 标题查询。 */
const props = defineProps<{ step: EditableStep }>();
const s = computed(() => props.step as EditableAppTargetStep);

const { openPicker } = useAppPicker();
const { getWindowInfo } = useStepTools();
</script>

<template>
  <input
    v-model="s.title"
    class="config-input step-title"
    placeholder="窗口标题（模糊匹配）"
  />
  <button class="pick-btn" @click="openPicker(step)">选择应用…</button>
  <button class="pick-btn small" @click="getWindowInfo(step)">查询标题</button>
  <span v-if="s.exe" class="step-exe-display" :title="s.exe">{{ s.exe }}</span>
</template>

<style>
.step-title {
  width: 260px;
  flex-shrink: 0;
}

.step-exe-display {
  font-size: 12px;
  color: var(--text-3);
  max-width: 280px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
