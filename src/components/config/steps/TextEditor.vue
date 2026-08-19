<script setup lang="ts">
import { computed } from "vue";
import type { EditableStep, EditableTextStep } from "../../../types/editable";

/** setClipboard / typeText / pasteText 共用：单个文本字段。 */
const props = defineProps<{ step: EditableStep }>();
const s = computed(() => props.step as EditableTextStep);
</script>

<template>
  <input v-model="s.text" class="config-input step-text" placeholder="内容" />
</template>

<style>
.step-text {
  width: 300px;
  flex-shrink: 0;
}
</style>
