<script setup lang="ts">
import type { Component } from "vue";
import StepTypeSelect from "../ui/StepTypeSelect.vue";
import StepInfoPopover from "./StepInfoPopover.vue";
import WaitEditor from "./steps/WaitEditor.vue";
import HotkeyEditor from "./steps/HotkeyEditor.vue";
import AppTargetEditor from "./steps/AppTargetEditor.vue";
import TextEditor from "./steps/TextEditor.vue";
import RunCommandEditor from "./steps/RunCommandEditor.vue";
import ClickEditor from "./steps/ClickEditor.vue";
import IfEditor from "./steps/IfEditor.vue";
import RollbackEditor from "./steps/RollbackEditor.vue";
import { useConfigState } from "../../composables/useConfigState";
import { useRecipes } from "../../composables/useRecipes";
import { normalizeStepFields } from "../../utils/steps";
import type { StepType } from "../../types";
import type { EditableStep } from "../../types/editable";

defineProps<{ steps: EditableStep[] }>();

/** 步骤类型 → 参数编辑器组件。新增步骤类型时在 steps/ 下新建编辑器并登记到此表。 */
const EDITORS: Record<StepType, Component> = {
  wait: WaitEditor,
  hotkey: HotkeyEditor,
  activateApp: AppTargetEditor,
  focusApp: AppTargetEditor,
  setClipboard: TextEditor,
  typeText: TextEditor,
  pasteText: TextEditor,
  runCommand: RunCommandEditor,
  click: ClickEditor,
  if: IfEditor,
  rollbackClipboard: RollbackEditor,
};

const { selectedStep, selectStep } = useConfigState();
const { addStepAt, removeStep, moveStep } = useRecipes();

/** 切换步骤类型：保留可复用字段，并补齐新类型的默认字段。 */
function onTypeChange(step: EditableStep, t: StepType) {
  if (step.type === t) return;
  (step as { type: StepType }).type = t;
  normalizeStepFields([step]);
}
</script>

<template>
  <div class="step-list">
    <button class="step-insert" title="在开头插入步骤" @click="addStepAt(steps, 0)">
      <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14M5 12h14" /></svg>
    </button>
    <template v-for="(step, si) in steps" :key="si">
      <div class="step-row" :class="{ active: selectedStep === step }" @click="selectStep(step)">
        <span class="step-index">{{ si + 1 }}</span>
        <div class="step-body">
          <StepTypeSelect
            :model-value="step.type"
            class="step-type"
            @update:model-value="onTypeChange(step, $event)"
          />
          <component :is="EDITORS[step.type]" :step="step" />
        </div>
        <div class="step-confirm-group" v-if="step.type !== 'wait'">
          <label
            class="step-confirm-toggle"
            :class="{ checked: step.confirm }"
            title="勾选后，该步骤执行前会弹出确认（需配方已启用「人工确认」，Enter 执行 / Esc 取消）"
          >
            <input type="checkbox" v-model="step.confirm" />
            <span>确认</span>
          </label>
        </div>
        <div class="step-controls">
          <StepInfoPopover :step-type="step.type" />
          <button title="上移" :disabled="si === 0" @click="moveStep(steps, si, -1)">
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M18 15l-6-6-6 6" /></svg>
          </button>
          <button title="下移" :disabled="si === steps.length - 1" @click="moveStep(steps, si, 1)">
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M6 9l6 6 6-6" /></svg>
          </button>
          <button class="step-del" title="删除步骤" @click.stop="removeStep(steps, si)">
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6L6 18M6 6l12 12" /></svg>
          </button>
        </div>
      </div>
      <button class="step-insert" title="在此处插入步骤" @click="addStepAt(steps, si + 1)">
        <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14M5 12h14" /></svg>
      </button>
    </template>
  </div>
</template>

<style>
/* ---------- 步骤列表 ---------- */

.step-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  overflow-y: auto;
  flex: 1;
  padding-right: 2px;
}

.step-insert {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  width: 100%;
  padding: 2px;
  border: 1px dashed rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  background: transparent;
  color: var(--text-3);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  opacity: 0.55;
  transition: background 0.15s, border-color 0.15s, color 0.15s, opacity 0.15s;
}

.step-insert:hover {
  opacity: 1;
  background: rgba(109, 124, 255, 0.1);
  border-color: rgba(124, 108, 255, 0.5);
  color: #fff;
}

.step-insert svg {
  flex-shrink: 0;
}

.step-row {
  display: flex;
  gap: 10px;
  align-items: center;
  padding: 12px 14px;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.025);
  transition: border-color 0.15s, background 0.15s;
}

.step-row:hover {
  border-color: var(--border-strong);
  background: rgba(255, 255, 255, 0.04);
}

.step-row.active {
  border-color: rgba(124, 108, 255, 0.45);
  background: linear-gradient(135deg, rgba(109, 124, 255, 0.1), rgba(168, 85, 247, 0.07));
}

.step-row.active .step-index {
  background: var(--accent-grad);
  border-color: transparent;
  color: #fff;
  box-shadow: 0 3px 8px rgba(124, 108, 255, 0.4);
}

.step-index {
  width: 24px;
  height: 24px;
  border-radius: 8px;
  background: rgba(109, 124, 255, 0.14);
  border: 1px solid rgba(124, 108, 255, 0.32);
  color: #a5b4fc;
  font-size: 11px;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.step-confirm-toggle {
  display: flex;
  align-items: center;
  gap: 4px;
  height: 30px;
  padding: 0 9px;
  border-radius: 8px;
  background: transparent;
  color: var(--text-3);
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  user-select: none;
  flex-shrink: 0;
  transition: background 0.15s, color 0.15s;
}

.step-confirm-toggle:hover {
  background: rgba(255, 255, 255, 0.14);
  color: var(--text-2);
}

.step-confirm-toggle.checked {
  color: var(--yellow);
}

.step-confirm-toggle input {
  accent-color: var(--accent);
  cursor: pointer;
  margin: 0;
}

.step-confirm-group {
  display: flex;
  align-items: center;
  flex-shrink: 0;
  margin-left: auto;
  align-self: center;
  padding: 3px;
  border: 1px solid var(--border);
  border-radius: 11px;
  background: rgba(255, 255, 255, 0.04);
}

/* 确认组存在时，右对齐由它负责，操作组紧跟其后。 */
.step-confirm-group + .step-controls {
  margin-left: 0;
}

.step-controls {
  display: flex;
  flex-direction: row;
  gap: 2px;
  flex-shrink: 0;
  margin-left: auto;
  align-self: center;
  padding: 3px;
  border: 1px solid var(--border);
  border-radius: 11px;
  background: rgba(255, 255, 255, 0.04);
}

.step-controls button {
  width: 32px;
  height: 30px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text-2);
  cursor: pointer;
  line-height: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s, color 0.15s;
}

.step-controls button svg {
  display: block;
}

.step-controls button:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.14);
}

.step-controls button.step-del:hover {
  background: rgba(248, 113, 113, 0.18);
  color: var(--red);
}

.step-controls button:disabled {
  opacity: 0.3;
  cursor: default;
}

.step-body {
  display: flex;
  gap: 8px;
  align-items: center;
  flex: 1;
  min-width: 0;
  flex-wrap: wrap;
  margin-right: 10px;
}

.step-type {
  width: 158px;
  flex-shrink: 0;
}

/* 各步骤编辑器共用小按钮样式（.pick-btn 基础样式见 styles/style.css）。 */
.pick-btn.small {
  height: 34px;
  padding: 0 10px;
  font-size: 12px;
}
</style>
