<script setup lang="ts">
import { useAppPicker } from "../../composables/useAppPicker";

const { picker, pickerSearch, filteredApps, pickApp, closePicker } = useAppPicker();
</script>

<template>
  <div v-if="picker" class="picker-overlay" @click="closePicker">
    <div class="picker-box" @click.stop>
      <div class="picker-title">选择应用</div>
      <div class="picker-search">
        <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="7" /><path d="M21 21l-4.35-4.35" /></svg>
        <input
          v-model="pickerSearch"
          class="config-input"
          placeholder="搜索应用名称或路径…"
          autofocus
        />
      </div>
      <div class="picker-list">
        <div
          v-for="(a, i) in filteredApps"
          :key="i"
          class="picker-item"
          @click="pickApp(a)"
        >
          <span class="picker-name">{{ a.name }}</span>
          <span class="picker-proc">{{ a.exe }}</span>
        </div>
        <div v-if="filteredApps.length === 0" class="picker-empty">
          未找到匹配的应用
        </div>
      </div>
    </div>
  </div>
</template>

<style>
/* ---------- 应用选择弹层 ---------- */

.picker-overlay {
  position: fixed;
  inset: 0;
  background: rgba(5, 7, 12, 0.6);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.picker-box {
  width: 480px;
  height: 60vh;
  min-height: 360px;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-lg);
  background: linear-gradient(180deg, #1b2030, #141823);
  box-shadow: 0 24px 64px rgba(0, 0, 0, 0.55);
  overflow: hidden;
  animation: pop 0.18s ease-out;
}

@keyframes pop {
  from {
    transform: translateY(8px) scale(0.97);
    opacity: 0;
  }
  to {
    transform: none;
    opacity: 1;
  }
}

.picker-title {
  padding: 16px 18px;
  font-size: 15px;
  font-weight: 700;
  border-bottom: 1px solid var(--border);
}

.picker-search {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 14px;
  border-bottom: 1px solid var(--border);
}

.picker-search svg {
  color: var(--text-3);
  flex-shrink: 0;
}

.picker-search .config-input {
  flex: 1;
}

.picker-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.picker-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 11px 14px;
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 13px;
  color: var(--text);
  transition: background 0.15s;
}

.picker-item:hover {
  background: linear-gradient(135deg, rgba(109, 124, 255, 0.2), rgba(168, 85, 247, 0.15));
}

.picker-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  font-weight: 500;
}

.picker-proc {
  color: var(--text-3);
  font-size: 12px;
  flex-shrink: 0;
  max-width: 220px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.picker-empty {
  padding: 24px;
  text-align: center;
  color: var(--text-3);
  font-size: 13px;
}
</style>
