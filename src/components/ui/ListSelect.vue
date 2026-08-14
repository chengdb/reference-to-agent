<script setup lang="ts" generic="T extends string">
import { useDropdown } from "../../composables/useDropdown";

const props = withDefaults(
  defineProps<{
    modelValue?: T;
    options: { value: T; label: string }[];
    width?: string;
  }>(),
  { width: "124px" }
);
const emit = defineEmits<{ "update:modelValue": [T] }>();

const { open, trigger, panel, pos, toggle, close } = useDropdown(props.width);

const current = () => props.options.find((o) => o.value === props.modelValue);

function pick(v: T) {
  emit("update:modelValue", v);
  close();
}
</script>

<template>
  <div class="sselect" :style="{ width }">
    <div ref="trigger" class="sselect-trigger" :class="{ open }" @click="toggle">
      <span class="sselect-value" :class="{ placeholder: !current() }">
        {{ current()?.label ?? "请选择" }}
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
    </div>
    <Teleport to="body">
      <Transition name="sselect">
        <div v-if="open" ref="panel" class="sselect-pop" :style="pos">
          <button
            v-for="o in options"
            :key="o.value"
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
