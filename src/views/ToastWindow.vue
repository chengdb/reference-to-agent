<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** 统一的警告/轻提示：独立小窗口，屏幕上方居中，无阴影，3 秒后由后端自动隐藏。 */
const message = ref("");

let unlisten: UnlistenFn | undefined;

onMounted(async () => {
  unlisten = await listen<string>("toast-message", (e) => {
    message.value = e.payload;
  });
});
onUnmounted(() => {
  unlisten?.();
});
</script>

<template>
  <div class="toast-root">
    <div class="toast-card">
      <span class="toast-msg">{{ message }}</span>
      <span class="toast-hint">即将关闭</span>
    </div>
  </div>
</template>

<style>
.toast-root {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  user-select: none;
  pointer-events: none;
}

.toast-card {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 12px 20px;
  border: 1px solid rgba(245, 158, 11, 0.45);
  border-radius: var(--radius-pill);
  background: rgba(13, 17, 28, 0.94);
  max-width: 100%;
  white-space: nowrap;
  overflow: hidden;
}

.toast-msg {
  font-size: 13px;
  font-weight: 600;
  color: var(--yellow);
  overflow: hidden;
  text-overflow: ellipsis;
}

.toast-hint {
  flex-shrink: 0;
  font-size: 11px;
  color: var(--text-3);
}
</style>