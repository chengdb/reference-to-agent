import { nextTick, onMounted, onUnmounted, reactive, ref } from "vue";

/**
 * 下拉/选择类组件共用的打开状态、定位与关闭逻辑：
 * 打开时在 trigger 下方对齐弹出（下方空间不足则改在上方），
 * 点击外部、Esc 或滚动到触发元素时关闭。
 */
export function useDropdown(width = "124px") {
  const open = ref(false);
  const trigger = ref<HTMLElement | null>(null);
  const panel = ref<HTMLElement | null>(null);
  const pos = reactive<{ top: string; bottom: string; left: string; width: string }>({
    top: "0px",
    bottom: "",
    left: "0px",
    width,
  });

  async function toggle() {
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

  function close() {
    open.value = false;
  }

  function onDocClick(e: MouseEvent) {
    if (trigger.value && !trigger.value.contains(e.target as Node)) close();
  }

  function onScroll(e: Event) {
    if (!open.value || !trigger.value) return;
    const t = e.target as Node;
    if (
      t === document ||
      (t.nodeType === 1 && (t as HTMLElement).contains(trigger.value))
    ) {
      close();
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") close();
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

  return { open, trigger, panel, pos, toggle, close };
}
