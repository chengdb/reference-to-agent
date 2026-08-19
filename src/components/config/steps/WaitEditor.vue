<script setup lang="ts">
import { computed } from "vue";
import type { EditableStep, EditableWaitStep } from "../../../types/editable";

const props = defineProps<{ step: EditableStep }>();
const s = computed(() => props.step as EditableWaitStep);
</script>

<template>
  <input v-model.number="s.ms" type="number" min="0" class="config-input step-ms" placeholder="毫秒" />
  <span class="step-unit">ms</span>
</template>

<style>
.step-ms {
  width: 92px;
}

.step-unit {
  color: var(--text-3);
  font-size: 12px;
}
</style>
