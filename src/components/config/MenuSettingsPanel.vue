<script setup lang="ts">
import { ref } from "vue";
import Radial from "../ui/Radial.vue";
import Select from "../ui/Select.vue";
import { useConfigState } from "../../composables/useConfigState";
import { useMenuSlots } from "../../composables/useMenuSlots";

const previewBgOptions = [
  { value: "dark", label: "暗色" },
  { value: "light", label: "亮色" },
  { value: "transparent", label: "透明" },
] as const;
const previewBg = ref<"dark" | "light" | "transparent">("dark");

const { cfg, activeSection } = useConfigState();
const {
  selectedSlot,
  orderedPreview,
  boundSlot,
  selectedItem,
  sectorOptions,
  fontSizeOptions,
  recipeOptions,
  nameColorHistory,
  sectorColorHistory,
  onPreviewSelect,
  onBindUpdate,
  setSlotLabel,
  setSlotIcon,
  setSlotLabelSize,
  setSlotLabelColor,
  setSlotColor,
  clearSlotLabelColor,
  clearSlotColor,
  setSlotShowIcon,
  setSlotShowLabel,
} = useMenuSlots();
</script>

<template>
  <section v-show="activeSection === 'menu'" class="card config-section menu-panel">
    <div class="menu-settings-main">
      <div class="ring-area">
        <div class="ring-controls">
          <div class="setting-card">
            <label for="menu-size">菜单大小</label>
            <div class="setting-card-control">
              <input
                id="menu-size"
                class="range-input"
                type="range"
                min="320"
                max="600"
                step="10"
                v-model.number="cfg.menu.size"
              />
              <span class="setting-value">{{ cfg.menu.size }}px</span>
            </div>
          </div>

          <div class="setting-card">
            <label for="menu-sectors">扇区数量</label>
            <Select
              id="menu-sectors"
              class="setting-select"
              v-model.number="cfg.menu.sectors"
              :options="sectorOptions"
            />
          </div>

          <div class="setting-card preview-bg-card">
            <label>预览背景</label>
            <div class="preview-bg-opts">
              <button
                v-for="opt in previewBgOptions"
                :key="opt.value"
                type="button"
                class="preview-bg-opt"
                :class="{ active: previewBg === opt.value }"
                @click="previewBg = opt.value"
              >
                <span class="preview-bg-swatch" :class="`bg-${opt.value}`"></span>
                {{ opt.label }}
              </button>
            </div>
          </div>
        </div>

        <div class="radial-preview" :class="`bg-${previewBg}`">
          <Radial
            :items="orderedPreview"
            :size="cfg.menu.size"
            :sectors="cfg.menu.sectors"
            :show-labels="cfg.menu.showLabels"
            :selected="selectedSlot"
            interactive
            allow-empty-select
            show-indices
            @select="onPreviewSelect"
          />
        </div>
      </div>

      <div class="bind-panel">
        <div class="bind-head">
          <div class="bind-title">
            <template v-if="selectedSlot != null">第 {{ selectedSlot + 1 }} 个扇区</template>
            <template v-else>未选择扇区</template>
          </div>
          <div class="bind-current">
            <template v-if="selectedItem">
              {{ selectedItem.label }}
            </template>
            <template v-else-if="selectedSlot != null">尚未绑定配方</template>
            <template v-else>点击左侧圆环选择一个扇区</template>
          </div>
        </div>

        <div class="config-label bind-label">绑定配方</div>
        <Select
          class="bind-select"
          :model-value="boundSlot?.recipe ?? ''"
          :disabled="selectedSlot == null"
          :options="recipeOptions"
          placeholder="点击左侧圆环选择一个扇区"
          @update:model-value="onBindUpdate"
        />

        <div
          v-if="boundSlot && boundSlot.recipe != null"
          class="bind-appearance"
        >
          <div class="config-label">菜单按钮外观</div>
          <div class="setting-row">
            <label for="slot-label">显示名称</label>
            <input
              id="slot-label"
              class="config-input"
              :value="boundSlot.label ?? ''"
              placeholder="默认显示配方名"
              :disabled="boundSlot.showLabel === false"
              @input="setSlotLabel(($event.target as HTMLInputElement).value)"
            />
            <label class="show-check" title="是否显示名称">
              <input
                type="checkbox"
                :checked="boundSlot.showLabel !== false"
                @change="setSlotShowLabel(($event.target as HTMLInputElement).checked)"
              />
              <span>显示</span>
            </label>
          </div>
          <div class="setting-row">
            <label for="slot-icon">图标</label>
            <input
              id="slot-icon"
              class="config-input"
              :value="boundSlot.icon ?? ''"
              placeholder="emoji，如 🧰"
              :disabled="boundSlot.showIcon === false"
              @input="setSlotIcon(($event.target as HTMLInputElement).value)"
            />
            <label class="show-check" title="是否显示图标">
              <input
                type="checkbox"
                :checked="boundSlot.showIcon !== false"
                @change="setSlotShowIcon(($event.target as HTMLInputElement).checked)"
              />
              <span>显示</span>
            </label>
          </div>
          <div class="setting-row">
            <label for="slot-fontsize">名称字号</label>
            <Select
              id="slot-fontsize"
              class="setting-select"
              :model-value="boundSlot.labelSize ?? 12"
              :options="fontSizeOptions"
              @update:model-value="setSlotLabelSize(Number($event))"
            />
          </div>
          <div class="setting-row">
            <label for="slot-labelcolor">名称颜色</label>
            <input
              type="color"
              class="color-input"
              :value="boundSlot.labelColor ?? '#a6aec4'"
              @change="setSlotLabelColor(($event.target as HTMLInputElement).value)"
            />
            <button v-if="boundSlot.labelColor" class="pick-btn" @click="clearSlotLabelColor">
              恢复默认
            </button>
            <div v-if="nameColorHistory.length" class="color-swatches">
              <button
                v-for="c in nameColorHistory"
                :key="'l' + c"
                class="color-swatch"
                :style="{ background: c }"
                :title="c"
                @click="setSlotLabelColor(c)"
              ></button>
            </div>
          </div>
          <div class="setting-row">
            <label>扇区颜色</label>
            <input
              type="color"
              class="color-input"
              :value="boundSlot.color ?? '#6d7cff'"
              @change="setSlotColor(($event.target as HTMLInputElement).value)"
            />
            <button v-if="boundSlot.color" class="pick-btn" @click="clearSlotColor">
              恢复默认
            </button>
            <div v-if="sectorColorHistory.length" class="color-swatches">
              <button
                v-for="c in sectorColorHistory"
                :key="'s' + c"
                class="color-swatch"
                :style="{ background: c }"
                :title="c"
                @click="setSlotColor(c)"
              ></button>
            </div>
          </div>
          <p class="bind-hint">外观仅作用于当前扇区；「显示名称」留空时显示配方名。</p>
        </div>
      </div>
    </div>
  </section>
