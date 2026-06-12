use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ops::Range,
    rc::Rc,
    sync::mpsc,
    time::{Duration, Instant},
};

use gpui::{
    AppContext as _, Bounds, Context, Entity, FocusHandle, Pixels, Point,
    SharedString, Size, UniformListScrollHandle, Window, point,
    px, size,
};
use gpui_component::{
    Theme, ThemeMode, ThemeRegistry,
    input::{InputEvent, InputState},
    resizable::ResizableState,
    scroll::ScrollbarHandle,
};
use rust_i18n::t;
use tokio::runtime::Runtime;

use crate::{
    config::{AuthMethod, ConfigStore},
    sftp::SftpHandle,
    system::{SystemSampler, SystemSnapshot},
    terminal::{self, BackendEvent, TabKind, TerminalTab},
    ssh_terminal,
};

pub(crate) struct TerminalScrollbarState {
    line_height: Pixels,
    total_lines: usize,
    viewport_lines: usize,
    display_offset: usize,
}

#[derive(Clone, Default)]
pub(crate) struct TerminalScrollbarHandle {
    state: Rc<RefCell<Option<TerminalScrollbarState>>>,
    pub(crate) future_display_offset: Rc<Cell<Option<usize>>>,
}

impl TerminalScrollbarHandle {
    pub(crate) fn update(&self, snapshot: &terminal::RenderSnapshot, line_height: Pixels) {
        self.state.replace(Some(TerminalScrollbarState {
            line_height,
            total_lines: snapshot.history_size + snapshot.rows,
            viewport_lines: snapshot.rows,
            display_offset: snapshot.display_offset,
        }));
    }
}

impl ScrollbarHandle for TerminalScrollbarHandle {
    fn offset(&self) -> Point<Pixels> {
        let state_ref = self.state.borrow();
        let Some(state) = state_ref.as_ref() else {
            return point(px(0.), px(0.));
        };
        let scroll_offset = state
            .total_lines
            .saturating_sub(state.viewport_lines)
            .saturating_sub(state.display_offset);
        point(px(0.), -(scroll_offset as f32 * state.line_height))
    }

    fn set_offset(&self, offset: Point<Pixels>) {
        let state_ref = self.state.borrow();
        let Some(state) = state_ref.as_ref() else {
            return;
        };
        let offset_delta = (offset.y / state.line_height).round() as i32;
        let max_offset = state.total_lines.saturating_sub(state.viewport_lines);
        let display_offset = (max_offset as i32 + offset_delta).clamp(0, max_offset as i32);
        self.future_display_offset
            .set(Some(display_offset as usize));
    }

    fn content_size(&self) -> Size<Pixels> {
        let state_ref = self.state.borrow();
        let Some(state) = state_ref.as_ref() else {
            return size(px(0.), px(0.));
        };
        size(
            px(0.),
            state.total_lines.max(state.viewport_lines) as f32 * state.line_height,
        )
    }
}



