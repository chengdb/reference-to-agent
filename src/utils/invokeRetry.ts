import { invoke } from "@tauri-apps/api/core";

/**
 * 带重试的 Tauri 命令调用。
 *
 * ## 为什么需要重试（关键陷阱，改这里前务必读完）
 *
 * tauri.conf.json 里声明的窗口（main/menu/list）会在应用启动时随 WebView 一起创建，
 * 它们的 JS 会立刻执行（比如 onMounted 里调后端命令）。而后端的状态是通过
 * `setup` 回调里的 `app.manage(AppState {...})` 注册的——这两者之间存在**竞态**：
 * 窗口 JS 可能比 setup 完成得更早。
 *
 * 如果命令带 `State<AppState>` 参数（如 get_config / get_menu / get_recipes），
 * 在 `app.manage()` 还没执行时就调用，后端会报：
 *   `state not managed for field \`state\` on command ...`
 * 前端把这个异常当成普通 failure catch 掉 → 界面表现为“配置全空”。
 *
 * 注意：这类竞态是**间歇性**的（启动慢了一点或 setup 多做了点事就更容易复现），
 * 所以不能假设“之前能用现在就能用”。setup 最终一定会完成（应用能正常开着就是证明），
 * 因此这里用“短间隔重试等状态就绪”而不是直接放弃。
 *
 * ## 用法约定
 * - 仅用于**应用启动瞬间**可能触发的调用（onMounted / 首轮加载）。
 * - 普通运行中（例如用户点按钮时）的调用**不要**走这里——那时状态早已就绪，
 *   没必要为此掩盖真正的后端错误。
 * - 重试耗尽仍失败时，会把最后一次异常**原样抛出**，交由上层 catch 处理，
 *   不要在这里吞掉错误（否则用户看不到任何提示）。
 */
export async function invokeWithRetry<T>(
  cmd: string,
  args?: Record<string, unknown>,
  opts: { tries?: number; delayMs?: number } = {}
): Promise<T> {
  const tries = opts.tries ?? 5;
  const delayMs = opts.delayMs ?? 120;
  let lastErr: unknown;
  for (let i = 0; i < tries; i++) {
    try {
      return await invoke<T>(cmd, args);
    } catch (e) {
      lastErr = e;
      // 最后一次不再等待，直接把错误向上抛，让调用方 catch 展示提示。
      if (i === tries - 1) break;
      await new Promise((r) => setTimeout(r, delayMs));
    }
  }
  throw lastErr;
}