</template>

<style>
.menu-panel {
  flex: 1;
  min-height: 780px;
  min-width: 1260px;
}

/* ---------- 菜单设置 ---------- */

.menu-settings-main {
  flex: 1;
  min-height: 684px;
  display: grid;
  grid-template-columns: minmax(696px, 1fr) minmax(560px, 1fr);
  grid-template-rows: minmax(744px, auto);
  gap: 28px;
}

.setting-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.setting-row label {
  width: 92px;
  flex-shrink: 0;
  color: var(--text-2);
  font-size: 13px;
}

.setting-card {
  width: 310px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  padding: 10px 5px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.03);
}

.setting-card label {
  width: 76px;
  flex-shrink: 0;
  color: var(--text-2);
  font-size: 13px;
}

.setting-card-control {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 12px;
}

.range-input {
  width: 130px;
  accent-color: var(--accent);
  cursor: pointer;
}

.setting-value {
  width: 60px;
  height: 40px;
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  background: #0e1119;
  color: var(--text);
  font-size: 13px;
  font-family: "Cascadia Code", Consolas, monospace;
}

.setting-select {
  width: 202px;
}

/* 圆环为主体：左环右绑定 */

.ring-area {
  min-width: 696px;
  min-height: 710px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.02);
  padding: 14px 16px;
  overflow: hidden;
}

