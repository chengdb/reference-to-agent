import { ref } from "vue";

/** 配置页顶部轻提示（成功/错误），2.5 秒自动消失。 */
const toast = ref<{ type: "success" | "error"; msg: string } | null>(null);
let toastTimer: number | undefined;

function showToast(msg: string, type: "success" | "error" = "success") {
  toast.value = { type, msg };
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => (toast.value = null), 2500);
}

export function useToast() {
  return { toast, showToast };
}
