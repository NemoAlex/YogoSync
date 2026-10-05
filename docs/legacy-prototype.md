# Historical Node prototype

This document records the original prototype design. For current USB/receiver support and desktop setup, see the [project README](../README.md) and [prototype README](../plugins/yogosync/README.md).


在 YOGO 75 PRO 的 6×6 RGB 点阵上显示 Codex 任务状态。

第一版支持 **macOS + YOGO 75 PRO 2.4G 接收器**，VID `0x373B` / PID `0x11FF`。
其他键盘、有线模式和 Windows 尚未实机验证，不自动匹配。

## 启动

需要 Node.js 22 或更新版本。

```sh
npm ci
npm start
```

在 Chrome 打开 http://127.0.0.1:19775 ，点击「连接 YOGO 接收器」。
首次连接前关闭 ATK HUB 的设备设置页，避免两个程序同时下发配置。
可点击各状态按钮进行 5 秒预览。停止服务前点击「恢复原灯效」，或用 Ctrl+C 正常退出。

## Codex 插件

可分发插件位于 `plugins/yogosync`，默认自动发现 `hooks/hooks.json`。
将该目录安装到 Codex 后，需要在 Codex 的 Hooks 审核界面（CLI `/hooks`）信任该插件的事件脚本。
不要跳过 Hook 信任机制。安装后使用新的任务测试。

Hook 只依赖 Node.js 内置模块。硬件控制服务另需 `node-hid`，两者应运行在连接键盘的同一台电脑。

## 状态语义

| 事件 | 图标 |
|---|---|
| UserPromptSubmit | 思考 |
| PreToolUse | 执行 |
| PostToolUse | 思考 |
| PermissionRequest | 等待批准 |
| Stop | 本轮回复完成，8 秒后待机 |
| Interrupt | 已中断，8 秒后待机 |

多个任务同时运行时，等待批准优先，其次执行、思考；一项任务结束不会覆盖其他活动任务。
“完成”仅表示本轮回复结束，不保证整个需求成功。“思考”由生命周期推断。
等待普通文字回复、托管工具及异常崩溃的事件覆盖仍需实测。

## 当前边界

- 第一版只在状态变化时切换静态图标。没有确认点阵上传是否涉及非易失存储前，不进行逐帧高频写入。
- 图案为自行绘制的 6×6 小猫、忙碌、提醒、勾号、中断图标，不是官方 ChatGPT 宠物素材。
- 自定义图案不能读回，连接时保存内置预设模式 0 作为恢复目标；停止时恢复该预设。
- 模式设置需完整三包提交；每次均先读配置并保留全部未知字段，只改变点阵设置。
- 原灯效备份保存在 `~/.local/share/yogosync/device-backup.json`，正常断开恢复；异常退出后下次连接会先恢复。
- 设备断开后在页面手动重连。页面关闭不会停止后台服务。
- Hooks 必须经用户信任；尚未收到实际 Hooks 时页面会明确显示等待事件。
- 本地 HTTP 仅监听 127.0.0.1，控制操作检查 Host、Origin 和随机 token。
- 不读取对话文件、不保存提示词或工具参数；仅保存临时任务标识、事件类型及时间。

## 验证

```sh
npm test
npm run devices
node plugins/yogosync/scripts/cli.mjs inspect
node plugins/yogosync/scripts/cli.mjs demo
```

最后一个命令会进行约 8 秒的实机图标切换，并恢复原灯效。它的临时恢复备份在 `.runtime/`。

协议研究见 [protocol.md](protocol.md)。