.ring-controls {
  flex-shrink: 0;
  display: flex;
  flex-wrap: wrap;
  gap: 14px 24px;
  align-items: center;
}

.radial-preview {
  position: relative;
  flex: 1;
  min-height: 620px;
  min-width: 620px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border);
  border-radius: 14px;
  transition: background 0.2s;
}

.radial-preview.bg-dark {
  background: #0d1019;
}

.radial-preview.bg-light {
  background: #e8ebf2;
}

.radial-preview.bg-transparent {
  background-color: #161a28;
  background-image: repeating-conic-gradient(rgba(255, 255, 255, 0.05) 0% 25%, transparent 0% 50%);
  background-size: 22px 22px;
}

.radial-preview .radial-sectors {
  width: auto;
  height: auto;
  max-width: 100%;
  max-height: 100%;
}

.preview-bg-card {
  width: auto;
}

.preview-bg-opts {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.preview-bg-opt {
  height: 34px;
  padding: 0 10px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  background: rgba(255, 255, 255, 0.05);
  color: var(--text-2);
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
  transition: background 0.15s, border-color 0.15s, color 0.15s;
}

.preview-bg-opt:hover {
  background: rgba(255, 255, 255, 0.1);
}

.preview-bg-opt.active {
  border-color: var(--accent);
  color: var(--accent);
  background: rgba(109, 124, 255, 0.12);
}

.preview-bg-swatch {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  border-radius: 4px;
  border: 1px solid rgba(255, 255, 255, 0.18);
}

.preview-bg-swatch.bg-light {
  background: #e8ebf2;
  border-color: rgba(70, 80, 110, 0.35);
}

.preview-bg-swatch.bg-transparent {
  border-color: rgba(255, 255, 255, 0.18);
  background-image: repeating-conic-gradient(#3a3f52 0% 25%, #232838 0% 50%);
  background-size: 8px 8px;
}

/* ---------- 绑定面板 ---------- */

.bind-panel {
  min-width: 560px;
  min-height: 550px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  overflow-y: auto;
}

.bind-head {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.03);
}

.bind-title {
  font-size: 14px;
  font-weight: 700;
}

.bind-current {
  font-size: 12.5px;
  color: var(--text-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.bind-label {
  margin-top: 4px;
}

.bind-select {
  width: 100%;
}

.bind-appearance {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.03);
}

.bind-appearance .setting-row {
  gap: 10px;
  flex-wrap: wrap;
}

.bind-appearance .setting-row label {
  width: 76px;
}

.show-check {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  width: auto !important;
  flex-shrink: 0;
  color: var(--text-3);
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
}

.show-check input[type="checkbox"] {
  width: 15px;
  height: 15px;
  margin: 0;
  accent-color: var(--accent);
  cursor: pointer;
}

.bind-appearance .setting-row .color-swatches {
  margin-left: auto;
}

.bind-appearance .config-input {
  flex: 1;
}

.color-input {
  width: 44px;
  height: 40px;
  padding: 4px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  background: #0e1119;
  cursor: pointer;
}

.color-swatches {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.color-swatch {
  width: 26px;
  height: 26px;
  border-radius: 8px;
  border: 2px solid rgba(255, 255, 255, 0.14);
  cursor: pointer;
  padding: 0;
  transition: transform 0.12s, border-color 0.12s;
}

.color-swatch:hover {
  transform: scale(1.12);
  border-color: var(--accent);
}

.bind-hint {
  color: var(--text-3);
  font-size: 11.5px;
  margin: 0;
  line-height: 1.6;
}
</style>
