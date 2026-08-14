<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref } from "vue";
import type { Step } from "../../types";

type StepType = Step["type"];

interface StepInfo {
  label: string;
  usage: string;
  vars: string[];
}

const props = defineProps<{ stepType: StepType }>();

/** 各步骤类型的说明：用途 + 产生的/支持的变量。 */
const INFO: Record<StepType, StepInfo> = {
  wait: {
    label: "等待",
    usage: "暂停指定毫秒数，常用于等待目标应用启动或 UI 就绪。",
    vars: [],
  },
  hotkey: {
    label: "快捷键",
    usage: "注入一个组合键，如 Ctrl+C、Enter。keys 字段支持 ${title} 变量展开。",
    vars: [],
  },
  activateApp: {
    label: "激活应用",
    usage:
      "按标题或 exe 匹配并激活窗口；未运行时自动启动（exe 或商店应用 AUMID）。",
    vars: ["${title}：聚焦成功后写入该窗口的真实标题"],
  },
  focusApp: {
    label: "聚焦应用",
    usage: "聚焦已打开的应用窗口（按标题模糊匹配，不启动新进程）。",
    vars: ["${title}：聚焦成功后写入该窗口的真实标题"],
  },
  setClipboard: {
    label: "写剪贴板",
    usage: "把文本写入剪贴板。text 字段支持 ${title}、${title:默认值} 变量展开。",
    vars: [],
  },
  typeText: {
    label: "输入文本(逐字)",
    usage: "逐字输入文本（受输入法影响，慎用于中文）。text 支持变量展开。",
    vars: [],
  },
  pasteText: {
    label: "输入文本(粘贴)",
    usage: "写入剪贴板并粘贴（对中文/长文本可靠）。text 支持变量展开。",
    vars: [],
  },
  runCommand: {
    label: "运行命令",
    usage: "运行外部命令。cmd 与每个 args 支持 ${title} 等变量展开。",
    vars: [],
  },
  click: {
    label: "点击坐标",
    usage:
      "在标题匹配的窗口内按 x/y 轴各自定位模拟左键点击，用于聚焦输入框等控件。",
    vars: [],
  },
  if: {
    label: "条件判断",
    usage:
      "按操作符比较 value 与 expected（均支持 ${title} 变量），命中则执行 then，否则依次尝试 elseIf，最终执行 else。字符串比较区分大小写；大于/小于在两侧都是数值时按数值比较，否则按字符串比较；正则匹配失败视为不命中。",
    vars: [],
  },
  rollbackClipboard: {
    label: "剪切板回滚",
    usage: "把剪贴板恢复为配方执行前的原始内容（置于复制/粘贴步骤之后）。",
    vars: [],
  },
};

const info = computed(() => INFO[props.stepType] ?? INFO.wait);

const open = ref(false);
const trigger = ref<HTMLButtonElement | null>(null);
const panel = ref<HTMLDivElement | null>(null);
const pos = reactive<{ top: string; left: string }>({ top: "0px", left: "0px" });

function updatePosition() {
  if (!trigger.value) return;
  const r = trigger.value.getBoundingClientRect();
  const gap = 8;
  const panelW = panel.value?.offsetWidth ?? 280;
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
  <div class="step-info" @mouseenter="show" @mouseleave="hide">
    <button
      ref="trigger"
      type="button"
      class="step-info-btn"
      title="步骤说明"
      @click.stop
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
        <div v-if="open" ref="panel" class="step-info-pop" :style="pos">
          <div class="step-info-title">{{ info.label }}</div>
          <div class="step-info-usage">{{ info.usage }}</div>
          <div v-if="info.vars.length" class="step-info-vars">
            <div class="step-info-vars-label">产生的变量</div>
            <div v-for="v in info.vars" :key="v" class="step-info-var">{{ v }}</div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style>
.step-info {
  position: relative;
  flex-shrink: 0;
  display: inline-flex;
}

/* 放在按钮组内时，与上移/下移/删除按钮保持一致的尺寸与交互。 */
.step-info .step-info-btn {
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

.step-info .step-info-btn:hover {
  color: var(--accent);
  background: rgba(109, 124, 255, 0.15);
}

.step-info-pop {
  position: fixed;
  z-index: 160;
  width: 280px;
  padding: 14px 16px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  background: linear-gradient(180deg, #1b2030, #141823);
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
  pointer-events: none;
}

.step-info-title {
  font-size: 14px;
  font-weight: 700;
  color: #fff;
  margin-bottom: 8px;
}

.step-info-usage {
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-2);
}

.step-info-vars {
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px dashed var(--border);
}

.step-info-vars-label {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-3);
  margin-bottom: 6px;
  letter-spacing: 0.4px;
}

.step-info-var {
  font-family: "Cascadia Code", Consolas, monospace;
  font-size: 11.5px;
  color: var(--accent);
  padding: 2px 0;
}

.sinfo-enter-active,
.sinfo-leave-active {
  transition: opacity 0.12s, transform 0.12s;
}

.sinfo-enter-from,
.sinfo-leave-to {
  opacity: 0;
  transform: translateX(-4px);
}
</style>
