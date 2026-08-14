<script setup lang="ts">
import { computed, ref } from "vue";

const props = withDefaults(
  defineProps<{
    /** 按扇区排好的项（含显示名称与外观），空位为 null。 */
    items: ({
      label: string;
      color?: string;
      icon?: string;
      showIcon?: boolean;
      showLabel?: boolean;
      labelSize?: number;
      labelColor?: string;
    } | null)[];
    size?: number;
    sectors?: number;
    showLabels?: boolean;
    /** 是否响应点击（菜单窗口为 true，配置预览为 false）。 */
    interactive?: boolean;
    /** 交互模式下空扇区也可点击（配置时选择槽位用）。 */
    allowEmptySelect?: boolean;
    /** 当前选中的扇区（高亮）。 */
    selected?: number | null;
    /** 在圆盘外沿显示 1 起的编号（配置预览用）。 */
    showIndices?: boolean;
  }>(),
  {
    size: 400,
    sectors: 8,
    showLabels: true,
    interactive: false,
    allowEmptySelect: false,
    selected: null,
    showIndices: false,
  }
);

const emit = defineEmits<{ (e: "select", index: number): void }>();

const CENTER = computed(() => props.size / 2);
const R_OUTER = computed(() => props.size / 2 - 10);
const R_INNER = computed(() => Math.max(36, props.size * 0.145));

function angle(i: number) {
  return -Math.PI / 2 + (i * 2 * Math.PI) / props.sectors;
}

function sectorPath(i: number) {
  const a0 = angle(i);
  const a1 = angle(i + 1);
  const p = (ang: number, rad: number) => ({
    x: CENTER.value + rad * Math.cos(ang),
    y: CENTER.value + rad * Math.sin(ang),
  });
  const q0 = p(a0, R_OUTER.value);
  const q1 = p(a1, R_OUTER.value);
  const p0 = p(a0, R_INNER.value);
  const p1 = p(a1, R_INNER.value);
  const large = a1 - a0 > Math.PI ? 1 : 0;
  return `M ${q0.x.toFixed(2)} ${q0.y.toFixed(2)} A ${R_OUTER.value} ${R_OUTER.value} 0 ${large} 1 ${q1.x.toFixed(2)} ${q1.y.toFixed(2)} L ${p1.x.toFixed(2)} ${p1.y.toFixed(2)} A ${R_INNER.value} ${R_INNER.value} 0 ${large} 0 ${p0.x.toFixed(2)} ${p0.y.toFixed(2)} Z`;
}

function labelPos(i: number) {
  const aMid = (angle(i) + angle(i + 1)) / 2;
  const rad = (R_OUTER.value + R_INNER.value) / 2;
  return {
    x: CENTER.value + rad * Math.cos(aMid),
    y: CENTER.value + rad * Math.sin(aMid),
  };
}

/** 扇区编号显示在圆环外沿（配置预览用）。 */
function indexPos(i: number) {
  const aMid = (angle(i) + angle(i + 1)) / 2;
  const rad = R_OUTER.value + 10;
  return {
    x: CENTER.value + rad * Math.cos(aMid),
    y: CENTER.value + rad * Math.sin(aMid),
  };
}

/** 有图标时，图标上移、文字下移，避免重叠。 */
function contentPos(i: number) {
  const p = labelPos(i);
  const hasIcon = Boolean(props.items[i]?.icon);
  return {
    icon: { x: p.x, y: p.y - (props.showLabels ? 10 : 0) },
    text: { x: p.x, y: p.y + (hasIcon ? 10 : 0) },
  };
}

function labelStyle(item: { labelSize?: number; labelColor?: string } | null) {
  if (!item) return undefined;
  return {
    ...(item.labelSize ? { fontSize: item.labelSize + "px" } : {}),
    ...(item.labelColor ? { fill: item.labelColor } : {}),
  };
}

function short(name: string) {
  return name.length > 6 ? name.slice(0, 6) + "…" : name;
}

/** 放大时围绕内侧弧中点缩放，使扇区只向环外膨胀、不向两侧重叠。 */
function sectorOrigin(i: number) {
  const aMid = (angle(i) + angle(i + 1)) / 2;
  const ox = CENTER.value + R_INNER.value * Math.cos(aMid);
  const oy = CENTER.value + R_INNER.value * Math.sin(aMid);
  // 限制缩放系数，保证放大后最外沿不超出窗口（size/2 - 4），避免被裁剪。
  const k = Math.min(1.06, Math.max(1.02, (props.size / 2 - 4 - R_INNER.value) / (R_OUTER.value - R_INNER.value)));
  return { "--ox": `${ox.toFixed(2)}px`, "--oy": `${oy.toFixed(2)}px`, "--k": k.toFixed(3) };
}

