<script setup lang="ts">
import { computed } from "vue";
import { useHotkeyRecording } from "../../../composables/useHotkeyRecording";
import type { EditableHotkeyStep, EditableStep } from "../../../types/editable";

const props = defineProps<{ step: EditableStep }>();
const s = computed(() => props.step as EditableHotkeyStep);

const { recording, recordingStep, startRecording } = useHotkeyRecording();
const isRecording = computed(
  () => recording.value === "step" && recordingStep.value === props.step
);
</script>

<template>
  <input
    class="config-input step-keys"
    :value="s.keys"
    readonly
    :placeholder="isRecording ? '请按下组合键…' : '录制或手填'"
  />
  <button class="rec-btn small" :class="{ recording: isRecording }" @click="startRecording(step)">
    {{ isRecording ? "…" : "录制" }}
  </button>
</template>

<style>
.step-keys {
  width: 220px;
  flex-shrink: 0;
  font-family: "Cascadia Code", Consolas, monospace;
}

.rec-btn.recording {
  border-color: var(--accent);
  color: var(--accent);
}
</style>
