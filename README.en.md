[中文](README.md) | [English](README.en.md)

# ashell

![Preview](preview.png)

`ashell` is a modern, GPUI Component-based desktop terminal client written in Rust.

This project focuses on providing a high-performance and visually appealing shell workspace by combining local and remote environments with a rich set of built-in features. 

## 🚀 v0.3 Massive Update!

The brand-new v0.3 introduces comprehensive upgrades to both architecture and user experience:
- ✨ **Global Real-Time Font Sizing** (Thanks to [@ancion](https://github.com/ancion) for [PR #6](https://github.com/rust-kotlin/ashell/pull/6)): Customize your global UI fonts and terminal rendering fonts independently. Adjust text size with real-time scaling and automatic persistence.
- ✨ **Concurrent SFTP Transfers**: A dedicated Transfers dialog now supports highly optimized, concurrent file uploads and downloads with precise progress, speed, and ETA calculations.
- ✨ **Persistent Workspace State**: Your sidebar width adjustments, terminal themes, font settings, and file visibility preferences are completely restored upon your next launch!
- ✨ **Intelligent SSH Disconnect/Reconnect**: Greatly enhanced SSH connection resilience. Provides a beautiful and localized overlay for one-click re-connection on unexpected disconnections.
- ✨ **Hot-Swappable i18n System**: Deeply optimized internationalization. Switch instantly between English and Simplified Chinese without restarting the application.
- ✨ **Smart Right-Click Actions**: Added intelligent right-click copy and paste for the terminal, featuring a dedicated settings toggle and OS-native keyboard shortcut hints.

## Download

You can download the latest pre-compiled releases for macOS, Windows, and Linux from the [GitHub Releases page](https://github.com/rust-kotlin/ashell/releases/latest).

## Mac Installation Guide

After downloading, please unzip the file and move `ashell.app` to your **Applications** folder. 
Since the app uses ad-hoc signing, macOS may warn that the app is "damaged" upon first launch. If this happens, open Terminal and run the following command:

```bash
sudo xattr -cr /Applications/ashell.app
```

## Features

The current version provides a fully-featured GPUI-native workspace:

- **Local & Remote Sessions:** Open local terminal tabs or connect to remote servers via SSH.
- **Advanced SSH Authentication:** Supports both password-based and key-based (file path or inline) SSH connections.
- **Session Management:** Easily save, reopen, edit, and remove your SSH sessions.
- **SFTP Integration:** Built-in SFTP file manager to browse, upload, download, and manage remote files.
- **Robust Terminal Emulator:** Parses terminal output with `alacritty_terminal`, supporting rich ANSI color spans, fast rendering, and complete keyboard input forwarding.
- **System Telemetry:** Real-time visualization of CPU, memory, swap, network, and disk metrics in the left cockpit sidebar.
- **Theming System:** Switch between multiple GPUI Component themes directly from the top toolbar.
- **Embedded Fonts:** Uses embedded Maple Mono NF CN fonts out-of-the-box for excellent CJK character and Nerd Font icon support.

## Run

To run the application locally:

```bash
cargo run --release
```

## Package macOS App

```bash
./scripts/package-macos-app.sh
open target/release/ashell.app
```

The packaging script creates a standard `.app` bundle. It does not attach an entitlements file, and after signing, it verifies that `com.apple.security.app-sandbox` is not present (meaning it runs non-sandboxed).

## License

This project is licensed under the [GPL-3.0-or-later License](LICENSE).
