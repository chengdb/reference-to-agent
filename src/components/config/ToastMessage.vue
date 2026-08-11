<script setup lang="ts">
import { useConfigStore } from "../../composables/useConfigStore";

const { toast } = useConfigStore();
</script>

<template>
  <Transition name="toast">
    <div v-if="toast" class="toast" :class="toast.type">
      <svg
        v-if="toast.type === 'success'"
        viewBox="0 0 24 24"
        width="16"
        height="16"
        fill="none"
        stroke="currentColor"
        stroke-width="2.6"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M20 6L9 17l-5-5" />
      </svg>
      <svg
        v-else
        viewBox="0 0 24 24"
        width="16"
        height="16"
        fill="none"
        stroke="currentColor"
        stroke-width="2.6"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M12 8v5M12 17h.01" />
        <circle cx="12" cy="12" r="9" />
      </svg>
      <span>{{ toast.msg }}</span>
    </div>
  </Transition>
</template>

<style>
/* ---------- 保存提示弹窗 ---------- */

.toast {
  position: fixed;
  top: 20px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 200;
  max-width: 380px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 18px;
  border-radius: var(--radius-pill);
  font-size: 13px;
  font-weight: 600;
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(6px);
}

.toast svg {
  flex-shrink: 0;
}

.toast.success {
  background: rgba(52, 211, 153, 0.14);
  border: 1px solid rgba(52, 211, 153, 0.35);
  color: var(--green);
}

.toast.error {
  background: rgba(248, 113, 113, 0.14);
  border: 1px solid rgba(248, 113, 113, 0.35);
  color: var(--red);
}

.toast-enter-active,
.toast-leave-active {
  transition: opacity 0.2s, transform 0.2s;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(-8px);
}
</style>
