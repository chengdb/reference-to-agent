<script setup lang="ts">
import { computed } from "vue";
import StepTypeSelect from "../ui/StepTypeSelect.vue";
import ListSelect from "../ui/ListSelect.vue";
import StepInfoPopover from "./StepInfoPopover.vue";
import VarInsertSelect from "./VarInsertSelect.vue";
import { useConfigStore, type EditableStep, type StepType } from "../../composables/useConfigStore";
import type { CompareOp } from "../../types";

defineProps<{ steps: EditableStep[] }>();

const {
  current,
  selectedStep,
  recording,
  recordingStep,
  coordPicking,
  selectStep,
  addStepAt,
  removeStep,
  moveStep,
  openPicker,
  getWindowInfo,
  startRecording,
  startCoordPicking,
  testClick,
} = useConfigStore();

/** 哪些步骤类型会产出哪些变量名（供条件判断下拉选择）。 */
const VAR_PRODUCERS: Partial<Record<StepType, string[]>> = {
  focusApp: ["title"],
  activateApp: ["title"],
};

/** 递归收集步骤树中会产出的变量名。 */
function collectVarNames(steps: EditableStep[], out: Set<string>) {
  for (const s of steps) {
    const produced = VAR_PRODUCERS[s.type];
    if (produced) for (const n of produced) out.add(n);
    if (s.type === "if") {
      collectVarNames(s.then ?? [], out);
      for (const b of s.elseIf ?? []) collectVarNames(b.then ?? [], out);
      collectVarNames(s.else ?? [], out);
    }
  }
}

/** 当前配方整棵步骤树中可用的变量名（变量为配方级单值，故扫描全树）。 */
const availableVars = computed(() => {
  const out = new Set<string>();
  collectVarNames(current.value?.steps ?? [], out);
  return [...out];
});

/** 比较操作符选项。 */
const OP_OPTIONS: { value: CompareOp; label: string }[] = [
  { value: "eq", label: "等于 ==" },
  { value: "ne", label: "不等于 !=" },
  { value: "gt", label: "大于 >" },
  { value: "ge", label: "大于等于 >=" },
  { value: "lt", label: "小于 <" },
  { value: "le", label: "小于等于 <=" },
  { value: "startsWith", label: "前缀 startsWith" },
  { value: "endsWith", label: "后缀 endsWith" },
  { value: "contains", label: "包含 contains" },
  { value: "matches", label: "正则匹配 matches" },
];

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

/** 添加一个 else-if 分支。 */
function addElseIf(step: EditableStep) {
  const b = {
    op: "eq" as CompareOp,
    value: "${title}",
    expected: "",
    then: [] as EditableStep[],
  };
  step.elseIf = step.elseIf ?? [];
  step.elseIf.push(b);
}

