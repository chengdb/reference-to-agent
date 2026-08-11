<script setup lang="ts">
import { useConfigStore } from "../../composables/useConfigStore";

const { cfg, activeSection, recording, startRecording } = useConfigStore();
</script>

<template>
  <section v-show="activeSection === 'basic'" class="card config-section basic-panel">
    <div class="config-label">全局快捷键 · 弹出菜单</div>
    <div class="hotkey-row">
      <input
        class="config-input hotkey-input"
        :value="cfg.globalHotkey"
        readonly
        :placeholder="recording === 'global' ? '请按下组合键…' : '点击右侧「录制」设置'"
      />
      <button
        class="rec-btn"
        :class="{ recording: recording === 'global' }"
        @click="startRecording('global')"
      >
        {{ recording === "global" ? "录制中 · Esc 取消" : "录制" }}
      </button>
    </div>
  </section>
</template>

<style>
.basic-panel {
  flex-shrink: 0;
}

.hotkey-row {
  display: flex;
  gap: 10px;
  align-items: center;
}

.hotkey-input {
  flex: 1;
  max-width: 300px;
  font-family: "Cascadia Code", Consolas, monospace;
}
</style>
