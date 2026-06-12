#![windows_subsystem = "windows"]


use anyhow::{Context as _, Result};
use gpui::{App, AppContext as _, Bounds, KeyBinding, WindowOptions, point, px, size};
use gpui_component::{Root, Theme, ThemeRegistry};
use gpui_component_assets::Assets;

mod config;
mod local_terminal;
mod sftp;
mod ssh_terminal;
mod system;
mod terminal;
mod terminal_element;
mod terminal_input;
mod app;
mod session;
mod sftp_ops;
mod theme;
mod dialogs;
mod ui;

use config::ConfigStore;

rust_i18n::i18n!("locales", fallback = "en");

const DEFAULT_COLS: u16 = 100;
const DEFAULT_ROWS: u16 = 30;

const SIDEBAR_WIDTH: f32 = 306.0;
const TAB_BAR_HEIGHT: f32 = 52.0;
const TERMINAL_PADDING_X: f32 = 32.0;
const TERMINAL_PADDING_Y: f32 = 32.0;
const TERMINAL_KEY_CONTEXT: &str = "AshellTerminal";
const EMBEDDED_THEME_JSONS: &[&str] = &[
    include_str!("../assets/themes/matrix.json"),
    include_str!("../assets/themes/tokyonight.json"),
    include_str!("../assets/themes/gruvbox.json"),
    include_str!("../assets/themes/solarized.json"),
];

gpui::actions!(ashell_terminal, [TerminalTabKey, TerminalBacktabKey]);

pub(crate) use app::{
    Ashell, ConnectionProgress, SelectorEntry, SftpContextMenuState,
};

fn load_fonts(cx: &mut App) -> Result<()> {
    let regular =
        std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/MapleMono-NF-CN-Regular.ttf").as_slice());
    let bold = std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/MapleMono-NF-CN-Bold.ttf").as_slice());
    cx.text_system()
        .add_fonts(vec![regular, bold])
        .context("load Maple Mono NF CN fonts")?;
    // At startup we don't have a config yet, so pass the default UI font family.
    // It will be reapplied in Ashell::new() -> apply_theme_preferences.
    theme::set_theme_font_names(cx.global_mut::<Theme>(), ".SystemUIFont");
    Ok(())
}

fn load_embedded_themes(cx: &mut App) {
    let registry = ThemeRegistry::global_mut(cx);
    for theme_json in EMBEDDED_THEME_JSONS {
        if let Err(err) = registry.load_themes_from_str(theme_json) {
            tracing::warn!("failed to load embedded theme: {err:#}");
        }
    }
}