function onClick(i: number) {
  if (props.interactive && (props.items[i] || props.allowEmptySelect)) {
    emit("select", i);
  }
}

/** 当前悬浮的扇区下标（用于将悬浮扇区排到最上层渲染）。 */
const hovered = ref<number | null>(null);

/** 渲染顺序：悬浮的扇区排最后，保证 SVG 绘制时不被相邻扇区遮挡。 */
const renderOrder = computed(() => {
  const order = props.items.map((_, i) => i);
  if (hovered.value !== null) {
    const idx = order.indexOf(hovered.value);
    if (idx !== -1) order.splice(idx, 1), order.push(hovered.value);
  }
  return order;
});
</script>

<template>
  <svg class="radial-sectors" :viewBox="`0 0 ${size} ${size}`" :width="size" :height="size">
    <template v-for="i in renderOrder" :key="i">
      <path
        :d="sectorPath(i)"
        class="radial-sector"
        :class="{ empty: !items[i], selected: selected === i }"
        :style="{
          ...sectorOrigin(i),
          ...(items[i]?.color ? { fill: items[i].color } : {}),
          pointerEvents: interactive && (items[i] || allowEmptySelect) ? 'auto' : 'none',
        }"
        @click="onClick(i)"
        @mouseenter="hovered = i"
        @mouseleave="hovered = null"
      >
        <title v-if="items[i]">{{ items[i].label }}</title>
      </path>
      <template v-if="items[i]">
        <text
          v-if="items[i].icon && items[i].showIcon !== false"
          :x="contentPos(i).icon.x"
          :y="contentPos(i).icon.y"
          class="radial-icon"
          text-anchor="middle"
          dominant-baseline="central"
        >
          {{ items[i].icon }}
        </text>
        <text
          v-if="showLabels && items[i].showLabel !== false"
          :x="contentPos(i).text.x"
          :y="contentPos(i).text.y"
          class="radial-label"
          :style="labelStyle(items[i])"
          text-anchor="middle"
          dominant-baseline="central"
        >
          {{ short(items[i].label) }}
        </text>
      </template>
      <text
        v-if="showIndices"
        :x="indexPos(i).x"
        :y="indexPos(i).y"
        class="radial-label radial-index"
        text-anchor="middle"
        dominant-baseline="central"
      >
        {{ i + 1 }}
      </text>
    </template>
    <text
      v-if="items.every((it) => !it)"
      :x="CENTER"
      :y="CENTER - 8"
      class="radial-label"
      text-anchor="middle"
      dominant-baseline="central"
    >
      暂无配方
    </text>
  </svg>
</template>

<style>
.radial-sectors {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  overflow: visible;
  flex-shrink: 0;
}

.radial-sector {
  fill: var(--sector-fill, #232a4d);
  stroke: var(--sector-stroke, rgba(255, 255, 255, 0.35));
  stroke-width: 1.5;
  cursor: pointer;
  transition: filter 0.15s, stroke 0.15s, transform 0.15s;
  transform-box: view-box;
  transform-origin: var(--ox, 50%) var(--oy, 50%);
  transform: scale(1);
}

.radial-sector:hover {
  filter: brightness(1.3);
  stroke: rgba(255, 255, 255, 0.75);
  stroke-width: 2.5;
  transform: scale(var(--k, 1.06));
}

.radial-sector.empty {
  fill: var(--sector-empty-fill, rgba(13, 17, 30, 0.35));
  stroke: var(--sector-empty-stroke, rgba(255, 255, 255, 0.55));
  cursor: default;
}

.radial-sector.empty:hover {
  filter: none;
}

.radial-sector.selected {
  stroke: var(--accent);
  stroke-width: 2.5;
  filter: brightness(1.35);
}

.radial-sector.selected:hover {
  filter: brightness(1.35);
}

.radial-label {
  fill: var(--text-2);
  font-size: 12.5px;
  font-weight: 600;
  pointer-events: none;
}

.radial-icon {
  fill: var(--text);
  font-size: 17px;
  pointer-events: none;
}

.radial-index {
  fill: var(--text-3);
  font-size: 11px;
}
</style>
