[中文](README.md) | [English](README.en.md)

# ashell

![Preview](preview.png)

`ashell` is a modern, GPUI Component-based desktop terminal client written in Rust.

This project focuses on providing a high-performance and visually appealing shell workspace by combining local and remote environments with a rich set of built-in features. 

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
