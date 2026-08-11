<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, reactive, ref } from "vue";

type OptionValue = string | number | null;

const props = withDefaults(
  defineProps<{
    modelValue: OptionValue;
    options: { value: OptionValue; label: string }[];
    disabled?: boolean;
    placeholder?: string;
  }>(),
  { disabled: false, placeholder: "请选择" }
);
const emit = defineEmits<{ "update:modelValue": [OptionValue] }>();

const open = ref(false);
const trigger = ref<HTMLButtonElement | null>(null);
const panel = ref<HTMLDivElement | null>(null);
const pos = reactive<{ top: string; bottom: string; left: string; width: string }>({
  top: "0px",
  bottom: "",
  left: "0px",
  width: "124px",
});

const current = () => props.options.find((o) => o.value === props.modelValue);

async function toggle() {
  if (props.disabled) return;
  open.value = !open.value;
  if (open.value) {
    await nextTick();
    updatePosition();
  }
}

function updatePosition() {
  if (!trigger.value) return;
  const r = trigger.value.getBoundingClientRect();
  const gap = 8;
  const panelH = panel.value?.offsetHeight ?? 0;
  const spaceBelow = window.innerHeight - r.bottom - gap;
  pos.left = `${r.left}px`;
  pos.width = `${r.width}px`;
  if (spaceBelow < panelH && r.top > spaceBelow) {
    pos.top = "";
    pos.bottom = `${window.innerHeight - r.top + gap}px`;
  } else {
    pos.bottom = "";
    pos.top = `${r.bottom + gap}px`;
  }
}

function pick(v: OptionValue) {
  emit("update:modelValue", v);
  open.value = false;
}

function onDocClick(e: MouseEvent) {
  if (trigger.value && !trigger.value.contains(e.target as Node)) open.value = false;
}

function onScroll(e: Event) {
  if (!open.value || !trigger.value) return;
  const t = e.target as Node;
  if (
    t === document ||
    (t.nodeType === 1 && (t as HTMLElement).contains(trigger.value))
  ) {
    open.value = false;
  }
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") open.value = false;
}

onMounted(() => {
  document.addEventListener("click", onDocClick);
  window.addEventListener("scroll", onScroll, true);
  window.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  document.removeEventListener("click", onDocClick);
  window.removeEventListener("scroll", onScroll, true);
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div class="sselect">
    <button
      ref="trigger"
      type="button"
      class="sselect-trigger"
      :class="{ open, disabled }"
      :disabled="disabled"
      @click="toggle"
    >
      <span class="sselect-value" :class="{ placeholder: !current() }">
        {{ current()?.label ?? placeholder }}
      </span>
      <svg
        class="sselect-chevron"
        :class="{ up: open }"
        viewBox="0 0 24 24"
        width="14"
        height="14"
        fill="none"
        stroke="currentColor"
        stroke-width="2.4"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M6 9l6 6 6-6" />
      </svg>
    </button>
    <Teleport to="body">
      <Transition name="sselect">
        <div v-if="open" ref="panel" class="sselect-pop" :style="pos">
          <button
            v-for="o in options"
            :key="String(o.value)"
            type="button"
            class="sselect-option"
            :class="{ selected: o.value === modelValue }"
            @click="pick(o.value)"
          >
            <span>{{ o.label }}</span>
            <svg
              v-if="o.value === modelValue"
              class="sselect-check"
              viewBox="0 0 24 24"
              width="14"
              height="14"
              fill="none"
              stroke="currentColor"
              stroke-width="2.4"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M20 6L9 17l-5-5" />
            </svg>
          </button>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style>
.sselect {
  position: relative;
  flex-shrink: 0;
}

.sselect-trigger {
  height: 40px;
  width: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  background: #0e1119;
  color: var(--text);
  font-size: 13px;
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.sselect-trigger:hover {
  border-color: var(--accent);
}

.sselect-trigger.open {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px rgba(109, 124, 255, 0.18);
}

.sselect-trigger.disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.sselect-value.placeholder {
  color: var(--text-3);
}

.sselect-value {
  flex: 1;
  text-align: left;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sselect-chevron {
  color: var(--text-3);
  flex-shrink: 0;
  transition: transform 0.15s;
}

.sselect-chevron.up {
  transform: rotate(180deg);
}

.sselect-pop {
  position: fixed;
  z-index: 120;
  padding: 6px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  background: linear-gradient(180deg, #1b2030, #141823);
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
}

.sselect-option {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 10px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text-2);
  font-size: 13px;
  text-align: left;
  cursor: pointer;
  transition: background 0.12s;
}

.sselect-option:hover {
  background: rgba(255, 255, 255, 0.07);
}

.sselect-option.selected {
  background: linear-gradient(135deg, rgba(109, 124, 255, 0.2), rgba(168, 85, 247, 0.15));
  color: #fff;
}

.sselect-check {
  margin-left: auto;
  color: var(--accent);
}

.sselect-enter-active,
.sselect-leave-active {
  transition: opacity 0.12s, transform 0.12s;
}

.sselect-enter-from,
.sselect-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
