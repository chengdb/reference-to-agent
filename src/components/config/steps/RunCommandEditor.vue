<script setup lang="ts">
import { computed } from "vue";
import type { EditableRunCommandStep, EditableStep } from "../../../types/editable";

const props = defineProps<{ step: EditableStep }>();
const s = computed(() => props.step as EditableRunCommandStep);
</script>

<template>
  <input v-model="s.cmd" class="config-input step-cmd" placeholder="命令" />
  <input v-model="s.argsText" class="config-input step-args" placeholder="参数，逗号分隔" />
</template>

<style>
.step-cmd {
  width: 220px;
  flex-shrink: 0;
}

.step-args {
  width: 260px;
  flex-shrink: 0;
}
</style>
