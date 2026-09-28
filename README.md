# YOGO Pet 1.0

让 YOGO 75 PRO 的 6 × 6 点阵屏随 Codex 任务变化。

桌面版使用 **Tauri 2 + Rust**，UI 与后端分层参考 SQLTunnel。运行不需要 Node.js、Python 或 Rust，也不需要保持 ATK 网页打开。当前已构建、连接实测的平台是 **macOS Apple Silicon**。

## 使用桌面版

1. 打开 `dist/YOGO-Pet-1.0.0-macOS-arm64.dmg`，将 **YOGO Pet.app** 拖到 Applications。
2. 插入 YOGO 75 PRO 的 2.4 GHz 接收器，打开 YOGO Pet，启用右上角服务开关。
3. 主界面显示“未安装插件”时，点击“安装插件”。软件会准备当前平台的原生插件并打开 Codex 插件页，在该页完成安装；若插件已停用，点击“前往启用”。已有个人市场条目会复用，不会覆盖或重复创建。
4. 查看 YOGO Pet 主界面的 Codex 状态；若显示“未授权”，点击“前往授权”查看应用内图示：可点击 Codex 对话框下方的 Hook 图标，或进入 Settings → Coding → Hooks，审阅并信任 YOGO Pet。返回后状态自动刷新。
5. 已授权后，在任意 Codex 任务中继续发消息验证联动，无需每次新建任务；若仍收不到事件，请重启 Codex 后继续原任务。思考／执行／等待确认／完成状态会切换到点阵屏。

主界面显示授权状态。只有已授权且本次运行收到任务事件时，才显示“已连接”；“已授权 · 等待事件”表示授权检查通过，尚未验证真实事件。检测通过本机 Codex 的只读接口完成，不会自动授予 Hook 信任。授权按钮只显示应用内说明，不跳转到 Codex。说明提供对话框 Hook 图标和 Settings → Coding → Hooks 两种入口；点击“我已授权，检查连接”会重新检测状态，不会直接标记授权成功。

插件定义变更后会显示“需要重新授权”，点击“前往授权”重新审阅。授权完成不会直接显示连接成功，必须收到真实任务事件。

手动安装备用：插件目录在安装包的 `Codex 插件/yogo-pet`。首次安装可在 Codex 中提供插件文件夹路径并说明：

> 请使用 plugin-creator 将这个 yogo-pet 插件安装到我的个人插件市场，保留原生 bin/yogo-pet-hook 和 hooks/hooks.json；不要创建另一份重复插件。安装后让我检查 Hooks 信任。

应用可直接运行，插件首次安装仍通过 Codex 完成，应用已提供安装入口和文件准备，最终安装与授权在 Codex 内完成。当前包未做 Apple Developer ID 签名与公证；正式对外发布前需完成签名、公证与下载分发。没有提供已验证的 Intel Mac 或 Windows 安装包。

## 窗口与交互

- 紧凑状态窗口：服务开关、主题选择、带横向棱格的点阵预览、状态／日志标签。
- 独立设置：打开应用时自动连接、完成图标停留时间。
- ⌘W 关闭当前窗口；主窗口关闭后常驻菜单栏。菜单退出和 ⌘Q 会先恢复原灯效，编辑未保存时会提示。
- 演示按钮显示 5 秒后回到自动跟随，不修改真实任务状态。
- 多任务按等待确认 → 执行 → 思考聚合；一个任务完成不会盖住另一个仍在运行的任务。
- 六种状态支持逐帧循环动画；主题编辑器支持绘制、自定义颜色、帧时长、保存与 JSON 导入／导出。单帧图案持续显示。

## 本地数据和恢复

默认数据目录：`~/.local/share/yogo-pet`，也可设置 `YOGO_PET_HOME`。

- `preferences.json`：桌面设置。
- `events/`：每次 Hooks 原子写入一个元数据事件，服务读取后删除。
- `device-backup.json`：接管前灯效备份，仅恢复并读回校验成功后删除。
- `desktop.lock`：操作系统文件锁，保证一个原生服务实例控制设备。
- `native-plugin/yogo-pet`：安装流程准备的当前平台插件。
- `themes.json`：自定义主题及当前选择。

Hooks 只写事件名、任务／轮次／工具调用标识和时间，不保存提示词或工具参数。任务日志仅在内存保留最近 250 条。应用不监听 HTTP 端口，不上传数据。

意外终止或拔出接收器后，备份会保留。重新插入同一接收器后连接，程序会先恢复上次备份，再开始控制；也可以关闭主界面的服务开关恢复灯效。若本来就是自定义图案且没有备份，需要先在 ATK 选择预设灯效，因为此协议还不能读回原自定义像素。

旧 Node 网页服务与桌面版使用同一事件目录。迁移时先正常结束旧服务；如果存在 `server.lock`，桌面版会阻止接管。不要在旧服务仍运行时删除锁文件。

## 开发

需要 Rust 工具链、Node.js（仅用于 Tauri 开发/打包工具）以及平台构建工具。

```sh
npm ci
npm run desktop:dev
npm run test:native
npm test
npm run desktop:build
```

桌面应用输出到 `target/release/bundle/macos/YOGO Pet.app`。`scripts/desktop.mjs` 先编译独立原生助手，再将其作为应用资源打包。macOS 安装包用 `scripts/package-macos.sh` 生成；传给其他人前应按正式签名流程处理。

## 项目结构

| 路径 | 职责 |
| --- | --- |
| `desktop/` | 状态和设置 UI，通过 Tauri 命令与事件收发快照 |
| `src-tauri/src/main.rs` | 应用生命周期、单实例、窗口、菜单栏、退出恢复 |
| `src-tauri/src/codex_connection.rs` | Codex 安装、启用与 Hook 信任状态的只读检测 |
| `src-tauri/src/plugin_setup.rs` | 准备个人市场插件并打开 Codex 安装页，不修改信任 |
| `src-tauri/src/commands.rs` | UI 命令边界、独立设置窗口、插件导出 |
| `crates/yogo-core/src/service_runtime.rs` | 串行设备工作线程、状态聚合、日志和快照 |
| `crates/yogo-core/src/device_service.rs` | HID 连接、备份、完整配置提交和恢复校验 |
| `crates/yogo-core/src/config_store.rs` | 设置校验和原子落盘 |
| `crates/yogo-core/src/task_states.rs` | 多任务与并行工具状态、旧轮次保护 |
| `crates/yogo-hook/` | 独立原生 Hooks 助手，异常不阻塞 Codex |
| `plugins/yogo-pet/` | 最初已验证的 Node 网页原型，保留用于协议比对 |

详细分层见 [architecture.md](docs/architecture.md)，设备协议见 [protocol.md](docs/protocol.md)，旧版原型见 [legacy-prototype.md](docs/legacy-prototype.md)。
