<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, reactive, ref } from "vue";

/** 通用悬浮提示：问号按钮 + 悬浮弹层（与步骤说明 StepInfoPopover 同一交互模式）。 */
defineProps<{ title: string; text: string }>();

const open = ref(false);
const trigger = ref<HTMLButtonElement | null>(null);
const panel = ref<HTMLDivElement | null>(null);
const pos = reactive<{ top: string; left: string }>({ top: "0px", left: "0px" });

function updatePosition() {
  if (!trigger.value) return;
  const r = trigger.value.getBoundingClientRect();
  const gap = 8;
  const panelW = panel.value?.offsetWidth ?? 260;
  // 右侧空间不足则向左弹出，否则向右弹出；顶部空间不足则向上对齐窗口内。
  const flipLeft = r.right + gap + panelW > window.innerWidth;
  pos.left = flipLeft
    ? `${r.left - gap - panelW}px`
    : `${r.right + gap}px`;
  const panelH = panel.value?.offsetHeight ?? 0;
  const top = r.top;
  pos.top = `${Math.max(8, Math.min(top, window.innerHeight - panelH - 8))}px`;
}

async function show() {
  open.value = true;
  await nextTick();
  updatePosition();
}

function hide() {
  open.value = false;
}

function onScroll() {
  if (open.value) open.value = false;
}

onMounted(() => {
  window.addEventListener("scroll", onScroll, true);
});

onUnmounted(() => {
  window.removeEventListener("scroll", onScroll, true);
});
</script>

<template>
  <div class="info-pop" @mouseenter="show" @mouseleave="hide">
    <button
      ref="trigger"
      type="button"
      class="info-pop-btn"
      title="说明"
      @click.stop.prevent
    >
      <svg
        viewBox="0 0 24 24"
        width="14"
        height="14"
        fill="none"
        stroke="currentColor"
        stroke-width="2.2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <circle cx="12" cy="12" r="9" />
        <path d="M12 11v5" />
        <path d="M12 8h.01" />
      </svg>
    </button>
    <Teleport to="body">
      <Transition name="sinfo">
        <div v-if="open" ref="panel" class="info-pop-panel" :style="pos">
          <div class="info-pop-title">{{ title }}</div>
          <div class="info-pop-text">{{ text }}</div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style>
.info-pop {
  position: relative;
  flex-shrink: 0;
  display: inline-flex;
}

.info-pop-btn {
  width: 32px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text-2);
  cursor: help;
  transition: background 0.15s, color 0.15s;
}

.info-pop-btn:hover {
  color: var(--accent);
  background: rgba(109, 124, 255, 0.15);
}

.info-pop-panel {
  position: fixed;
  z-index: 160;
  width: 260px;
  padding: 12px 14px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  background: linear-gradient(180deg, #1b2030, #141823);
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
  pointer-events: none;
}

.info-pop-title {
  font-size: 13px;
  font-weight: 700;
  color: #fff;
  margin-bottom: 6px;
}

.info-pop-text {
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-2);
}
</style>
