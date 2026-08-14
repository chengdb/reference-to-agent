<script setup lang="ts">
import StepTypeSelect from "../ui/StepTypeSelect.vue";
import { useConfigStore } from "../../composables/useConfigStore";

const {
  cfg,
  activeSection,
  current,
  selectedIndex,
  selectedStep,
  recording,
  coordPicking,
  selectRecipe,
  removeRecipe,
  duplicateRecipe,
  addRecipe,
  save,
  selectStep,
  addStepAt,
  removeStep,
  moveStep,
  openPicker,
  startRecording,
  startCoordPicking,
  testClick,
} = useConfigStore();
</script>

<template>
  <section v-show="activeSection === 'recipes'" class="card config-section recipe-editor">
    <div class="recipe-list">
      <div class="recipe-list-title">配方</div>
      <div
        v-for="(r, i) in cfg.recipes"
        :key="i"
        class="recipe-item"
        :class="{ active: i === selectedIndex }"
        @click="selectRecipe(i)"
      >
        <span class="recipe-item-name" :title="r.name">{{ r.name }}</span>
        <button class="recipe-copy" title="复制配方" @click.stop="duplicateRecipe(i)">
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="11" height="11" rx="2" /><path d="M5 15V5a2 2 0 0 1 2-2h10" /></svg>
        </button>
        <button class="recipe-del" title="删除配方" @click.stop="removeRecipe(i)">
          ×
        </button>
      </div>
      <button class="recipe-add" @click="addRecipe">+ 添加配方</button>
    </div>

    <div v-if="current" class="recipe-edit">
      <div class="recipe-name-row">
        <input v-model="current.name" class="config-input recipe-name" placeholder="配方名称" />
        <button class="btn btn-primary recipe-save" @click="save">保存配方</button>
      </div>
      <div class="step-list">
        <button class="step-insert" title="在开头插入步骤" @click="addStepAt(0)">
          <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14M5 12h14" /></svg>
        </button>
        <template v-for="(step, si) in current.steps" :key="si">
          <div class="step-row" :class="{ active: selectedStep === si }" @click="selectStep(si)">
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
                  :placeholder="recording === 'step:' + si ? '请按下组合键…' : '录制或手填'"
                />
                <button
                  class="rec-btn small"
                  :class="{ recording: recording === 'step:' + si }"
                  @click="startRecording('step:' + si)"
                >
                  {{ recording === "step:" + si ? "…" : "录制" }}
                </button>
              </template>

              <template v-else-if="step.type === 'activateApp'">
                <input
                  v-model="step.title"
                  class="config-input step-title"
                  placeholder="窗口标题（模糊匹配）"
                />
                <button class="pick-btn" @click="openPicker(si)">选择应用…</button>
                <span v-if="step.exe" class="step-exe-display" :title="step.exe">{{ step.exe }}</span>
              </template>

              <template v-else-if="step.type === 'focusApp'">
                <input
                  v-model="step.title"
                  class="config-input step-title"
                  placeholder="窗口标题（模糊匹配）"
                />
                <button class="pick-btn" @click="openPicker(si)">选择应用…</button>
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
                  <select v-model="step.xBase" class="config-input step-axis-base">
                    <option value="left">距左边</option>
                    <option value="right">距右边</option>
                  </select>
                  <input
                    v-model.number="step.xValue"
                    type="number"
                    step="0.01"
                    min="0"
                    class="config-input step-axis-num"
                    placeholder="偏移"
                  />
                  <select v-model="step.xUnit" class="config-input step-axis-unit">
                    <option value="percent">%</option>
                    <option value="px">px</option>
                  </select>
                </div>
                <div class="step-axis">
                  <span class="step-axis-label">Y</span>
                  <select v-model="step.yBase" class="config-input step-axis-base">
                    <option value="top">距上边</option>
                    <option value="bottom">距下边</option>
                  </select>
                  <input
                    v-model.number="step.yValue"
                    type="number"
                    step="0.01"
                    min="0"
                    class="config-input step-axis-num"
                    placeholder="偏移"
                  />
                  <select v-model="step.yUnit" class="config-input step-axis-unit">
                    <option value="percent">%</option>
                    <option value="px">px</option>
                  </select>
                </div>
                <button
                  class="pick-btn small"
                  :class="{ recording: coordPicking === si }"
                  @click="startCoordPicking(si)"
                >
                  {{ coordPicking === si ? "移到输入框后按 Enter…" : "拾取坐标" }}
                </button>
                <button class="pick-btn small" @click="testClick(si)">测试点击</button>
              </template>

              <template v-else-if="step.type === 'rollbackClipboard'">
                <span class="step-hint">恢复为配方执行前的剪贴板内容</span>
              </template>
            </div>
            <div class="step-controls">
              <button title="上移" :disabled="si === 0" @click="moveStep(si, -1)">
                <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M18 15l-6-6-6 6" /></svg>
              </button>
              <button
                title="下移"
                :disabled="si === current.steps.length - 1"
                @click="moveStep(si, 1)"
              >
                <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M6 9l6 6 6-6" /></svg>
              </button>
              <button class="step-del" title="删除步骤" @click="removeStep(si)">
                <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6L6 18M6 6l12 12" /></svg>
              </button>
            </div>
          </div>
          <button class="step-insert" title="在此处插入步骤" @click="addStepAt(si + 1)">
            <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14M5 12h14" /></svg>
          </button>
        </template>
      </div>
    </div>
    <div v-else class="recipe-edit empty">选择一个配方进行编辑，或点击「+ 添加配方」</div>
  </section>
