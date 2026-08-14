<script setup lang="ts">
import { computed } from "vue";
import { useDropdown } from "../../composables/useDropdown";

const props = defineProps<{ available: string[] }>();
const emit = defineEmits<{ select: [string] }>();

const { open, trigger, panel, pos, toggle, close } = useDropdown("104px");

/** 每个变量名生成两种模板：${name} 与 ${name:默认值}。 */
const items = computed(() => {
  const r: { label: string; insert: string }[] = [];
  for (const n of props.available) {
    r.push({ label: `\${${n}}`, insert: `\${${n}}` });
    r.push({ label: `\${${n}:默认值}`, insert: `\${${n}:默认值}` });
  }
  return r;
});

function pick(insert: string) {
  emit("select", insert);
  close();
}
</script>

<template>
  <div class="sselect var-insert" :style="{ width: '104px' }">
    <div ref="trigger" class="sselect-trigger" :class="{ open }" @click="toggle">
      <span class="sselect-value"><span class="var-insert-label">变量…</span></span>
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
            v-for="it in items"
            :key="it.insert"
            type="button"
            class="sselect-option"
            @click="pick(it.insert)"
          >
            <span>{{ it.label }}</span>
          </button>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style>
.var-insert {
  flex-shrink: 0;
}

.var-insert .sselect-trigger {
  font-family: "Cascadia Code", Consolas, monospace;
  font-size: 12px;
  padding: 0 8px;
}

.var-insert-label {
  color: var(--text-3);
}
</style>
