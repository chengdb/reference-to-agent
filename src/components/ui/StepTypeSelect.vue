<script setup lang="ts">
import { useDropdown } from "../../composables/useDropdown";
import type { Step } from "../../types";

type StepType = Step["type"];

const props = defineProps<{ modelValue: StepType }>();
const emit = defineEmits<{ "update:modelValue": [StepType] }>();

const { open, trigger, panel, pos, toggle, close } = useDropdown();

const options: { type: StepType; label: string }[] = [
  { type: "wait", label: "等待" },
  { type: "hotkey", label: "快捷键" },
  { type: "activateApp", label: "激活应用" },
  { type: "focusApp", label: "聚焦应用" },
  { type: "setClipboard", label: "写剪贴板" },
  { type: "typeText", label: "输入文本(逐字)" },
  { type: "pasteText", label: "输入文本(粘贴)" },
  { type: "runCommand", label: "运行命令" },
  { type: "click", label: "点击坐标" },
  { type: "if", label: "条件判断" },
  { type: "rollbackClipboard", label: "剪切板回滚" },
];

const current = () => options.find((o) => o.type === props.modelValue);

function pick(t: StepType) {
  emit("update:modelValue", t);
  close();
}
</script>

<template>
  <div class="sselect">
    <button
      ref="trigger"
      type="button"
      class="sselect-trigger"
      :class="{ open }"
      @click="toggle"
    >
      <span class="step-dot" :class="'t-' + modelValue"></span>
      <span class="sselect-value">{{ current()?.label }}</span>
      <svg
        class="sselect-chevron"
        :class="{ up: open }"
        viewBox="0 0 24 24"
        width="14"
        height="14"
        fill="none"
        stroke="currentColor"
        stroke-width="2.4"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M6 9l6 6 6-6" />
      </svg>
    </button>
    <Teleport to="body">
      <Transition name="sselect">
        <div v-if="open" ref="panel" class="sselect-pop" :style="pos">
          <button
            v-for="o in options"
            :key="o.type"
            type="button"
            class="sselect-option"
            :class="{ selected: o.type === modelValue }"
            @click="pick(o.type)"
          >
            <span class="step-dot" :class="'t-' + o.type"></span>
            <span>{{ o.label }}</span>
            <svg
              v-if="o.type === modelValue"
              class="sselect-check"
              viewBox="0 0 24 24"
              width="14"
              height="14"
              fill="none"
              stroke="currentColor"
              stroke-width="2.4"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M20 6L9 17l-5-5" />
            </svg>
          </button>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style>
.sselect {
  position: relative;
  flex-shrink: 0;
}

.sselect-trigger {
  height: 40px;
  width: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  background: #0e1119;
  color: var(--text);
  font-size: 13px;
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.sselect-trigger:hover {
  border-color: var(--accent);
}

.sselect-trigger.open {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px rgba(109, 124, 255, 0.18);
}

.sselect-value {
  flex: 1;
  text-align: left;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sselect-chevron {
  color: var(--text-3);
  flex-shrink: 0;
  transition: transform 0.15s;
}

.sselect-chevron.up {
  transform: rotate(180deg);
}

.sselect-pop {
  position: fixed;
  z-index: 120;
  padding: 6px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  background: linear-gradient(180deg, #1b2030, #141823);
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
}

.sselect-option {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 10px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text-2);
  font-size: 13px;
  text-align: left;
  cursor: pointer;
  transition: background 0.12s;
}

.sselect-option:hover {
  background: rgba(255, 255, 255, 0.07);
}

.sselect-option.selected {
  background: linear-gradient(135deg, rgba(109, 124, 255, 0.2), rgba(168, 85, 247, 0.15));
  color: #fff;
}

.sselect-check {
  margin-left: auto;
  color: var(--accent);
}

.sselect-enter-active,
.sselect-leave-active {
  transition: opacity 0.12s, transform 0.12s;
}

.sselect-enter-from,
.sselect-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

.step-dot {
  width: 8px;
  height: 8px;
  border-radius: var(--radius-pill);
  flex-shrink: 0;
}

.t-wait {
  background: #64748b;
}

.t-hotkey {
  background: #f59e0b;
}

.t-activateApp {
  background: #6d7cff;
}

.t-focusApp {
  background: #22d3ee;
}

.t-setClipboard {
  background: #a855f7;
}

.t-typeText {
  background: #f472b6;
}

.t-pasteText {
  background: #ec4899;
}

.t-runCommand {
  background: #34d399;
}

.t-click {
  background: #60a5fa;
}

.t-if {
  background: #c084fc;
}

.t-rollbackClipboard {
  background: #fbbf24;
}
</style>
