# Reference to Agent

Windows 桌面小工具：在 IDE 中选中代码（或复制文件路径）后，按全局快捷键在鼠标旁弹出菜单，一键把内容发送到目标 AI Agent（如 Claude Desktop）的聊天输入框并发送。

## 功能

- 全局快捷键（默认 `Ctrl+Alt+R`）在鼠标位置弹出悬浮菜单；再次按同一快捷键收起。
- 菜单项即「动作配方」：一个按顺序执行的步骤序列。
- 内置动作原语：注入组合键、切换/启动目标应用、等待、剪贴板、输入文本、运行命令。
- 常驻托盘；主窗口关闭后隐藏到托盘。
- 设置界面可视化编辑配方（增删配方、编辑步骤、热键录制），保存后实时应用；配置以 JSON 存于 `%APPDATA%\com.fengy.reference-to-agent\config.json`。

## 使用

1. 运行 `reference-to-agent.exe`（或 `npm run tauri dev` 开发模式）。
2. 在 VS Code / JetBrains 中选中代码。
3. 按 `Ctrl+Alt+R` → 菜单出现在鼠标旁 → 点击「发送选中代码到 Claude」。
4. 动作序列自动执行：复制选中内容 → 切到 Claude 窗口 → 粘贴 → 发送。

### 复制文件路径

- JetBrains 系：默认 `Ctrl+Shift+C` 即「复制引用」，无需配置。
- VS Code：默认没有复制路径快捷键，需要在 `keybindings.json` 给 `workbench.action.files.copyPathOfActiveFile` 绑定一个快捷键，然后修改对应配方的 `hotkey` 步骤。

## 配置

设置界面（托盘或菜单→设置）提供可视化编辑：

- **全局快捷键**：点击「录制」后直接按下组合键。
- **配方**：左侧列表可增删；选中后编辑名称与步骤。步骤可增删、上下调整顺序，每步选类型后按字段填参数；快捷键类步骤支持录制，「激活应用 / 聚焦应用」可通过「选择应用…」从本机已安装应用（开始菜单快捷方式 + 商店应用）中选取，并自动填入标题与 exe 路径（商店应用填入 `shell:AppsFolder\<AUMID>`，运行时按标题聚焦、未启动时按 AUMID 拉起）。
- 保存后立即生效（自动重新注册全局快捷键），无需重启。

步骤类型：

| type | 参数 | 说明 |
|---|---|---|
| `wait` | `ms` | 等待毫秒数 |
| `hotkey` | `keys` | 注入组合键，如 `Ctrl+Shift+C`、`Enter`、`Alt+Tab` |
| `activateApp` | `title`、`exe?` | 按标题模糊匹配激活窗口；未找到且给了 `exe` 时先启动该程序并等待窗口出现（商店应用传 `shell:AppsFolder\<AUMID>`） |
| `focusApp` | `title` | 聚焦已打开的应用窗口（按标题模糊匹配，不启动新进程） |
| `setClipboard` | `text` | 写入剪贴板 |
| `typeText` | `text` | 逐字输入文本（受输入法影响，慎用于中文） |
| `pasteText` | `text` | 写入剪贴板并粘贴（对中文/长文本可靠） |
| `runCommand` | `cmd`、`args` | 运行外部命令 |
| `click` | `title`、`x`、`y` | 在标题匹配的窗口内模拟鼠标左键点击，用于聚焦输入框等控件。`x`/`y` 各自独立定位：`x.base` 取 `left`/`right`，`y.base` 取 `top`/`bottom`（相对哪条边）；`x.value`/`y.value` 为偏移量，`x.unit`/`y.unit` 取 `percent`（百分比，0~1）或 `px`（固定像素）。沉底的输入框建议 `x={base:left,value:0.5,unit:percent}`、`y={base:bottom,value:0.08,unit:percent}`。可在设置界面用「拾取坐标」（按鼠标位置自动选基准边）与「测试点击」辅助 |
| `rollbackClipboard` | 无 | 把剪贴板恢复为配方执行前的原始内容（放在复制/粘贴类步骤之后，避免覆盖用户原有剪贴板） |

配置最终写入 `%APPDATA%\com.fengy.reference-to-agent\config.json`，结构即为上表对应的 JSON。

`activateApp` 的 `title` 按「包含」匹配窗口标题；给多个目标 agent 建配方时，各自指定自己的 `title` 即可。若目标应用未运行，配好 `exe` 全路径后可自动拉起。

## 开发

```bash
npm install
npm run tauri dev      # 开发模式
npm run tauri build    # 打包
```

技术栈：Tauri 2（Rust）+ Vue 3（Vite + TypeScript）。

注意：本机使用 MSYS2 mingw64（GNU）工具链编译，`npm run tauri` 已自动把 `C:\msys64\mingw64\bin` 注入 `PATH`（供 `windres`/链接器使用），直接运行即可，无需手动配置环境变量；`lib` 的 `crate-type` 只保留 `lib` 以规避 GNU 链接器导出符号问题。若改用其他机器，请按需调整 `package.json` 里的路径。

## 局限与注意

- 本质是跨应用 UI 自动化（注入按键 + 切窗），对目标应用窗口标题、输入框行为敏感；目标应用改版可能影响对应配方。
- 执行序列的时序（等待毫秒）可能需要按目标应用启动速度微调。
- 发送代码到外部 AI 服务属于数据外发，请确认内容允许。
- 目标 AI 应用若有官方 CLI（如 Claude Code），脚本直连更稳定；本工具面向「无 API 的桌面 GUI 应用」场景。
