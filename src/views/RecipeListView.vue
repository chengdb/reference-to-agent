<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invokeWithRetry } from "../utils/invokeRetry";
import type { Recipe } from "../types";

const win = getCurrentWindow();
const recipes = ref<Recipe[]>([]);
const query = ref("");
const active = ref(0);

/** 过滤后的可见列表：按名称包含匹配。 */
const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return recipes.value;
  return recipes.value.filter((r) => r.name.toLowerCase().includes(q));
});

const count = computed(() => filtered.value.length);

// 筛选结果变化时把选中项收拢回可见范围内。
watch(count, (n) => {
  if (active.value >= n) active.value = Math.max(0, n - 1);
});

async function refresh() {
  // 启动竞态：见 invokeRetry.ts。onMounted 时后端 app.manage() 可能尚未执行，
  // 首调 get_recipes 会报 "state not managed"，这里带重试等状态就绪。
  recipes.value = await invokeWithRetry<Recipe[]>("get_recipes");
  if (active.value >= filtered.value.length) active.value = Math.max(0, filtered.value.length - 1);
}

async function run(name: string) {
  try {
    await invoke("run_recipe", { name });
  } catch (e) {
    // 失败原因改为顶部轻提示，不显示在配方列表窗口内。
    invoke("show_warning", { message: String(e) });
  }
}

function close() {
  invoke("hide_list_window");
}

function move(delta: number) {
  if (filtered.value.length === 0) return;
  active.value = (active.value + delta + filtered.value.length) % filtered.value.length;
}

function runSelected() {
  const item = filtered.value[active.value];
  if (item) run(item.name);
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    close();
    return;
  }
  if (e.key === "ArrowDown") {
    e.preventDefault();
    move(1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    move(-1);
  } else if (e.key === "Enter") {
    runSelected();
  } else {
    const digit = Number(e.key);
    if (Number.isInteger(digit) && digit >= 1 && digit <= 9 && digit <= filtered.value.length) {
      const item = filtered.value[digit - 1];
      if (item) run(item.name);
    }
  }
}

function onBlur() {
  close();
}

let unlisten: (() => void) | undefined;
let unlistenFocus: (() => void) | undefined;
onMounted(async () => {
  // 先挂载监听再拉取数据：即使刷新失败，Esc/失焦关闭也不能失效。
  unlisten = await listen("list-updated", refresh);
  unlistenFocus = await win.onFocusChanged(({ payload }) => {
    if (payload) refresh();
  });
  window.addEventListener("keydown", onKeydown);
  window.addEventListener("blur", onBlur);
  try {
    await refresh();
  } catch (e) {
    invoke("show_warning", { message: String(e) });
  }
});
onUnmounted(() => {
  unlisten?.();
  unlistenFocus?.();
  window.removeEventListener("keydown", onKeydown);
  window.removeEventListener("blur", onBlur);
});
</script>

<template>
  <div class="list-root">
    <div class="list-card">
      <div class="list-head">
        <div class="list-title">
          完整配方
          <span class="list-count" v-if="count > 0">{{ count }}</span>
        </div>
        <input
          v-model="query"
          class="list-search"
          type="text"
          placeholder="筛选配方…"
          spellcheck="false"
          @keydown.stop
          @keydown.up.prevent="move(-1)"
          @keydown.down.prevent="move(1)"
          @keydown.enter.prevent="runSelected"
          @keydown.esc="close"
        />
      </div>

      <div class="list-body">
        <button
          v-for="(r, i) in filtered"
          :key="r.name"
          class="list-item"
          :class="{ active: i === active }"
          @click="run(r.name)"
          @mouseenter="active = i"
        >
          <span class="item-name">{{ r.name }}</span>
          <span class="item-steps">{{ r.steps.length }} 步</span>
        </button>
        <div v-if="count === 0" class="list-empty">没有匹配的配方</div>
      </div>

      <div class="list-hint">点击执行 · ↑↓ 选择 · Enter 运行 · Esc 关闭</div>
    </div>
  </div>
</template>

<style>
.list-root {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 8px;
  user-select: none;
}

.list-card {
  width: 100%;
  max-width: 360px;
  max-height: calc(100vh - 16px);
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border-strong);
  border-radius: 18px;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.05), rgba(255, 255, 255, 0) 48px),
    rgba(13, 17, 28, 0.92);
  overflow: hidden;
}

.list-head {
  padding: 12px 14px 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  border-bottom: 1px solid var(--border);
}

.list-title {
  font-size: 14px;
  font-weight: 700;
  color: var(--text);
  letter-spacing: 0.2px;
}

.list-count {
  margin-left: 6px;
  padding: 1px 8px;
  border-radius: var(--radius-pill);
  background: rgba(109, 124, 255, 0.18);
  color: var(--accent);
  font-size: 11px;
  font-weight: 600;
}

.list-search {
  height: 34px;
  padding: 0 12px;
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.05);
  color: var(--text);
  font-size: 12.5px;
  outline: none;
}

.list-search:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px rgba(109, 124, 255, 0.16);
}

.list-search::placeholder {
  color: var(--text-3);
}

.list-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 6px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.list-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  width: 100%;
  padding: 9px 12px;
  border: 1px solid transparent;
  border-radius: 11px;
  background: transparent;
  color: var(--text);
  font-size: 13px;
  font-weight: 500;
  text-align: left;
  cursor: pointer;
  transition: background 0.12s, border-color 0.12s, transform 0.12s;
}

.list-item:hover {
  background: rgba(255, 255, 255, 0.07);
}

.list-item:active {
  transform: scale(0.99);
}

.list-item.active {
  background: linear-gradient(135deg, rgba(109, 124, 255, 0.24), rgba(168, 85, 247, 0.18));
  border-color: rgba(109, 124, 255, 0.45);
}

.item-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.item-steps {
  flex-shrink: 0;
  font-size: 11px;
  color: var(--text-3);
}

.list-empty {
  padding: 26px 0;
  text-align: center;
  color: var(--text-3);
  font-size: 12.5px;
}

.list-hint {
  padding: 8px 14px;
  border-top: 1px solid var(--border);
  color: var(--text-3);
  font-size: 11px;
  text-align: center;
  white-space: nowrap;
}
</style>