function removeElseIf(step: EditableStep, i: number) {
  if (!step.elseIf) return;
  step.elseIf.splice(i, 1);
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
          <StepTypeSelect v-model="step.type" class="step-type" />

          <template v-if="step.type === 'wait'">
            <input v-model.number="step.ms" type="number" min="0" class="config-input step-ms" placeholder="毫秒" />
            <span class="step-unit">ms</span>
          </template>

          <template v-else-if="step.type === 'hotkey'">
            <input
              class="config-input step-keys"
              :value="step.keys"
              readonly
              :placeholder="recording === 'step' && recordingStep === step ? '请按下组合键…' : '录制或手填'"
            />
            <button
              class="rec-btn small"
              :class="{ recording: recording === 'step' && recordingStep === step }"
              @click="startRecording(step)"
            >
              {{ recording === "step" && recordingStep === step ? "…" : "录制" }}
            </button>
          </template>

          <template v-else-if="step.type === 'activateApp'">
            <input
              v-model="step.title"
              class="config-input step-title"
              placeholder="窗口标题（模糊匹配）"
            />
            <button class="pick-btn" @click="openPicker(step)">选择应用…</button>
            <button class="pick-btn small" @click="getWindowInfo(step)">查询标题</button>
            <span v-if="step.exe" class="step-exe-display" :title="step.exe">{{ step.exe }}</span>
          </template>

          <template v-else-if="step.type === 'focusApp'">
            <input
              v-model="step.title"
              class="config-input step-title"
              placeholder="窗口标题（模糊匹配）"
            />
            <button class="pick-btn" @click="openPicker(step)">选择应用…</button>
            <button class="pick-btn small" @click="getWindowInfo(step)">查询标题</button>
            <span v-if="step.exe" class="step-exe-display" :title="step.exe">{{ step.exe }}</span>
          </template>

          <template v-else-if="step.type === 'setClipboard' || step.type === 'typeText' || step.type === 'pasteText'">
            <input v-model="step.text" class="config-input step-text" placeholder="内容" />
          </template>

          <template v-else-if="step.type === 'runCommand'">
            <input v-model="step.cmd" class="config-input step-cmd" placeholder="命令" />
            <input v-model="step.argsText" class="config-input step-args" placeholder="参数，逗号分隔" />
          </template>

          <template v-else-if="step.type === 'click'">
            <input
              v-model="step.title"
              class="config-input step-click-title"
              placeholder="目标窗口标题（模糊匹配）"
            />
            <div class="step-axis">
              <span class="step-axis-label">X</span>
              <ListSelect v-model="step.xBase" :options="BASE_X" width="108px" />
              <input
                v-model.number="step.xValue"
                type="number"
                step="0.01"
                min="0"
                class="config-input step-axis-num"
                placeholder="偏移"
              />
              <ListSelect v-model="step.xUnit" :options="UNIT" width="72px" />
            </div>
            <div class="step-axis">
              <span class="step-axis-label">Y</span>
              <ListSelect v-model="step.yBase" :options="BASE_Y" width="108px" />
              <input
                v-model.number="step.yValue"
                type="number"
                step="0.01"
                min="0"
                class="config-input step-axis-num"
                placeholder="偏移"
              />
              <ListSelect v-model="step.yUnit" :options="UNIT" width="72px" />
            </div>
            <button
              class="pick-btn small"
              :class="{ recording: coordPicking === step }"
              @click="startCoordPicking(step)"
            >
              {{ coordPicking === step ? "移到输入框后按 Enter…" : "拾取坐标" }}
            </button>
            <button class="pick-btn small" @click="testClick(step)">测试点击</button>
          </template>

          <template v-else-if="step.type === 'if'">
            <div class="step-if">
              <div class="step-if-head">
                <span class="step-if-kw">if</span>
                <ListSelect
                  v-model="step.op"
                  :options="OP_OPTIONS"
                  width="150px"
                />
                <VarInsertSelect :available="availableVars" @select="step.value = $event" />
                <input v-model="step.value" class="config-input step-if-value" placeholder="${title}" />
                <VarInsertSelect :available="availableVars" @select="step.expected = $event" />
                <input v-model="step.expected" class="config-input step-if-expected" placeholder="期望值" />
                <button class="pick-btn small" @click="addElseIf(step)">+ else if</button>
              </div>

              <div class="step-branch">
                <div class="step-branch-label">then</div>
                <StepList :steps="step.then ?? []" />
              </div>

              <template v-for="(b, bi) in step.elseIf ?? []" :key="'ei' + bi">
                <div class="step-branch">
                  <div class="step-branch-label">
                    else if
                    <ListSelect
                      v-model="b.op"
                      :options="OP_OPTIONS"
                      width="150px"
                    />
                    <VarInsertSelect :available="availableVars" @select="b.value = $event" />
                    <input v-model="b.value" class="config-input step-if-value" placeholder="${title}" />
                    <VarInsertSelect :available="availableVars" @select="b.expected = $event" />
                    <input v-model="b.expected" class="config-input step-if-expected" placeholder="期望值" />
                    <button class="step-branch-del" title="删除此分支" @click="removeElseIf(step, bi)">×</button>
                  </div>
                  <StepList :steps="b.then ?? []" />
                </div>
              </template>

              <div class="step-branch">
                <div class="step-branch-label">else</div>
                <StepList :steps="step.else ?? []" />
              </div>
            </div>
          </template>

          <template v-else-if="step.type === 'rollbackClipboard'">
            <span class="step-hint">恢复为配方执行前的剪贴板内容</span>
          </template>
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

.step-ms {
  width: 92px;
}

.step-unit {
  color: var(--text-3);
  font-size: 12px;
}

.step-keys {
  width: 220px;
  flex-shrink: 0;
  font-family: "Cascadia Code", Consolas, monospace;
}

.step-text {
  width: 300px;
  flex-shrink: 0;
}

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

.step-cmd {
  width: 220px;
  flex-shrink: 0;
}

.step-args {
  width: 260px;
  flex-shrink: 0;
}

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

.pick-btn.small {
  height: 34px;
  padding: 0 10px;
  font-size: 12px;
}

.rec-btn.recording,
.pick-btn.recording {
  border-color: var(--accent);
  color: var(--accent);
}

.step-hint {
  color: var(--text-3);
  font-size: 12px;
}

/* ---------- 条件分叉 ---------- */

.step-if {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
  border: 1px solid rgba(192, 132, 252, 0.35);
  border-radius: 10px;
  padding: 10px;
  background: rgba(192, 132, 252, 0.05);
}

.step-if-head {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.step-if-kw {
  font-family: "Cascadia Code", Consolas, monospace;
  font-weight: 700;
  color: #c084fc;
  font-size: 13px;
}

.step-if-value {
  width: 160px;
  font-family: "Cascadia Code", Consolas, monospace;
}

.step-if-expected {
  width: 160px;
  font-family: "Cascadia Code", Consolas, monospace;
}

.step-branch {
  border-left: 2px solid rgba(192, 132, 252, 0.3);
  padding-left: 10px;
  margin-left: 4px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.step-branch-label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-family: "Cascadia Code", Consolas, monospace;
  font-size: 12px;
  font-weight: 700;
  color: #c084fc;
}

.step-branch-del {
  width: 22px;
  height: 22px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-3);
  cursor: pointer;
  line-height: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s, color 0.15s;
}

.step-branch-del:hover {
  background: rgba(248, 113, 113, 0.18);
  color: var(--red);
}
</style>
