# YogoSync

[English](README.md)

将 ChatGPT / Codex 的任务状态实时显示在 **YOGO 75 PRO 的 6 × 6 RGB 点阵屏**上。支持 **macOS Apple Silicon**，支持 **USB 有线与 2.4 GHz 接收器**，自动识别可用连接。

## 功能

- 同步待机、思考、执行、等待确认、完成和中断六种状态，多任务优先显示等待确认和执行状态。
- 自定义图标、颜色和逐帧动画，支持主题导入与导出。
- 黑、白、黄三色屏罩预览，菜单栏常驻。
- 设置支持跟随系统、简体中文和 English，默认跟随操作系统首选语言：中文使用简体中文，其他语言使用英文。

## 公开测试版 · macOS 安装

从 [GitHub Releases](https://github.com/NemoAlex/YogoSync/releases) 下载 **Apple Silicon DMG** 和对应的 `.sha256` 文件，要求 macOS 12 或更高版本。当前为公开测试版，使用临时签名，尚未完成 Apple Developer ID 签名与公证。

1. 打开 DMG，将 **YogoSync** 拖入 **应用程序**。
2. 先尝试打开一次。如果 macOS 因无法验证开发者或未公证而阻止打开，并且你信任此次下载，请进入 **系统设置 → 隐私与安全性 → 仍要打开**，再确认 **打开**。详见 [Apple 官方说明](https://support.apple.com/zh-cn/102445)。
3. 可在下载目录执行 `shasum -a 256 -c YogoSync-1.1.0-macOS-arm64.dmg.sha256`，核对文件完整性。

## 开始使用

连接数据线并切到有线模式，或插入接收器，然后打开 YogoSync，在首页点击 **配置 Hooks**，再按 **前往授权** 中的图示完成授权。Codex CLI 用户可通过 `/hooks` 授权。

授权后继续任意任务，收到事件即显示“已连接”；若一直等待事件，可重启客户端再试。已有 YogoSync 插件的用户，请先停用或卸载旧插件，再配置 Hooks。

## 说明

- 停止连接或退出时恢复原预设灯效；原自定义图案无法恢复，会切回 ATK 内置预设（模式 0）。
- 任务状态仅在本机处理，不保存对话内容，不上传任务数据。
