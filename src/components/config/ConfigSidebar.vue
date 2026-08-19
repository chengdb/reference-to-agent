<script setup lang="ts">
import { useConfigState } from "../../composables/useConfigState";

const { activeSection, setSection } = useConfigState();
</script>

<template>
  <aside class="sidebar">
    <button
      class="sidebar-item"
      :class="{ active: activeSection === 'menu' }"
      @click="setSection('menu')"
    >
      <svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v3M12 18v3M3 12h3M18 12h3" /><circle cx="12" cy="12" r="4.5" /></svg>
      <span>菜单设置</span>
    </button>
    <button
      class="sidebar-item"
      :class="{ active: activeSection === 'recipes' }"
      @click="setSection('recipes')"
    >
      <svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 3h6M10 3v5.5L5.5 18a2 2 0 0 0 1.8 3h9.4a2 2 0 0 0 1.8-3L14 8.5V3" /><path d="M7.5 15h9" /></svg>
      <span>配方设置</span>
    </button>
    <button
      class="sidebar-item"
      :class="{ active: activeSection === 'basic' }"
      @click="setSection('basic')"
    >
      <svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.3 3.3a1.8 1.8 0 0 1 3.4 0l.7 2a1.8 1.8 0 0 0 1.3 1.2l2.1.4a1.8 1.8 0 0 1 1 2.9l-1.4 1.6a1.8 1.8 0 0 0-.4 1.7l.6 2a1.8 1.8 0 0 1-2.5 2.2l-1.9-1a1.8 1.8 0 0 0-1.7 0l-1.9 1a1.8 1.8 0 0 1-2.5-2.2l.6-2a1.8 1.8 0 0 0-.4-1.7L5.1 10a1.8 1.8 0 0 1 1-2.9l2.1-.4a1.8 1.8 0 0 0 1.3-1.2z" /><circle cx="12" cy="12" r="3" /></svg>
      <span>全局设置</span>
    </button>
  </aside>
</template>

<style>
.sidebar {
  width: 172px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.03);
  align-self: stretch;
}

.sidebar-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 10px 12px;
  border: none;
  border-radius: 11px;
  background: transparent;
  color: var(--text-2);
  font-size: 15px;
  font-weight: 600;
  text-align: left;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.sidebar-item svg {
  flex-shrink: 0;
  color: var(--text-3);
  transition: color 0.15s;
}

.sidebar-item:hover {
  background: rgba(255, 255, 255, 0.06);
}

.sidebar-item.active {
  background: linear-gradient(135deg, rgba(109, 124, 255, 0.22), rgba(168, 85, 247, 0.18));
  color: #fff;
}

.sidebar-item.active svg {
  color: var(--accent);
}
</style>
