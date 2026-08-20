<script setup lang="ts">
import StepList from "./StepList.vue";
import InfoPopover from "../ui/InfoPopover.vue";
import { useConfigState } from "../../composables/useConfigState";
import { useRecipes } from "../../composables/useRecipes";

const { cfg, activeSection, current, selectedIndex, save } = useConfigState();
const { selectRecipe, removeRecipe, duplicateRecipe, addRecipe } = useRecipes();
</script>

<template>
  <section v-show="activeSection === 'recipes'" class="card config-section recipe-editor">
    <div class="recipe-list">
      <div class="recipe-list-title">
        配方
        <span class="recipe-count" :class="{ zero: cfg.recipes.length === 0 }">
          {{ cfg.recipes.length }} 个
        </span>
      </div>
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
        <label class="recipe-confirm-toggle">
          <input type="checkbox" v-model="current.confirm" />
          <span>人工确认</span>
          <InfoPopover
            title="人工确认"
            text="启用后，步骤中勾选了「确认」的步骤会在执行前弹出确认，按 Enter 执行、Esc 取消。"
          />
        </label>
        <button class="btn btn-primary recipe-save" @click="save">保存配方</button>
      </div>
      <StepList :steps="current.steps" />
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
  display: flex;
  align-items: center;
  gap: 8px;
}

.recipe-count {
  margin-left: auto;
  padding: 1px 9px;
  border-radius: var(--radius-pill);
  background: rgba(109, 124, 255, 0.18);
  color: var(--accent);
  font-size: 11px;
  font-weight: 600;
}

.recipe-count.zero {
  background: rgba(245, 158, 11, 0.16);
  color: var(--yellow);
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

.recipe-confirm-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 38px;
  padding: 0 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: rgba(255, 255, 255, 0.03);
  color: var(--text-2);
  font-size: 12.5px;
  font-weight: 600;
  cursor: pointer;
  user-select: none;
  flex-shrink: 0;
  transition: border-color 0.15s, background 0.15s;
}

.recipe-confirm-toggle:hover {
  border-color: rgba(124, 108, 255, 0.5);
  background: rgba(109, 124, 255, 0.08);
}

.recipe-confirm-toggle input {
  accent-color: var(--accent);
  cursor: pointer;
}
</style>