</template>

<style>
.recipe-editor {
  flex-direction: row;
  align-items: stretch;
  gap: 14px;
  flex: 1;
  min-height: 0;
}

/* ---------- 配方列表 ---------- */

.recipe-list {
  width: 236px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 10px;
  background: rgba(255, 255, 255, 0.03);
  overflow-y: auto;
}

.recipe-list-title {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-3);
  padding: 2px 6px 6px;
  letter-spacing: 0.4px;
}

.recipe-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  padding: 9px 11px;
  border: 1px solid transparent;
  border-radius: 11px;
  cursor: pointer;
  font-size: 15px;
  color: var(--text-2);
  transition: background 0.15s;
}

.recipe-item:hover {
  background: rgba(255, 255, 255, 0.06);
}

/* 条目操作按钮默认隐藏，悬浮条目时淡入显示 */
.recipe-item:not(:hover) .recipe-copy,
.recipe-item:not(:hover) .recipe-del {
  opacity: 0;
  pointer-events: none;
}

.recipe-item:hover .recipe-copy,
.recipe-item:hover .recipe-del {
  opacity: 1;
}

.recipe-item.active {
  background: linear-gradient(135deg, rgba(109, 124, 255, 0.22), rgba(168, 85, 247, 0.18));
  border-color: rgba(124, 108, 255, 0.35);
  color: #fff;
}

.recipe-item-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

.recipe-del {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text-3);
  font-size: 20px;
  cursor: pointer;
  line-height: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background 0.15s, color 0.15s, opacity 0.15s;
}

.recipe-del:hover {
  background: rgba(248, 113, 113, 0.15);
  color: var(--red);
}

.recipe-copy {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text-3);
  font-size: 16px;
  cursor: pointer;
  line-height: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background 0.15s, color 0.15s, opacity 0.15s;
}

.recipe-copy:hover {
  background: rgba(109, 124, 255, 0.15);
  color: var(--accent);
}

.recipe-copy svg {
  display: block;
}

.recipe-add {
  padding: 8px 16px;
  margin-top: 2px;
  border: 1px dashed var(--border-strong);
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--text-2);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  text-align: center;
  transition: background 0.15s, border-color 0.15s;
}

.recipe-add:hover {
  background: rgba(109, 124, 255, 0.1);
  border-color: rgba(124, 108, 255, 0.5);
  color: #fff;
}

/* ---------- 配方编辑区 ---------- */

.recipe-edit {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
}

.recipe-edit.empty {
  align-items: center;
  justify-content: center;
  color: var(--text-3);
  font-size: 13px;
  border: 1.5px dashed var(--border-strong);
  border-radius: 14px;
}

.recipe-name-row {
  display: flex;
  gap: 10px;
  align-items: center;
}

.recipe-name {
  flex: 1;
  font-weight: 600;
  font-size: 14px;
}

.recipe-save {
  flex-shrink: 0;
}

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

.step-axis-base {
  width: 82px;
  flex-shrink: 0;
}

.step-axis-num {
  width: 76px;
  flex-shrink: 0;
}

.step-axis-unit {
  width: 58px;
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
</style>