pub(crate) struct Ashell {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) selector_focus_handle: FocusHandle,
    pub(crate) host_input: Entity<InputState>,
    pub(crate) session_name_input: Entity<InputState>,
    pub(crate) port_input: Entity<InputState>,
    pub(crate) user_input: Entity<InputState>,
    pub(crate) password_input: Entity<InputState>,
    pub(crate) key_path_input: Entity<InputState>,
    pub(crate) key_inline_input: Entity<InputState>,
    pub(crate) sftp_path_input: Entity<InputState>,
    pub(crate) ssh_auth_method: AuthMethod,
    pub(crate) editing_session_id: Option<String>,
    pub(crate) follow_system_theme: bool,
    pub(crate) theme_mode: ThemeMode,
    pub(crate) light_theme_name: SharedString,
    pub(crate) dark_theme_name: SharedString,
    pub(crate) ui_font_size: f32,
    pub(crate) terminal_font_size: f32,
    pub(crate) ui_font_family: SharedString,
    pub(crate) terminal_font_family: SharedString,
    pub(crate) tabs: Vec<TerminalTab>,
    pub(crate) sftp_handles: HashMap<String, SftpHandle>,
    pub(crate) active_tab: Option<String>,
    pub(crate) selector_selection: usize,
    pub(crate) workspace_panels: Entity<ResizableState>,
    pub(crate) body_panels: Entity<ResizableState>,
    pub(crate) terminal_scrollbar: TerminalScrollbarHandle,
    pub(crate) remote_files_scroll_handle: UniformListScrollHandle,
    pub(crate) tabs_scroll_handle: gpui::ScrollHandle,
    pub(crate) selector_scroll_handle: gpui::ScrollHandle,
    pub(crate) saved_scroll_handle: gpui::ScrollHandle,
    pub(crate) connection_progress: Option<ConnectionProgress>,
    pub(crate) pending_sftp_path_sync: Option<String>,
    pub(crate) sftp_context_menu: Option<SftpContextMenuState>,
    pub(crate) sftp_creating_folder: bool,
    pub(crate) sftp_new_folder_input: Entity<InputState>,
    pub(crate) sftp_delete_scroll_handle: gpui::ScrollHandle,
    pub(crate) show_hidden_files: bool,
    pub(crate) transfers: Vec<crate::terminal::Transfer>,
    pub(crate) show_transfers_dialog: bool,
    pub(crate) system_status: Option<SharedString>,
    pub(crate) terminal_bounds: Option<Bounds<Pixels>>,
    pub(crate) terminal_selecting: bool,
    pub(crate) terminal_marked_text: Option<String>,
    pub(crate) status: SharedString,
    pub(crate) config: ConfigStore,
    pub(crate) system_sampler: SystemSampler,
    pub(crate) system: SystemSnapshot,
    pub(crate) cpu_history: Vec<f32>,
    pub(crate) net_rx_history: Vec<f32>,
    pub(crate) net_tx_history: Vec<f32>,
    pub(crate) last_system_sample: Instant,
    pub(crate) last_theme_sync: Instant,

    pub(crate) remote_sample_in_flight: bool,
    pub(crate) runtime: Runtime,
    pub(crate) events_rx: mpsc::Receiver<BackendEvent>,
    pub(crate) events_tx: mpsc::Sender<BackendEvent>,
    pub(crate) _subscriptions: Vec<gpui::Subscription>,
}

#[derive(Clone)]
pub(crate) enum SelectorEntry {
    Local,
    NewSsh,
    Saved(String),
}

#[derive(Clone)]
pub(crate) struct ConnectionProgress {
    pub(crate) tab_id: String,
    pub(crate) title: SharedString,
    pub(crate) lines: Vec<SharedString>,
    pub(crate) failed: bool,
}

#[derive(Clone)]
pub(crate) struct SftpContextMenuState {
    pub(crate) remote_path: String,
    pub(crate) is_dir: bool,
    pub(crate) position: Point<Pixels>,
}

