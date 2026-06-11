[中文](README.md) | [English](README.en.md)

# ashell

![Preview](preview.png)

`ashell` 是一款现代化的、基于 GPUI Component 构建的 Rust 桌面终端客户端。

该项目旨在通过结合本地和远程环境，并内置丰富的核心功能，提供一个高性能且美观的 Shell 工作区。

## 🚀 v0.3 版本重磅更新！

全新发布的 v0.3 迎来了架构和体验的全面升级：
- ✨ **全局实时字体设置与无级缩放**（感谢 [@ancion](https://github.com/ancion) 提供的 [PR #6](https://github.com/rust-kotlin/ashell/pull/6)）：支持自定义全局 UI 字体和终端渲染字体，支持实时字号调节缩放并自动记忆。
- ✨ **SFTP 多任务并发传输队列**：引入全新的独立传输对话框，支持多文件并发上传下载，提供精准的进度条、速率和剩余时间显示。
- ✨ **持久化工作区记忆**：您对侧边栏的尺寸拖拽、终端主题、字体、隐藏文件开关等一切设置都会在下次启动时原样恢复！
- ✨ **智能断线重连与状态感知**：全面增强的 SSH 网络异常处理机制，断线后直接提供美观的覆盖层用于一键重连或关闭。
- ✨ **多语言热切换增强**：深度优化的 i18n 系统，包含中文与英文在内的所有界面文案现在均支持热切换。
- ✨ **智能右键快捷操作**：新增终端右键智能复制与粘贴功能，提供专属设置项自由控制开关，并支持根据不同操作系统自适应显示原生快捷键提示。

## 下载

您可以从 [GitHub Releases 页面](https://github.com/rust-kotlin/ashell/releases/latest) 下载 macOS、Windows 和 Linux 版本的最新预编译程序。

## Mac 安装指南

下载并解压后，请先将 `ashell.app` 拖入或移动到 **应用程序 (Applications)** 目录。
由于应用采用本地签名，初次启动时如果系统提示“App 已损坏，无法打开”，请打开终端（Terminal）并执行以下命令：

```bash
sudo xattr -cr /Applications/ashell.app
```

## 功能特性

当前版本提供了一个功能完备的 GPUI 原生工作区：

- **本地与远程会话**：支持打开本地终端标签页或通过 SSH 连接到远程服务器。
- **高级 SSH 认证**：支持基于密码和基于密钥（文件路径或内联内容）的 SSH 连接方式。
- **会话管理**：可以轻松地保存、重新打开、编辑和删除您的 SSH 会话。
- **SFTP 集成**：内置 SFTP 文件管理器，可以浏览、上传、下载以及管理远程文件。
- **强大的终端模拟器**：使用 `alacritty_terminal` 解析终端输出，支持丰富的 ANSI 颜色代码、极速渲染和完整的键盘输入转发。
- **系统遥测**：在左侧边栏实时可视化显示 CPU、内存、Swap、网络和磁盘的使用指标。
- **主题系统**：支持直接从顶部工具栏切换多种 GPUI Component 颜色主题。
- **内置字体**：开箱即用，内置 Maple Mono NF CN 字体，提供卓越的中日韩（CJK）字符与 Nerd Font 图标支持。

## 运行

在本地运行该应用：

```bash
cargo run --release
```

## 打包 macOS 应用

```bash
./scripts/package-macos-app.sh
open target/release/ashell.app
```

该打包脚本会创建一个标准的 `.app` 应用程序包。它没有附加 entitlements 文件，并且在签名后会验证是否不存在 `com.apple.security.app-sandbox`（这意味着它在非沙盒模式下运行）。

## 协议

本项目采用 [GPL-3.0-or-later 协议](LICENSE) 开源。
