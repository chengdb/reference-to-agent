<script setup lang="ts">
import { computed } from "vue";
import ListSelect from "../../ui/ListSelect.vue";
import VarInsertSelect from "../VarInsertSelect.vue";
import StepList from "../StepList.vue";
import { useConfigState } from "../../../composables/useConfigState";
import type { CompareOp, StepType } from "../../../types";
import type { EditableIfStep, EditableStep } from "../../../types/editable";

const props = defineProps<{ step: EditableStep }>();
const s = computed(() => props.step as EditableIfStep);

const { current } = useConfigState();

/** 哪些步骤类型会产出哪些变量名（供条件判断下拉选择）。与 Rust actions.rs 的 VAR_TITLE 对齐。 */
const VAR_PRODUCERS: Partial<Record<StepType, string[]>> = {
  focusApp: ["title"],
  activateApp: ["title"],
};

/** 递归收集步骤树中会产出的变量名。 */
function collectVarNames(steps: EditableStep[], out: Set<string>) {
  for (const st of steps) {
    const produced = VAR_PRODUCERS[st.type];
    if (produced) for (const n of produced) out.add(n);
    if (st.type === "if") {
      collectVarNames(st.then ?? [], out);
      for (const b of st.elseIf ?? []) collectVarNames(b.then ?? [], out);
      collectVarNames(st.else ?? [], out);
    }
  }
}

/** 当前配方整棵步骤树中可用的变量名（变量为配方级单值，故扫描全树）。 */
const availableVars = computed(() => {
  const out = new Set<string>();
  collectVarNames(current.value?.steps ?? [], out);
  return [...out];
});

/** 比较操作符选项。 */
const OP_OPTIONS: { value: CompareOp; label: string }[] = [
  { value: "eq", label: "等于 ==" },
  { value: "ne", label: "不等于 !=" },
  { value: "gt", label: "大于 >" },
  { value: "ge", label: "大于等于 >=" },
  { value: "lt", label: "小于 <" },
  { value: "le", label: "小于等于 <=" },
  { value: "startsWith", label: "前缀 startsWith" },
  { value: "endsWith", label: "后缀 endsWith" },
  { value: "contains", label: "包含 contains" },
  { value: "matches", label: "正则匹配 matches" },
];

/** 添加一个 else-if 分支。 */
function addElseIf(step: EditableIfStep) {
  const b = {
    op: "eq" as CompareOp,
    value: "${title}",
    expected: "",
    then: [] as EditableStep[],
  };
  step.elseIf = step.elseIf ?? [];
  step.elseIf.push(b);
}

function removeElseIf(step: EditableIfStep, i: number) {
  if (!step.elseIf) return;
  step.elseIf.splice(i, 1);
}
</script>

<template>
  <div class="step-if">
    <div class="step-if-head">
      <span class="step-if-kw">if</span>
      <ListSelect v-model="s.op" :options="OP_OPTIONS" width="150px" />
      <VarInsertSelect :available="availableVars" @select="s.value = $event" />
      <input v-model="s.value" class="config-input step-if-value" placeholder="${title}" />
      <VarInsertSelect :available="availableVars" @select="s.expected = $event" />
      <input v-model="s.expected" class="config-input step-if-expected" placeholder="期望值" />
      <button class="pick-btn small" @click="addElseIf(s)">+ else if</button>
    </div>

    <div class="step-branch">
      <div class="step-branch-label">then</div>
      <StepList :steps="s.then ?? []" />
    </div>

    <template v-for="(b, bi) in s.elseIf ?? []" :key="'ei' + bi">
      <div class="step-branch">
        <div class="step-branch-label">
          else if
          <ListSelect v-model="b.op" :options="OP_OPTIONS" width="150px" />
          <VarInsertSelect :available="availableVars" @select="b.value = $event" />
          <input v-model="b.value" class="config-input step-if-value" placeholder="${title}" />
          <VarInsertSelect :available="availableVars" @select="b.expected = $event" />
          <input v-model="b.expected" class="config-input step-if-expected" placeholder="期望值" />
          <button class="step-branch-del" title="删除此分支" @click="removeElseIf(s, bi)">×</button>
        </div>
        <StepList :steps="b.then ?? []" />
      </div>
    </template>

    <div class="step-branch">
      <div class="step-branch-label">else</div>
      <StepList :steps="s.else ?? []" />
    </div>
  </div>
</template>

<style>
/* ---------- 条件分叉 ---------- */

.step-if {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
  border: 1px solid rgba(192, 132, 252, 0.35);
  border-radius: 10px;
  padding: 10px;
  background: rgba(192, 132, 252, 0.05);
}

.step-if-head {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.step-if-kw {
  font-family: "Cascadia Code", Consolas, monospace;
  font-weight: 700;
  color: #c084fc;
  font-size: 13px;
}

.step-if-value {
  width: 160px;
  font-family: "Cascadia Code", Consolas, monospace;
}

.step-if-expected {
  width: 160px;
  font-family: "Cascadia Code", Consolas, monospace;
}

.step-branch {
  border-left: 2px solid rgba(192, 132, 252, 0.3);
  padding-left: 10px;
  margin-left: 4px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.step-branch-label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-family: "Cascadia Code", Consolas, monospace;
  font-size: 12px;
  font-weight: 700;
  color: #c084fc;
}

.step-branch-del {
  width: 22px;
  height: 22px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-3);
  cursor: pointer;
  line-height: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s, color 0.15s;
}

.step-branch-del:hover {
  background: rgba(248, 113, 113, 0.18);
  color: var(--red);
}
</style>
