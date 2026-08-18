<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import ConfigSidebar from "../components/config/ConfigSidebar.vue";
import BasicSettingsPanel from "../components/config/BasicSettingsPanel.vue";
import RecipeEditorPanel from "../components/config/RecipeEditorPanel.vue";
import MenuSettingsPanel from "../components/config/MenuSettingsPanel.vue";
import AppPickerDialog from "../components/config/AppPickerDialog.vue";
import ToastMessage from "../components/config/ToastMessage.vue";
import { useConfigStore } from "../composables/useConfigStore";

const { load, save, onKeydown, onCoordPickKeydown, loadError } = useConfigStore();

onMounted(() => {
  load();
  window.addEventListener("keydown", onKeydown, true);
  window.addEventListener("keydown", onCoordPickKeydown, true);
});
onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown, true);
  window.removeEventListener("keydown", onCoordPickKeydown, true);
});
</script>

<template>
  <div class="config">
    <div v-if="loadError" class="config-load-error">
      <span class="config-load-error-title">配置加载失败</span>
      <span class="config-load-error-msg">{{ loadError }}</span>
    </div>
    <div class="config-head">
      <div class="config-brand">
        <div class="config-logo"></div>
        <div>
          <div class="config-title">Reference to Agent</div>
          <div class="config-sub">把代码 / 文件路径一键发送给 AI Agent</div>
        </div>
      </div>
      <div class="config-actions">
        <button class="btn btn-primary" @click="save">保存并应用</button>
      </div>
    </div>

    <div class="config-body">
      <ConfigSidebar />

      <div class="config-panel">
        <BasicSettingsPanel />
        <RecipeEditorPanel />
        <MenuSettingsPanel />
      </div>
    </div>

    <AppPickerDialog />
    <ToastMessage />
  </div>
</template>

<style>
/* ---------- 配置窗口 ---------- */

.config {
  height: 100vh;
  padding: 12px 16px 16px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  box-sizing: border-box;
  overflow: hidden;
  background:
    radial-gradient(1100px 480px at 15% -10%, rgba(109, 124, 255, 0.12), transparent 60%),
    radial-gradient(900px 420px at 100% -5%, rgba(168, 85, 247, 0.08), transparent 55%),
    var(--bg);
}

.config-load-error {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 12px;
  border: 1px solid rgba(248, 113, 113, 0.5);
  border-radius: 10px;
  background: rgba(248, 113, 113, 0.12);
  color: var(--red, #f87171);
  font-size: 12px;
}
.config-load-error-title {
  font-weight: 700;
}
.config-load-error-msg {
  font-family: ui-monospace, monospace;
  word-break: break-all;
  white-space: pre-wrap;
  opacity: 0.9;
}

.config-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.config-brand {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.config-logo {
  position: relative;
  width: 34px;
  height: 34px;
  border-radius: 11px;
  background: var(--accent-grad);
  box-shadow: 0 6px 16px rgba(124, 108, 255, 0.4);
  flex-shrink: 0;
}

.config-logo::after {
  content: "";
  position: absolute;
  inset: 10px;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.85);
}

.config-title {
  font-size: 20px;
  font-weight: 700;
  letter-spacing: 0.2px;
  line-height: 1.2;
}

.config-sub {
  font-size: 12px;
  color: var(--text-3);
  margin-top: 2px;
}

.config-actions {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}

/* ---------- 配置页主体：左侧栏 + 面板 ---------- */

.config-body {
  flex: 1;
  min-height: 0;
  display: flex;
  gap: 16px;
  overflow-x: auto;
}

.config-panel {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
}

.config-panel .recipe-editor {
  flex: 1;
  min-height: 400px;
}
</style>
