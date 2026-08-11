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
