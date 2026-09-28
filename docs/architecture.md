# 桌面版结构与验收

## SQLTunnel 对应关系

本次只读取参考工程 `/Users/nemo/dev/workspace/SQLTunnel`，没有修改参考项目。

| SQLTunnel | YOGO Pet |
| --- | --- |
| `electron/main.ts` | `src-tauri/src/main.rs`：生命周期、窗口、菜单、单实例 |
| `electron/service-runtime.ts` | `yogo-core/service_runtime.rs`：服务状态、业务服务生命周期、250 条日志、快照通知 |
| `electron/config-store.ts` | `yogo-core/config_store.rs`：偏好设置、验证、原子写入 |
| `src/gateway-service.ts` | `yogo-core/device_service.rs`：硬件业务和资源清理 |
| `shared/desktop.ts` | Rust 序列化的 DesktopSnapshot / Preferences / TaskSnapshot |
| `desktop/src/App.tsx` | `desktop/index.html + app.js` 和独立 `settings.html + settings.js` |

UI 使用 SQLTunnel 的系统字体、浅灰底、蓝色操作色、紧凑服务栏、状态／日志标签、圆角分组、底部设置入口和独立设置侧栏。主窗口精简至 480px，测试图案入口默认折叠；移除重复设备卡片、开发说明与关于页。

## 数据流

```text
Codex Hooks stdin
  → yogo-pet-hook：白名单事件，过滤正文/参数
  → events/*.json：私有权限、临时文件写完后原子改名
  → ServiceRuntime：200ms 消费，最多一批 500 条
  → TaskStates：按任务、轮次、工具调用聚合
  → DeviceService：状态变化才更新 6×6 RGB 图案
  → 专有 HID 接口 → 2.4GHz 接收器 → 点阵屏

ServiceRuntime → DesktopSnapshot → snapshot-changed → 两个 UI 窗口
UI → Tauri commands → 串行工作队列 → ServiceRuntime
```

原生服务没有 HTTP 控制端口。HID 所有读写和慢操作由单一工作线程负责，前端命令使用阻塞任务池等待结果，不阻塞窗口。备份文件在修改设备前同步写入。配置保存失败只反馈设置错误，不把仍正常运行的设备误标为断开。

## 设备边界

只允许 VID 0x373B、PID 0x11FF、usagePage 0xFF60、usage 0x61。不会打开普通键盘/鼠标接口。macOS HID 使用共享设备模式。

写入原灯效配置需要完整 64 字节的 24/24/16 三段提交。恢复时只替换点阵的 9 个字段，保留此时其他配置。每次模式更改和恢复均回读比较。自定义帧协议按原型逐字节比对。

退出分两阶段：先请求工作线程恢复，再允许 Tauri 退出。macOS 使用自定义退出菜单代替直接调用 Cocoa terminate 的预定义 Quit；另在 RunEvent::Exit 做同步恢复兜底。强制杀进程或断电无法保证当场恢复，依靠下次启动时的磁盘备份恢复。

## 已完成验证（2026-09-24）

- Rust 7 项核心测试：协议与原 JS 全部图案/数据包一致、恢复字段范围、多工具计数、旧轮次/完成后迟到事件、多任务优先级、并发原子事件和隐私、单实例/设置/事件消费。
- Rust 1 项独立进程测试：真实 hook 二进制 stdin→事件文件，过滤正文与工具参数，坏 JSON 仍成功返回 `{}`。
- 原 Node 原型 8 项回归全部通过（HTTP 集成测试需允许本机绑定端口）。
- Tauri macOS Apple Silicon release 构建、应用启动、原生窗口截图检查。
- 真实接收器连接，原配置备份、进入自定义模式和回读验证成功。
- 原生 helper 发送测试开始/执行事件，桌面显示“正在执行、1 项任务”，HID 写帧没有错误。
- 意外退出遗留备份恢复后重连成功。
- 修复后 ⌘Q 恢复成功，回读校验通过后备份被删除。
- 最终分发目录中的应用启动、设置 8→10→8 保存、原生插件导出及导出插件结构验证通过。
- 最终主窗口已连接接收器，所有主要控件可见。

用户先前已确认 Node 原型的思考、执行、完成图标在物理屏幕显示正确；桌面版迁移后具体视觉显示仍应由用户观察设备验收，不能由软件写入成功替代。

## 分发范围

当前产物仅 macOS arm64 预览包，没有正式 Developer ID 签名/公证。Windows、Intel Mac、Linux 需要各平台构建、权限与设备实测，不能把当前 arm64 hook 直接用于其他平台。插件更新后的 Hooks 信任由 Codex 管理，程序不绕过信任检查。