#[cfg(target_os = "macos")]
fn sync_macos_launch_environment() {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let Ok(output) = std::process::Command::new(&shell).args(["-l", "-c", "env -0"]).output() else {
        return;
    };
    if !output.status.success() {
        return;
    }

    for entry in output.stdout.split(|b| *b == 0) {
        if entry.is_empty() {
            continue;
        }
        let Some(eq) = entry.iter().position(|b| *b == b'=') else {
            continue;
        };
        let Ok(key) = std::str::from_utf8(&entry[..eq]) else {
            continue;
        };
        let Ok(value) = std::str::from_utf8(&entry[eq + 1..]) else {
            continue;
        };

        let should_import = matches!(
            key,
            "PATH"
                | "MANPATH"
                | "INFOPATH"
                | "LANG"
                | "LC_ALL"
                | "LC_CTYPE"
                | "SHELL"
                | "HOME"
                | "HOMEBREW_PREFIX"
                | "HOMEBREW_CELLAR"
                | "HOMEBREW_REPOSITORY"
        ) || key.starts_with("LC_");

        if should_import {
            unsafe {
                std::env::set_var(key, value);
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn sync_macos_launch_environment() {}

fn open_main_window(cx: &mut App) {
    let mut window_options = WindowOptions::default();

    #[cfg(not(target_os = "macos"))]
    if let Ok(img) = image::load_from_memory(include_bytes!("../assets/icons/ashell.png")) {
        window_options.icon = Some(std::sync::Arc::new(img.into_rgba8()));
    }

    let config = ConfigStore::load().unwrap_or_else(|_| ConfigStore::in_memory());
    if let Some(bounds) = config.window_bounds() {
        window_options.window_bounds = Some(match bounds {
            crate::config::SavedWindowBounds::Fullscreen {
                x,
                y,
                width,
                height,
            } => gpui::WindowBounds::Fullscreen(Bounds::new(
                point(px(*x), px(*y)),
                size(px(*width), px(*height)),
            )),
            crate::config::SavedWindowBounds::Maximized {
                x,
                y,
                width,
                height,
            } => gpui::WindowBounds::Maximized(Bounds::new(
                point(px(*x), px(*y)),
                size(px(*width), px(*height)),
            )),
            crate::config::SavedWindowBounds::Windowed {
                x,
                y,
                width,
                height,
            } => gpui::WindowBounds::Windowed(Bounds::new(
                point(px(*x), px(*y)),
                size(px(*width), px(*height)),
            )),
        });
    } else if let Some(display) = cx.displays().first().cloned() {
        let display_bounds = display.bounds();
        let width = display_bounds.size.width * 0.8;
        let height = display_bounds.size.height * 0.9;

        let x = display_bounds.origin.x + (display_bounds.size.width - width) / 2.0;

        #[cfg(target_os = "macos")]
        let y = display_bounds.origin.y;
        #[cfg(not(target_os = "macos"))]
        let y = display_bounds.origin.y + (display_bounds.size.height - height) / 2.0;

        window_options.window_bounds = Some(gpui::WindowBounds::Windowed(Bounds::new(
            point(x, y),
            size(width, height),
        )));
    }

    cx.open_window(window_options, |window, cx| {
        window.activate_window();
        window.set_window_title("ashell");
        Theme::sync_system_appearance(Some(window), cx);
        let view = cx.new(|cx| Ashell::new(window, cx));

        let workspace_panels_clone = view.read(cx).workspace_panels.clone();
        let body_panels_clone = view.read(cx).body_panels.clone();
        window.on_window_should_close(cx, move |window: &mut gpui::Window, cx: &mut gpui::App| {
            let mut config = ConfigStore::load().unwrap_or_else(|_| ConfigStore::in_memory());
            let current_bounds = window.window_bounds();
            let saved_bounds = match current_bounds {
                gpui::WindowBounds::Fullscreen(b) => crate::config::SavedWindowBounds::Fullscreen {
                    x: b.origin.x.into(),
                    y: b.origin.y.into(),
                    width: b.size.width.into(),
                    height: b.size.height.into(),
                },
                gpui::WindowBounds::Maximized(b) => crate::config::SavedWindowBounds::Maximized {
                    x: b.origin.x.into(),
                    y: b.origin.y.into(),
                    width: b.size.width.into(),
                    height: b.size.height.into(),
                },
                gpui::WindowBounds::Windowed(b) => crate::config::SavedWindowBounds::Windowed {
                    x: b.origin.x.into(),
                    y: b.origin.y.into(),
                    width: b.size.width.into(),
                    height: b.size.height.into(),
                },
            };
            let workspace_sizes: Vec<f32> = workspace_panels_clone
                .read(cx)
                .sizes()
                .iter()
                .map(|s| s.into())
                .collect();
            let body_sizes: Vec<f32> = body_panels_clone
                .read(cx)
                .sizes()
                .iter()
                .map(|s| s.into())
                .collect();
            config.set_layout_state(Some(saved_bounds), Some(workspace_sizes), Some(body_sizes));
            let _ = config.save();
            true
        });

        cx.new(|cx| Root::new(view, window, cx))
    })
    .expect("failed to open window");
}

fn main() {
    sync_macos_launch_environment();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    #[cfg(target_os = "macos")]
    let app = gpui_platform::application()
        .with_assets(Assets)
        .with_quit_mode(gpui::QuitMode::Explicit);

    #[cfg(not(target_os = "macos"))]
    let app = gpui_platform::application().with_assets(Assets);
    app.on_reopen(|cx| {
        if cx.windows().is_empty() {
            open_main_window(cx);
        }
    });
    app.run(move |cx| {
        gpui_component::init(cx);
        cx.bind_keys([
            KeyBinding::new("tab", TerminalTabKey, Some(TERMINAL_KEY_CONTEXT)),
            KeyBinding::new("shift-tab", TerminalBacktabKey, Some(TERMINAL_KEY_CONTEXT)),
        ]);
        load_embedded_themes(cx);
        if let Err(err) = load_fonts(cx) {
            tracing::warn!("failed to load embedded fonts: {err:#}");
        }
        open_main_window(cx);
    });
}

