<script setup lang="ts">
import { useConfigState } from "../../composables/useConfigState";
import { useHotkeyRecording } from "../../composables/useHotkeyRecording";

const { cfg, activeSection } = useConfigState();
const { recording, startRecording } = useHotkeyRecording();
</script>

<template>
  <section v-show="activeSection === 'basic'" class="card config-section basic-panel">
    <div class="config-label">全局快捷键 · 弹出圆盘菜单</div>
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

    <div class="config-label">全局快捷键 · 弹出完整配方列表</div>
    <div class="hotkey-row">
      <input
        class="config-input hotkey-input"
        :value="cfg.listHotkey"
        readonly
        :placeholder="recording === 'list' ? '请按下组合键…' : '点击右侧「录制」设置'"
      />
      <button
        class="rec-btn"
        :class="{ recording: recording === 'list' }"
        @click="startRecording('list')"
      >
        {{ recording === "list" ? "录制中 · Esc 取消" : "录制" }}
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
