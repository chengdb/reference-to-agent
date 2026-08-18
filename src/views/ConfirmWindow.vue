<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

/** 后端下发的确认请求内容（与 Rust ConfirmRequest 字段对应）。 */
interface ConfirmReq {
  recipeName: string;
  stepSeq: number;
  stepDesc: string;
}

const win = getCurrentWindow();
const recipeName = ref("");
const stepSeq = ref(0);
const stepDesc = ref("");

/** 防抖：短时间内的连点只响应第一次，避免一次确认误拖到下一个确认。 */
let lastRespondAt = 0;

function respond(choice: string) {
  const now = Date.now();
  if (now - lastRespondAt < 350) return;
  lastRespondAt = now;
  invoke("confirm_step", { choice });
}

function onKeydown(e: KeyboardEvent) {
  if (e.repeat) return;
  if (e.key === "Enter") {
    e.preventDefault();
    respond(e.shiftKey ? "confirmAll" : "confirm");
  } else if (e.key === "Escape") {
    e.preventDefault();
    respond("cancel");
  }
}

/** 窗口失焦（切走/点别处）视为取消，与圆盘/列表窗口行为一致。 */
function onBlur() {
  respond("cancel");
}

function applyReq(req: ConfirmReq) {
  recipeName.value = req.recipeName;
  stepSeq.value = req.stepSeq;
  stepDesc.value = req.stepDesc;
  // 新请求到来即重置防抖，保证新确认立即可响应。
  lastRespondAt = 0;
}

/** 主动拉取最新的确认请求，兜底事件先于监听注册的时序问题。 */
async function refresh() {
  try {
    const req = await invoke<ConfirmReq | null>("get_confirm_request");
    if (req) applyReq(req);
  } catch {
    /* 后端尚未就绪时忽略 */
  }
}

let unlisten: UnlistenFn | undefined;
let unlistenFocus: UnlistenFn | undefined;
onMounted(async () => {
  unlisten = await listen<ConfirmReq>("confirm-request", (e) => applyReq(e.payload));
  unlistenFocus = await win.onFocusChanged(({ payload }) => {
    if (payload) refresh();
  });
  window.addEventListener("keydown", onKeydown);
  window.addEventListener("blur", onBlur);
  await refresh();
});
onUnmounted(() => {
  unlisten?.();
  unlistenFocus?.();
  window.removeEventListener("keydown", onKeydown);
  window.removeEventListener("blur", onBlur);
});
</script>

<template>
  <div class="confirm-root">
    <div class="confirm-card">
      <div class="confirm-head">
        <span class="confirm-badge">人工确认</span>
        <span class="confirm-recipe">{{ recipeName }}</span>
      </div>
      <div class="confirm-desc">
        <span v-if="stepSeq > 0" class="confirm-seq">第 {{ stepSeq }} 步</span>
        <span class="confirm-text">{{ stepDesc }}</span>
      </div>
      <div class="confirm-hint">
        按 <b>Enter</b> 确认 · <b>Shift+Enter</b> 确认剩余步骤 · <b>Esc</b> 取消
      </div>
    </div>
  </div>
</template>

<style>
.confirm-root {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 14px;
  user-select: none;
}

.confirm-card {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 18px 20px;
  border: 1.5px solid rgba(124, 108, 255, 0.55);
  border-radius: 18px;
  background: linear-gradient(180deg, rgba(24, 29, 46, 0.96), rgba(13, 17, 28, 0.96));
  box-shadow: 0 12px 40px rgba(2, 6, 23, 0.6);
}

.confirm-head {
  display: flex;
  align-items: center;
  gap: 10px;
}

.confirm-badge {
  padding: 3px 10px;
  border-radius: var(--radius-pill);
  background: rgba(245, 158, 11, 0.16);
  border: 1px solid rgba(245, 158, 11, 0.35);
  color: var(--yellow);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 1px;
  flex-shrink: 0;
}

.confirm-recipe {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-2);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.confirm-desc {
  display: flex;
  align-items: baseline;
  gap: 8px;
  min-height: 42px;
}

.confirm-seq {
  flex-shrink: 0;
  font-family: "Cascadia Code", Consolas, monospace;
  font-size: 12px;
  font-weight: 700;
  color: var(--accent);
}

.confirm-text {
  font-size: 16px;
  font-weight: 600;
  color: var(--text);
  line-height: 1.4;
  word-break: break-all;
}

.confirm-hint {
  padding-top: 3px;
  border-top: 1px solid var(--border);
  color: var(--text-3);
  font-size: 12px;
  text-align: center;
}

.confirm-hint b {
  color: var(--accent);
}
</style>