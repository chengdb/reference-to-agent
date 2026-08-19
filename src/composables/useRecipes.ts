import { useConfigState } from "./useConfigState";
import { newRecipeTemplateSteps } from "../utils/steps";
import type { EditableRecipe, EditableStep } from "../types/editable";

const { cfg, selectedIndex, selectedStep } = useConfigState();

function selectRecipe(i: number) {
  selectedIndex.value = i;
  selectedStep.value = null;
}

function addRecipe() {
  cfg.recipes.push({
    name: `新配方 ${cfg.recipes.length + 1}`,
    steps: newRecipeTemplateSteps(),
  });
  selectedIndex.value = cfg.recipes.length - 1;
  selectedStep.value = null;
}

function uniqueRecipeName(base: string) {
  const names = new Set(cfg.recipes.map((r) => r.name));
  if (!names.has(base)) return base;
  let n = 2;
  while (names.has(`${base} ${n}`)) n++;
  return `${base} ${n}`;
}

/** 复制配方：深拷贝步骤（浅拷贝会让副本与原件共享嵌套 if 分支数组），取一个不重名的新名字，并选中副本。 */
function duplicateRecipe(i: number) {
  const src = cfg.recipes[i];
  if (!src) return;
  const copy: EditableRecipe = {
    name: uniqueRecipeName(`${src.name} 副本`),
    steps: JSON.parse(JSON.stringify(src.steps)) as EditableStep[],
  };
  cfg.recipes.splice(i + 1, 0, copy);
  selectedIndex.value = i + 1;
  selectedStep.value = null;
}

function removeRecipe(i: number) {
  if (!window.confirm(`删除配方“${cfg.recipes[i].name}”？`)) return;
  cfg.recipes.splice(i, 1);
  // 同步菜单绑定：删除的配方解绑，索引在其后的配方前移。
  cfg.menu.slots.forEach((s, si) => {
    if (!s) return;
    if (s.recipe === i) cfg.menu.slots[si] = null;
    else if (s.recipe != null && s.recipe > i) s.recipe--;
  });
  selectedStep.value = null;
  if (selectedIndex.value >= cfg.recipes.length) {
    selectedIndex.value = Math.max(0, cfg.recipes.length - 1);
  }
}

/* ---------- 步骤增删与排序（list 可为嵌套 if 分支） ---------- */

function addStepAt(list: EditableStep[], i: number) {
  list.splice(i, 0, { type: "wait", ms: 100 });
  selectedStep.value = list[i] ?? null;
}

function removeStep(list: EditableStep[], i: number) {
  list.splice(i, 1);
  selectedStep.value = null;
}

function moveStep(list: EditableStep[], i: number, dir: number) {
  const ni = i + dir;
  if (ni < 0 || ni >= list.length) return;
  const tmp = list[i];
  list[i] = list[ni];
  list[ni] = tmp;
  selectedStep.value = list[ni];
}

/** 配方与步骤的编辑操作。状态本体见 useConfigState。 */
export function useRecipes() {
  return {
    selectRecipe,
    addRecipe,
    duplicateRecipe,
    removeRecipe,
    addStepAt,
    removeStep,
    moveStep,
  };
}
