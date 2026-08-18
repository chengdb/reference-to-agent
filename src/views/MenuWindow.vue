<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import Radial from "../components/ui/Radial.vue";
import { buildMenuItems, type MenuItem } from "../utils/menu";
import { invokeWithRetry } from "../utils/invokeRetry";
import type { MenuConfig, Recipe } from "../types";

const win = getCurrentWindow();
const recipes = ref<Recipe[]>([]);
const menu = ref<MenuConfig>({ size: 400, sectors: 8, showLabels: true, slots: [] });

const ordered = computed<(MenuItem | null)[]>(() =>
  buildMenuItems(recipes.value, menu.value.sectors, menu.value.slots)
);
const count = computed(() => ordered.value.filter(Boolean).length);
const extras = computed(() => Math.max(0, recipes.value.length - count.value));
const closeSize = computed(() => 2 * Math.max(36, menu.value.size * 0.145));
const closeIcon = computed(() => Math.round(closeSize.value * 0.2));

async function refresh() {
  // 启动竞态：见 invokeRetry.ts。onMounted 时后端 app.manage() 可能尚未执行，
  // 首调 get_menu 会报 "state not managed"，这里带重试等状态就绪。
  const data = await invokeWithRetry<{ recipes: Recipe[]; menu: MenuConfig }>("get_menu");
  recipes.value = data.recipes;
  menu.value = data.menu;
}

async function run(name: string) {
  try {
    await invoke("run_recipe", { name });
  } catch (e) {
    // 失败原因改为顶部轻提示，不显示在圆盘窗口内。
    invoke("show_warning", { message: String(e) });
  }
}

function onSelect(i: number) {
  const item = ordered.value[i];
  if (item) run(item.recipe.name);
}

function close() {
  invoke("hide_menu_window");
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") close();
  const digit = Number(e.key);
  if (Number.isInteger(digit) && digit >= 1 && digit <= ordered.value.length) {
    const item = ordered.value[digit - 1];
    if (item) run(item.recipe.name);
  }
}

function onBlur() {
  close();
}

let unlisten: (() => void) | undefined;
let unlistenFocus: (() => void) | undefined;
let unlistenResized: (() => void) | undefined;
onMounted(async () => {
  await refresh();
  unlisten = await listen("menu-updated", refresh);
  // 每次窗口显示并获得焦点时重新拉取配置，确保尺寸/绑定与最近一次保存一致。
  unlistenFocus = await win.onFocusChanged(({ payload }) => {
    if (payload) refresh();
  });
  // Rust 调整窗口尺寸后兜底刷新（尺寸变化必然触发 resize）。
  unlistenResized = await win.onResized(() => refresh());
  window.addEventListener("keydown", onKeydown);
  window.addEventListener("blur", onBlur);
});
onUnmounted(() => {
  unlisten?.();
  unlistenFocus?.();
  unlistenResized?.();
  window.removeEventListener("keydown", onKeydown);
  window.removeEventListener("blur", onBlur);
});
</script>

<template>
  <div class="radial" :style="{ width: menu.size + 'px', height: menu.size + 'px' }">
    <Radial
      :items="ordered"
      :size="menu.size"
      :sectors="menu.sectors"
      :show-labels="menu.showLabels"
      interactive
      @select="onSelect"
    />

    <button class="radial-close" title="Esc 关闭" :style="{ width: closeSize + 'px', height: closeSize + 'px' }" @click="close">
      <svg viewBox="0 0 24 24" :width="closeIcon" :height="closeIcon" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6L6 18M6 6l12 12" /></svg>
    </button>

    <div v-if="extras > 0" class="radial-more">
      还有 {{ extras }} 个配方未绑定到菜单扇区
    </div>
  </div>
</template>

<style>
.radial {
  position: fixed;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  user-select: none;
  overflow: hidden;
}

.radial-close {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 100px;
  height: 100px;
  border-radius: 50%;
  border: 1.5px solid rgba(255, 255, 255, 0.45);
  background: rgba(13, 17, 28, 0.5);
  color: #fff;
  box-shadow: 0 4px 18px rgba(2, 6, 23, 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background 0.15s, color 0.15s, transform 0.15s, border-color 0.15s;
}

.radial-close:hover {
  background: rgba(17, 20, 29, 0.92);
  color: var(--text-2);
  border-color: rgba(255, 255, 255, 0.8);
  transform: translate(-50%, -50%) scale(1.05);
}

.radial-more {
  position: absolute;
  bottom: 26px;
  left: 50%;
  transform: translateX(-50%);
  color: var(--text-3);
  font-size: 11px;
  white-space: nowrap;
}
</style>