impl Ashell {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let host_input = cx.new(|cx| InputState::new(window, cx).placeholder(t!("host")));
        let session_name_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("name (optional)"));
        let port_input = cx.new(|cx| InputState::new(window, cx).default_value("22"));
        let user_input = cx.new(|cx| InputState::new(window, cx).default_value("root"));
        let password_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("password"))
                .masked(true)
        });
        let key_path_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("~/.ssh/id_ed25519"));
        let key_inline_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(5)
                .placeholder("-----BEGIN OPENSSH PRIVATE KEY-----")
        });
        let sftp_path_input = cx.new(|cx| InputState::new(window, cx).default_value("/"));
        let sftp_new_folder_input = cx.new(|cx| InputState::new(window, cx).placeholder(t!("new_folder").to_string()));

        let _subscriptions = vec![
            cx.subscribe_in(&host_input, window, Self::on_input_event),
            cx.subscribe_in(&session_name_input, window, Self::on_input_event),
            cx.subscribe_in(&port_input, window, Self::on_input_event),
            cx.subscribe_in(&user_input, window, Self::on_input_event),
            cx.subscribe_in(&password_input, window, Self::on_input_event),
            cx.subscribe_in(&key_path_input, window, Self::on_input_event),
            cx.subscribe_in(&key_inline_input, window, Self::on_input_event),
            cx.subscribe_in(&sftp_path_input, window, Self::on_input_event),
            cx.subscribe_in(&sftp_new_folder_input, window, Self::on_input_event),
        ];

        let (events_tx, events_rx) = mpsc::channel();
        let workspace_panels = cx.new(|_| ResizableState::default());
        let body_panels = cx.new(|_| ResizableState::default());
        let mut system_sampler = SystemSampler::new();
        let system = system_sampler.sample();
        let default_light_theme_name = ThemeRegistry::global(cx).default_light_theme().name.clone();
        let default_dark_theme_name = ThemeRegistry::global(cx).default_dark_theme().name.clone();
        let config = ConfigStore::load().unwrap_or_else(|err| {
            tracing::warn!("failed to load config: {err:#}");
            ConfigStore::in_memory()
        });
        let follow_system_theme =
            if config.light_theme_name().is_empty() && config.dark_theme_name().is_empty() {
                true
            } else {
                config.follow_system_theme()
            };

        let theme_mode = match config.theme_mode() {
            "light" => ThemeMode::Light,
            "dark" => ThemeMode::Dark,
            _ => ThemeMode::Light,
        };
        let light_theme_name = if config.light_theme_name().is_empty() {
            default_light_theme_name
        } else {
            config.light_theme_name().into()
        };
        let dark_theme_name = if config.dark_theme_name().is_empty() {
            default_dark_theme_name
        } else {
            config.dark_theme_name().into()
        };

        let configured_locale = config.locale();
        let mut active_locale = configured_locale.to_string();
        if active_locale == "system" {
            active_locale = sys_locale::get_locale().unwrap_or_else(|| "en".to_string());
            if active_locale.starts_with("zh") {
                active_locale = "zh-CN".to_string();
            } else {
                active_locale = "en".to_string();
            }
        }
        rust_i18n::set_locale(&active_locale);
        gpui_component::set_locale(&active_locale);
        let ui_font_family: SharedString = config.ui_font_family().into();
        let terminal_font_family: SharedString = config.terminal_font_family().into();
        let mut this = Self {
            focus_handle: cx.focus_handle(),
            selector_focus_handle: cx.focus_handle(),
            host_input,
            session_name_input,
            port_input,
            user_input,
            password_input,
            key_path_input,
            key_inline_input,
            sftp_path_input,
            ssh_auth_method: AuthMethod::Password,
            editing_session_id: None,
            follow_system_theme,
            theme_mode,
            light_theme_name,
            dark_theme_name,
            ui_font_size: config.ui_font_size(),
            terminal_font_size: config.terminal_font_size(),
            ui_font_family,
            terminal_font_family,
            tabs: Vec::new(),
            sftp_handles: HashMap::new(),
            active_tab: None,
            selector_selection: 0,
            workspace_panels,
            body_panels,
            terminal_scrollbar: TerminalScrollbarHandle::default(),
            remote_files_scroll_handle: UniformListScrollHandle::new(),
            tabs_scroll_handle: gpui::ScrollHandle::new(),
            selector_scroll_handle: gpui::ScrollHandle::new(),
            saved_scroll_handle: gpui::ScrollHandle::new(),
            connection_progress: None,
            pending_sftp_path_sync: Some("/".into()),
            sftp_context_menu: None,
            sftp_creating_folder: false,
            sftp_new_folder_input,
            sftp_delete_scroll_handle: gpui::ScrollHandle::new(),
            show_hidden_files: config.show_hidden_files(),
            transfers: config.transfers(),
            show_transfers_dialog: false,
            system_status: None,
            terminal_bounds: None,
            terminal_selecting: false,
            terminal_marked_text: None,
            status: "ready".into(),
            config,
            system_sampler,
            system,
            cpu_history: Vec::with_capacity(20),
            net_rx_history: Vec::with_capacity(20),
            net_tx_history: Vec::with_capacity(20),
            last_system_sample: Instant::now(),
            last_theme_sync: Instant::now(),

            remote_sample_in_flight: false,
            runtime: Runtime::new().expect("create tokio runtime"),
            events_rx,
            events_tx,
            _subscriptions,
        };

        this.apply_theme_preferences(window, cx);
        // this.open_local(cx);
        this.start_event_pump(cx);
        this
    }

    pub(crate) fn on_input_event(
        &mut self,
        input: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if input == &self.sftp_path_input {
            if let InputEvent::PressEnter { .. } = event {
                let path = self
                    .sftp_path_input
                    .read(cx)
                    .text()
                    .to_string()
                    .trim()
                    .to_string();
                self.navigate_sftp(if path.is_empty() { "/".into() } else { path }, cx);
                window.prevent_default();
                cx.stop_propagation();
            }
        } else if input == &self.sftp_new_folder_input {
            match event {
                InputEvent::PressEnter { .. } => {
                    let name = self.sftp_new_folder_input.read(cx).text().to_string();
                    if !name.is_empty() {
                        let base_path = self.sftp_path_input.read(cx).text().to_string();
                        let path = crate::sftp::join_remote(&base_path, &name);
                        if let Some(id) = self.active_tab.clone() {
                            if let Some(handle) = self.sftp_handles.get(&id) {
                                let _ = handle.commands.send(crate::sftp::SftpCommand::CreateDir(path));
                            }
                        }
                    }
                    self.sftp_creating_folder = false;
                    window.prevent_default();
                    cx.stop_propagation();
                }
                InputEvent::Blur => {
                    self.sftp_creating_folder = false;
                }
                _ => {}
            }
        }
        cx.notify();
    }

    pub(crate) fn start_event_pump(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                if this
                    .update(cx, |this, cx| {
                        this.drain_backend_events();
                        this.sample_system_if_due();
                        this.sync_theme_if_due(cx);
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    pub(crate) fn drain_backend_events(&mut self) {
        let mut transfers_changed = false;
        while let Ok(event) = self.events_rx.try_recv() {
            match event {
                BackendEvent::Output { tab_id, bytes } => {
                    if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                        tab.feed(&bytes);
                    }
                }
                BackendEvent::Status { tab_id, text } => {
                    if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                        tab.status = text.clone();
                    }
                    if let Some(progress) = self.connection_progress.as_mut() {
                        if progress.tab_id == tab_id {
                            progress.lines.push(text.clone().into());
                        }
                    }
                    self.status = text.into();
                }
                BackendEvent::Connected { tab_id } => {
                    if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                        tab.connected = true;
                    }
                    self.request_active_system_snapshot();
                    if self
                        .connection_progress
                        .as_ref()
                        .is_some_and(|progress| progress.tab_id == tab_id)
                    {
                        self.connection_progress = None;
                    }
                }
                BackendEvent::SftpEntries {
                    tab_id,
                    path,
                    entries,
                } => {
                    if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                        if let Some(sftp) = tab.sftp.as_mut() {
                            sftp.current_path = path;
                            sftp.entries = entries;
                            if self.active_tab.as_deref() == Some(tab_id.as_str()) {
                                self.pending_sftp_path_sync = Some(sftp.current_path.clone());
                            }
                        }
                    }
                }
                BackendEvent::SftpPreview { tab_id, preview } => {
                    if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                        if let Some(sftp) = tab.sftp.as_mut() {
                            sftp.selected_path = Some(preview.path.clone());
                            sftp.preview = Some(preview);
                        }
                    }
                }
                BackendEvent::SftpStatus { tab_id, text } => {
                    if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                        if let Some(sftp) = tab.sftp.as_mut() {
                            sftp.status = text.clone();
                        }
                    }
                    self.status = text.into();
                }
                BackendEvent::RemoteSystem { tab_id, snapshot } => {
                    self.remote_sample_in_flight = false;
                    if self.active_tab.as_deref() == Some(tab_id.as_str()) {
                        self.system_status = None;
                        self.system = snapshot.clone();
                        self.cpu_history.push(snapshot.cpu_percent);
                        if self.cpu_history.len() > 20 {
                            self.cpu_history.remove(0);
                        }
                        self.net_rx_history.push(snapshot.net_rx_rate as f32);
                        if self.net_rx_history.len() > 20 {
                            self.net_rx_history.remove(0);
                        }
                        self.net_tx_history.push(snapshot.net_tx_rate as f32);
                        if self.net_tx_history.len() > 20 {
                            self.net_tx_history.remove(0);
                        }
                    }
                }
                BackendEvent::RemoteSystemUnavailable { tab_id, reason } => {
                    self.remote_sample_in_flight = false;
                    if self.active_tab.as_deref() == Some(tab_id.as_str()) {
                        self.system_status = Some(reason.clone().into());
                        self.status = reason.into();
                    }
                }
                BackendEvent::Closed { tab_id, reason } => {
                    self.remote_sample_in_flight = false;
                    let mut tab_title = None;
                    let mut session_label = None;
                    if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                        tab.connected = false;
                        tab.status = reason.clone();
                        tab_title = Some(tab.title.clone());
                        session_label = tab.session.as_ref().map(|session| {
                            format!("{}@{}:{}", session.user, session.host, session.port)
                        });
                    }
                    if self.active_tab.as_deref() == Some(tab_id.as_str()) {
                        self.system_status = Some(reason.clone().into());
                    }
                    let is_graceful_exit = reason == "Terminal process exited"
                        || reason == "SSH session exited";
                    if let Some(progress) = self.connection_progress.as_mut() {
                        if progress.tab_id == tab_id {
                            progress.lines.push(reason.clone().into());
                            let _ = session_label;
                            let _ = tab_title;
                            progress.title = t!("connection_failed").into();
                            progress.failed = true;
                        }
                    } else if let Some(_) = session_label {
                        if !is_graceful_exit {
                            self.connection_progress = Some(ConnectionProgress {
                                tab_id: tab_id.clone(),
                                title: t!("connection_failed").into(),
                                lines: vec![reason.clone().into()],
                                failed: true,
                            });
                        }
                    }
                    self.status = reason.into();
                }
                BackendEvent::TransferProgress {
                    tab_id: _,
                    id,
                    transferred,
                    total,
                    state,
                } => {
                    if let Some(t) = self.transfers.iter_mut().find(|t| t.info.id == id) {
                        t.transferred = transferred;
                        if let Some(total) = total {
                            t.total = Some(total);
                        }
                        t.state = state;
                        transfers_changed = true;
                    }
                }
                BackendEvent::TransferStarted { tab_id, info } => {
                    let tab_title = self
                        .tabs
                        .iter()
                        .find(|t| t.id == tab_id)
                        .map(|t| t.title.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    self.transfers.insert(
                        0,
                        crate::terminal::Transfer {
                            tab_id,
                            tab_title,
                            info,
                            transferred: 0,
                            total: None,
                            state: crate::terminal::TransferState::Running,
                        },
                    );
                    transfers_changed = true;
                }
                BackendEvent::SftpHome { tab_id, home } => {
                    if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                        if let Some(sftp) = tab.sftp.as_mut() {
                            sftp.home_dir = home;
                        }
                    }
                }
                BackendEvent::TerminalTitleChanged { tab_id, title } => {
                    if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                        tab.title = title.clone();
                    }
                }
            }
        }
        if transfers_changed {
            self.config.set_transfers(self.transfers.clone());
        }
    }

    pub(crate) fn sample_system_if_due(&mut self) {
        if self.last_system_sample.elapsed() >= SystemSampler::interval() {
            self.last_system_sample = Instant::now();
            // When an SSH tab is active with remote data flowing, don't push
            // local machine data into history. Instead trigger a remote fetch
            // so both the current values and history reflect the remote server.
            if matches!(self.active_kind(), Some(TabKind::Ssh)) && self.system_status.is_none() {
                self.request_active_system_snapshot();
                return;
            }
            let snapshot = self.system_sampler.sample();
            let cpu_usage = snapshot.cpu_percent;
            self.cpu_history.push(cpu_usage);
            if self.cpu_history.len() > 20 {
                self.cpu_history.remove(0);
            }
            self.net_rx_history.push(snapshot.net_rx_rate as f32);
            if self.net_rx_history.len() > 20 {
                self.net_rx_history.remove(0);
            }
            self.net_tx_history.push(snapshot.net_tx_rate as f32);
            if self.net_tx_history.len() > 20 {
                self.net_tx_history.remove(0);
            }
            self.system = snapshot;
        }
    }

    pub(crate) fn sync_theme_if_due(&mut self, cx: &mut Context<Self>) {
        if self.follow_system_theme && self.last_theme_sync.elapsed() >= Duration::from_secs(1) {
            self.last_theme_sync = Instant::now();
            Theme::sync_system_appearance(None, cx);
            cx.refresh_windows();
        }
    }

    pub(crate) fn request_active_system_snapshot(&mut self) {
        if let Some((tab_id, session)) = self.active_ssh_session() {
            if self.remote_sample_in_flight {
                return;
            }
            self.remote_sample_in_flight = true;
            let events = self.events_tx.clone();
            self.runtime.spawn(async move {
                match ssh_terminal::sample_remote_system(session).await {
                    Ok(snapshot) => {
                        let _ = events.send(BackendEvent::RemoteSystem { tab_id, snapshot });
                    }
                    Err(err) => {
                        let _ = events.send(BackendEvent::RemoteSystemUnavailable {
                            tab_id,
                            reason: format!("remote metrics unavailable: {err:#}"),
                        });
                    }
                }
            });
        }
    }

    pub(crate) fn terminal_ime_bounds_for_range(
        &self,
        range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        cell_width: f32,
        line_height: f32,
    ) -> Option<Bounds<Pixels>> {
        let snapshot = self.active_snapshot()?;
        let cursor = snapshot.cursor?;
        let x = element_bounds.origin.x
            + px(cell_width) * cursor.col as f32
            + px(cell_width) * range_utf16.start as f32;
        let y = element_bounds.origin.y
            + px(line_height) * cursor.row as f32;
        Some(Bounds::new(
            point(x, y),
            size(
                px(cell_width),
                px(line_height),
            ),
        ))
    }
}
