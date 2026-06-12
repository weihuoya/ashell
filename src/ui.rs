
use gpui::{
    Context, ElementId, Focusable as _, FontWeight, Hsla, InteractiveElement as _,
    IntoElement, MouseButton, MouseDownEvent,
    ParentElement as _, PathBuilder, Pixels, Render,
    StatefulInteractiveElement as _, Styled as _, Window,
    canvas, div, point, prelude::FluentBuilder as _, px, rems, uniform_list,
};
use gpui_component::{
    ActiveTheme, Disableable as _, ElementExt, IconName, Root, Sizable as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::Input,
    menu::{ContextMenuExt as _, PopupMenuItem},
    progress::Progress,
    resizable::{h_resizable, resizable_panel, v_resizable},
    scroll::{ScrollableElement as _, Scrollbar, ScrollbarShow},
    tab::{Tab, TabBar},
    v_flex,
};
use rust_i18n::t;

use crate::{
    Ashell, SIDEBAR_WIDTH, TERMINAL_KEY_CONTEXT,
    sftp_ops::is_editable_text_file,
    sftp::format_mtime,
    system::format_bytes,
    terminal::{self, TabKind},
    terminal_element,
};

impl Ashell {
    fn render_home_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .h_full()
            .items_center()
            .justify_center()
            .gap_4()
            .child(
                div()
                    .text_size(rems(2.333))
                    .font_weight(FontWeight::BOLD)
                    .child("Ashell"),
            )
            .child(
                div()
                    .text_size(rems(1.083))
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("open_local_or_ssh")),
            )
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        Button::new("home-open-local")
                            .primary()
                            .label(t!("local_terminal").to_string())
                            .on_click(cx.listener(|this, _, _, cx| this.open_local(cx))),
                    )
                    .child(
                        Button::new("home-open-session")
                            .ghost()
                            .label(t!("open_session").to_string())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.show_selector_dialog(window, cx)
                            })),
                    ),
            )
    }

    fn render_sftp_panel(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active_sftp = self.active_sftp();

        let header = h_flex()
            .flex_none()
            .h(px(34.))
            .items_center()
                    .gap_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tab_bar)
            .child(
                div()
                    .text_size(rems(1.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(cx.theme().primary)
                    .child(t!("remote_files")),
            )
            .child(div().flex_1())
            .when_some(active_sftp.clone(), |this, sftp| {
                let selected_entries = sftp.selected_entries.clone();
                this.child(
                    Button::new("sftp-refresh")
                        .ghost()
                        .small()
                        .icon(IconName::ArrowRight)
                        .label(t!("refresh").to_string())
                        .on_click(cx.listener(|this, _, _, cx| this.refresh_sftp(cx))),
                )
                .child(
                    Button::new("sftp-new-folder")
                        .ghost()
                        .small()
                        .icon(IconName::Folder)
                        .label(t!("new_folder").to_string())
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.sftp_creating_folder = true;
                            this.sftp_new_folder_input.update(cx, |input, cx| {
                                input.set_value("", window, cx);
                                input.focus_handle(cx).focus(window, cx);
                            });
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("sftp-delete-selected")
                        .ghost()
                        .small()
                        .icon(IconName::Close)
                        .label(if selected_entries.is_empty() {
                            t!("delete_selected").to_string()
                        } else {
                            format!(
                                "{} ({})",
                                t!("delete_selected").to_string(),
                                selected_entries.len()
                            )
                        })
                        .disabled(selected_entries.is_empty())
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.show_delete_confirm_dialog(window, cx);
                        })),
                )
                .child(
                    Button::new("sftp-upload-file")
                        .ghost()
                        .small()
                        .icon(IconName::Plus)
                        .label(t!("upload_file").to_string())
                        .on_click(
                            cx.listener(|this, _, window, cx| this.upload_sftp_files(window, cx)),
                        ),
                )
                .child(
                    Button::new("sftp-upload-folder")
                        .ghost()
                        .small()
                        .icon(IconName::Folder)
                        .label(t!("upload_folder").to_string())
                        .on_click(
                            cx.listener(|this, _, window, cx| this.upload_sftp_folder(window, cx)),
                        ),
                )
                .child(
                    Button::new("sftp-download-selected")
                        .ghost()
                        .small()
                        .icon(IconName::ArrowDown)
                        .label(if selected_entries.is_empty() {
                            t!("download").to_string()
                        } else {
                            t!("download_count", count = selected_entries.len()).to_string()
                        })
                        .disabled(selected_entries.is_empty())
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.download_selected_sftp_entries(window, cx);
                        })),
                )
                .child(
                    Checkbox::new("sftp-show-hidden")
                        .small()
                        .label(t!("hidden").to_string())
                        .checked(self.show_hidden_files)
                        .tab_stop(false)
                        .on_click(cx.listener(|this, checked, _, cx| {
                            this.show_hidden_files = *checked;
                            this.config.set_show_hidden_files(*checked);
                            let _ = this.config.save();
                            cx.notify();
                        })),
                )
            })
            .child(
                Button::new("open-transfers")
                    .ghost()
                    .small()
                    .icon(IconName::ArrowDown)
                    .label(t!("transfers").to_string())
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.show_transfers_dialog(window, cx);
                    })),
            )
            .child(
                Button::new("sftp-minimize-toggle")
                    .ghost()
                    .small()
                    .icon(if self.sftp_panel_minimized { IconName::ChevronUp } else { IconName::ChevronDown })
                    .on_click(cx.listener(|this, _, window, cx| {
                        let state = this.body_panels.clone();
                        let minimized = this.sftp_panel_minimized;
                        
                        if !minimized {
                            // Going to minimized: save the current size
                            let sizes = state.read(cx).sizes();
                            if sizes.len() > 1 {
                                this.prev_monitoring_size = Some(sizes[1]);
                            }
                            this.sftp_panel_minimized = true;
                        } else {
                            // Going to unminimized: restore the old size
                            this.sftp_panel_minimized = false;
                            let prev_size = this.prev_monitoring_size.unwrap_or(px(328.));
                            
                            cx.on_next_frame(window, move |_this: &mut crate::app::Ashell, window: &mut gpui::Window, cx: &mut gpui::Context<crate::app::Ashell>| {
                                cx.on_next_frame(window, move |this: &mut crate::app::Ashell, window: &mut gpui::Window, cx: &mut gpui::Context<crate::app::Ashell>| {
                                    this.body_panels.update(cx, |state, cx| {
                                        let sizes = state.sizes();
                                        let c_size_f32: f32 = sizes.iter().map(|s| s.as_f32()).sum();
                                        let c_size = px(c_size_f32);
                                        let target_p0 = c_size - prev_size;
                                        state.resize_panel(0, target_p0, window, cx);
                                    });
                                    cx.notify();
                                });
                            });
                        }
                        cx.notify();
                    })),
            );

        let Some(sftp) = active_sftp else {
            let mut panel = v_flex()
                .gap_0()
                .border_color(cx.theme().border)
                .bg(cx.theme().background);
            
            if !self.sftp_panel_minimized {
                panel = panel.size_full();
            } else {
                panel = panel.flex_none();
            }
            
            panel = panel.child(header);
            
            if !self.sftp_panel_minimized {
                panel = panel.child(
                    v_flex()
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .p_3()
                        .child(
                            div()
                                .text_size(rems(1.0))
                                .text_color(cx.theme().muted_foreground)
                                .child(t!("open_ssh_tab_sftp")),
                        ),
                );
            }
            return panel.into_any_element();
        };

        let selected_path = sftp.selected_path.clone();
        let entries = sftp
            .entries
            .clone()
            .into_iter()
            .filter(|entry| self.show_hidden_files || !entry.name.starts_with('.'))
            .collect::<Vec<_>>();
        let status = sftp.status.clone();
        let selected_entries = sftp.selected_entries.clone();
        let all_selected = !entries.is_empty()
            && entries
                .iter()
                .all(|e| selected_entries.contains(&e.full_path));
        let parent_path = Self::sftp_parent_path(&sftp.current_path);
        let view = cx.entity();
        let icon_col_width = px(14.);
        let size_col_width = px(96.);
        let modified_col_width = px(152.);

        let mut panel = v_flex()
            .gap_0()
            .border_color(cx.theme().border)
            .bg(cx.theme().background);

        if !self.sftp_panel_minimized {
            panel = panel.size_full();
        } else {
            panel = panel.flex_none();
        }

        panel = panel
            .on_drop(
                cx.listener(|this, paths: &gpui::ExternalPaths, _window, cx| {
                    let paths_to_upload: Vec<String> = paths
                        .0
                        .iter()
                        .map(|p| p.to_string_lossy().to_string())
                        .collect();
                    this.upload_sftp_files_batch(paths_to_upload, cx);
                }),
            )
            .child(header);

        if !self.sftp_panel_minimized {
            panel = panel
            .child(
                h_flex()
                    .h(px(36.))
                    .items_center()
                    .gap_2()
                    .px_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().muted)
                    .child(
                        Button::new("sftp-up")
                            .ghost()
                            .small()
                            .icon(IconName::ChevronUp)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.navigate_sftp(parent_path.clone(), cx);
                            })),
                    )
                    .child(Input::new(&self.sftp_path_input).flex_1().tab_index(0))
                    .child(div().flex_none()),
            )
            .child(
                h_flex()
                    .h(px(26.))
                    .px_3()
                    .items_center()
                    .gap_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().muted.opacity(0.8))
                    .child(
                        h_flex()
                            .w(px(24.))
                            .flex_none()
                            .items_center()
                            .justify_center()
                            .child(
                                Checkbox::new("sftp-select-all")
                                    .checked(all_selected)
                                    .on_click(cx.listener(move |this, checked, _, cx| {
                                        this.toggle_all_sftp_entries(*checked, cx);
                                    })),
                            ),
                    )
                    .child(
                        h_flex()
                            .flex_1()
                            .min_w(px(0.))
                            .items_center()
                            .gap_2()
                            .child(div().w(icon_col_width).flex_none())
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(rems(0.917))
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("name")),
                            ),
                    )
                    .child(
                        div()
                            .w(size_col_width)
                            .flex_none()
                            .text_size(rems(0.917))
                            .text_color(cx.theme().muted_foreground)
                            .child(t!("size")),
                    )
                    .child(
                        div()
                            .w(modified_col_width)
                            .flex_none()
                            .text_size(rems(0.917))
                            .text_color(cx.theme().muted_foreground)
                            .child(t!("modified")),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .relative()
                    .min_h(px(0.))
                    .child({
                        let entries = entries.clone();
                        let selected_entries = selected_entries.clone();
                        let selected_path = selected_path.clone();
                        let view = view.clone();
                        let theme = cx.theme().clone();
                        let icon_col_width = icon_col_width;
                        let size_col_width = size_col_width;
                        let modified_col_width = modified_col_width;
                        uniform_list(
                            "sftp-entries-list",
                            entries.len(),
                            move |range, window, _cx| {
                                range
                                    .into_iter()
                                    .filter_map(|ix| {
                                        let entry = entries.get(ix)?;
                                        let entry = entry.clone();
                                        let is_checked =
                                            selected_entries.contains(&entry.full_path);
                                        let is_selected = selected_path.as_deref()
                                            == Some(entry.full_path.as_str());
                                        let name_color = if entry.is_dir {
                                            theme.primary
                                        } else {
                                            theme.foreground
                                        };
                                        let bg = if is_selected {
                                            theme.secondary
                                        } else if ix % 2 == 0 {
                                            theme.background
                                        } else {
                                            theme.muted.opacity(0.5)
                                        };
                                        Some(
                                            h_flex()
                                                .w_full()
                                                .h(px(28.))
                                                .items_center()
                    .gap_2()
                                                .bg(bg)
                                                .hover(|style| style.bg(theme.muted.opacity(0.8)))
                                                .border_b_1()
                                                .border_color(theme.border.opacity(0.35))
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    window.listener_for(&view, {
                                                        let entry = entry.clone();
                                                        move |this, _, _, cx| {
                                                            this.dismiss_sftp_context_menu(cx);
                                                            this.select_sftp_entry(
                                                                entry.clone(),
                                                                cx,
                                                            );
                                                        }
                                                    }),
                                                )
                                                .on_mouse_down(
                                                    MouseButton::Right,
                                                    window.listener_for(&view, {
                                                        let entry = entry.clone();
                                                        let remote_path = entry.full_path.clone();
                                                        move |this, event: &MouseDownEvent, _, cx| {
                                                            this.mark_sftp_entry_selected(
                                                                &entry.full_path,
                                                                cx,
                                                            );
                                                            this.open_sftp_context_menu(
                                                                remote_path.clone(),
                                                                entry.is_dir,
                                                                event.position,
                                                                cx,
                                                            );
                                                        }
                                                    }),
                                                )
                                                .child(
                                                    h_flex()
                                                        .w(px(24.))
                                                        .flex_none()
                                                        .items_center()
                                                        .justify_center()
                                                        .on_mouse_down(
                                                            MouseButton::Left,
                                                            |_, _, cx| cx.stop_propagation(),
                                                        )
                                                        .on_mouse_down(
                                                            MouseButton::Right,
                                                            |_, _, cx| cx.stop_propagation(),
                                                        )
                                                        .child(
                                                            Checkbox::new(ElementId::Name(
                                                                format!(
                                                                    "check-{}",
                                                                    entry.full_path
                                                                )
                                                                .into(),
                                                            ))
                                                            .checked(is_checked)
                                                            .on_click(window.listener_for(&view, {
                                                                let path = entry.full_path.clone();
                                                                move |this, checked, _, cx| {
                                                                    this.toggle_sftp_entry(
                                                                        path.clone(),
                                                                        *checked,
                                                                        cx,
                                                                    );
                                                                }
                                                            })),
                                                        ),
                                                )
                                                .child(
                                                    h_flex()
                                                        .flex_1()
                                                        .min_w(px(0.))
                                                        .items_center()
                                                        .gap_2()
                                                        .child(
                                                            div()
                                                                .w(icon_col_width)
                                                                .flex_none()
                                                                .text_size(rems(1.0))
                                                                .text_color(name_color)
                                                                .child(if entry.is_dir {
                                                                    "📁"
                                                                } else {
                                                                    "📄"
                                                                }),
                                                        )
                                                        .child(
                                                            div()
                                                                .flex_1()
                                                                .min_w(px(0.))
                                                                .overflow_hidden()
                                                                .text_size(rems(1.0))
                                                                .text_color(name_color)
                                                                .child(entry.name),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .w(size_col_width)
                                                        .flex_none()
                                                        .text_size(rems(0.917))
                                                        .text_color(theme.muted_foreground)
                                                        .child(if entry.is_dir {
                                                            "-".to_string()
                                                        } else {
                                                            format_bytes(entry.size)
                                                        }),
                                                )
                                                .child(
                                                    div()
                                                        .w(modified_col_width)
                                                        .flex_none()
                                                        .text_size(rems(0.917))
                                                        .text_color(theme.muted_foreground)
                                                        .child(format_mtime(entry.modified)),
                                                )
                                                .child(div().w(px(12.)).flex_none())
                                                .into_any_element(),
                                        )
                                    })
                                    .collect::<Vec<_>>()
                            },
                        )
                        .size_full()
                        .track_scroll(&self.remote_files_scroll_handle)
                    })
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .right_0()
                            .bottom_0()
                            .w(px(16.))
                            .child(
                                Scrollbar::vertical(&self.remote_files_scroll_handle)
                                    .scrollbar_show(ScrollbarShow::Always),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .flex_none()
                    .h(px(24.))
                    .px_3()
                    .items_center()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().tab_bar)
                    .child(
                        div()
                            .min_w(px(0.))
                            .overflow_hidden()
                            .text_size(rems(0.917))
                            .text_color(cx.theme().muted_foreground)
                            .child(status),
                    ),
            );
        }

        panel.into_any_element()
    }

    fn render_monitoring_panel(
        &mut self,
        viewport_width: Pixels,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let cpu_color = cx.theme().chart_1;
        let mem_color = cx.theme().chart_2;
        let swap_color = cx.theme().chart_3;
        let net_color = cx.theme().chart_4;
        let disk_color = cx.theme().chart_5;
        let border_color = cx.theme().border;
        let muted_fg = cx.theme().muted_foreground;

        let cpu_pct = self.system.cpu_percent;
        // Dynamic CPU line color: green <30%, amber 30-80%, red >80%
        // NOTE: Hsla.h is normalized 0..=1 (not degrees)
        let cpu_path_color = {
            let pct = cpu_pct * 100.0;
            if pct < 30.0 {
                Hsla {
                    h: 120.0 / 360.0,
                    s: 0.65,
                    l: 0.45,
                    a: 1.0,
                }
            } else if pct < 80.0 {
                Hsla {
                    h: 45.0 / 360.0,
                    s: 0.8,
                    l: 0.55,
                    a: 1.0,
                }
            } else {
                Hsla {
                    h: 0.0,
                    s: 0.8,
                    l: 0.55,
                    a: 1.0,
                }
            }
        };
        // Network TX color: derived from net_color for visual distinction from RX
        let net_tx_color = if net_color.l > 0.5 {
            Hsla {
                l: net_color.l * 0.6,
                ..net_color
            }
        } else {
            Hsla {
                l: net_color.l * 1.5,
                ..net_color
            }
        };
        let mem_pct = self.system.mem_percent;
        let swap_pct = self.system.swap_percent;
        let mem_detail = self.system.mem_detail.clone();
        let swap_detail = self.system.swap_detail.clone();
        let net_rx = self.system.net_rx.clone();
        let net_tx = self.system.net_tx.clone();

        let (disk_used, disk_total) = self.system.disks.iter().fold((0u64, 0u64), |(u, t), d| {
            (u + (d.total_bytes - d.available_bytes), t + d.total_bytes)
        });
        let disk_pct = if disk_total > 0 {
            disk_used as f64 / disk_total as f64 * 100.0
        } else {
            0.0
        };

        let cpu_spark_data = self.cpu_history.clone();
        let net_rx_history = self.net_rx_history.clone();
        let net_tx_history = self.net_tx_history.clone();
        let disks = self.system.disks.clone();
        let card_min_w = px(110.);

        let show_net_card = viewport_width > px(750.);
        let show_disk_card = viewport_width > px(600.);

        // --- CPU card ---
        let cpu_card = v_flex()
            .min_w(card_min_w)
            .flex_1()
            .h_full()
            .px_1()
            .py_1()
            .gap_0p5()
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .child(
                        div()
                            .text_size(rems(0.833))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(cpu_color)
                            .child("CPU"),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_size(rems(0.833))
                            .text_color(muted_fg)
                            .child(format!("{:.0}%", cpu_pct * 100.0)),
                    ),
            )
            .child(
                canvas(
                    move |bounds, _window, _cx| {
                        let n = cpu_spark_data.len();
                        if n < 2 {
                            return None;
                        }
                        let mut path = PathBuilder::stroke(px(1.5));
                        let w = bounds.size.width;
                        let h = bounds.size.height;
                        let max_val = cpu_spark_data
                            .iter()
                            .cloned()
                            .fold(0.0f32, f32::max)
                            .max(0.1);
                        for (i, &val) in cpu_spark_data.iter().enumerate() {
                            let x = bounds.origin.x + w * i as f32 / (n - 1).max(1) as f32;
                            let y = bounds.origin.y + h * (1.0 - val / max_val * 0.85);
                            let pt = point(x, y);
                            if i == 0 {
                                path.move_to(pt);
                            } else {
                                path.line_to(pt);
                            }
                        }
                        path.build().ok()
                    },
                    move |_bounds, path_opt, window, _cx| {
                        if let Some(path) = path_opt {
                            window.paint_path(path, cpu_path_color);
                        }
                    },
                )
                .flex_1()
                .w_full(),
            );

        // --- MEM card: mem + swap ---
        let mem_card = v_flex()
            .min_w(card_min_w)
            .flex_1()
            .h_full()
            .px_1()
            .py_1()
            .gap_0p5()
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .child(
                        div()
                            .text_size(rems(0.833))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(mem_color)
                            .child("MEM"),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_size(rems(0.833))
                            .text_color(muted_fg)
                            .child(format!("{:.0}%", mem_pct * 100.0)),
                    ),
            )
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_1()
                    .child(
                        Progress::new("mem-progress")
                            .value(mem_pct * 100.0)
                            .color(mem_color)
                            .with_size(px(5.))
                            .flex_1(),
                    )
                    .child(
                        div()
                            .text_size(rems(0.7))
                            .text_color(muted_fg)
                            .child(mem_detail),
                    ),
            )
            .when(self.system.total_swap > 0, |this| {
                this.child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .gap_1()
                        .child(
                            Progress::new("swap-progress")
                                .value(swap_pct * 100.0)
                                .color(swap_color)
                                .with_size(px(4.))
                                .flex_1(),
                        )
                        .child(
                            div()
                                .text_size(rems(0.7))
                                .text_color(muted_fg)
                                .child(swap_detail),
                        ),
                )
            });

        // --- NET card: rx/tx text + dual sparkline ---
        let net_card = if show_net_card {
            Some(
                v_flex()
                    .min_w(card_min_w)
                    .flex_1()
                    .h_full()
                    .px_1()
                    .py_1()
                    .gap_0p5()
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .child(
                                div()
                                    .text_size(rems(0.833))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(net_color)
                                    .child("NET"),
                            )
                            .child(div().flex_1())
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_size(rems(0.75))
                                            .text_color(net_color)
                                            .child(format!("↓{}", net_rx)),
                                    )
                                    .child(
                                        div()
                                            .text_size(rems(0.75))
                                            .text_color(net_tx_color)
                                            .child(format!("↑{}", net_tx)),
                                    ),
                            ),
                    )
                    .child(
                        canvas(
                            move |bounds, _window, _cx| {
                                let n_rx = net_rx_history.len();
                                let n_tx = net_tx_history.len();
                                if n_rx < 2 || n_tx < 2 {
                                    return None;
                                }
                                let all: Vec<f32> = net_rx_history
                                    .iter()
                                    .chain(net_tx_history.iter())
                                    .cloned()
                                    .collect();
                                let max_val = all.iter().cloned().fold(0.0f32, f32::max).max(1.0);
                                let w = bounds.size.width;
                                let h = bounds.size.height;
                                let mut paths = Vec::new();

                                let mut rx_path = PathBuilder::stroke(px(1.5));
                                for (i, &val) in net_rx_history.iter().enumerate() {
                                    let x =
                                        bounds.origin.x + w * i as f32 / (n_rx - 1).max(1) as f32;
                                    let y = bounds.origin.y + h * (1.0 - val / max_val * 0.85);
                                    let pt = point(x, y);
                                    if i == 0 {
                                        rx_path.move_to(pt);
                                    } else {
                                        rx_path.line_to(pt);
                                    }
                                }
                                if let Ok(path) = rx_path.build() {
                                    paths.push((path, net_color));
                                }

                                let mut tx_path = PathBuilder::stroke(px(1.0));
                                for (i, &val) in net_tx_history.iter().enumerate() {
                                    let x =
                                        bounds.origin.x + w * i as f32 / (n_tx - 1).max(1) as f32;
                                    let y = bounds.origin.y + h * (1.0 - val / max_val * 0.85);
                                    let pt = point(x, y);
                                    if i == 0 {
                                        tx_path.move_to(pt);
                                    } else {
                                        tx_path.line_to(pt);
                                    }
                                }
                                if let Ok(path) = tx_path.build() {
                                    paths.push((path, net_tx_color));
                                }

                                Some(paths)
                            },
                            move |_bounds, paths_opt, window, _cx| {
                                if let Some(paths) = paths_opt {
                                    for (path, color) in paths {
                                        window.paint_path(path, color);
                                    }
                                }
                            },
                        )
                        .flex_1()
                        .w_full(),
                    ),
            )
        } else {
            None
        };

        // --- DISK card ---
        let disk_card = if show_disk_card {
            Some(
                v_flex()
                    .min_w(card_min_w)
                    .flex_1()
                    .h_full()
                    .px_1()
                    .py_1()
                    .gap_0p5()
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .child(
                                div()
                                    .text_size(rems(0.833))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(disk_color)
                                    .child("DISK"),
                            )
                            .child(div().flex_1())
                            .child(
                                div()
                                    .text_size(rems(0.833))
                                    .text_color(muted_fg)
                                    .child(format!("{:.0}%", disk_pct)),
                            ),
                    )
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_h(px(0.))
                            .child(
                                v_flex()
                                    .id("disk-scroll")
                                    .track_scroll(&self.disk_scroll_handle)
                                    .overflow_y_scroll()
                                    .size_full()
                                    .children(disks.iter().map(|disk| {
                                        let pct = if disk.total_bytes > 0 {
                                            (disk.total_bytes - disk.available_bytes) as f64
                                                / disk.total_bytes as f64
                                                * 100.0
                                        } else {
                                            0.0
                                        };
                                        let mount_short = disk.mount.clone();
                                        let mount_id = format!("disk-{}", mount_short);
                                        h_flex()
                                            .w_full()
                                            .items_center()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_size(rems(0.667))
                                                    .text_color(muted_fg)
                                                    .child(mount_short),
                                            )
                                            .child(
                                                Progress::new(mount_id)
                                                    .value(pct as f32)
                                                    .color(disk_color)
                                                    .with_size(px(4.))
                                                    .flex_1(),
                                            )
                                            .child(
                                                div()
                                                    .text_size(rems(0.667))
                                                    .text_color(muted_fg)
                                                    .child(format!("{:.0}%", pct)),
                                            )
                                    }))
                            )
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .right_0()
                                    .bottom_0()
                                    .w(px(8.))
                                    .child(
                                        Scrollbar::vertical(&self.disk_scroll_handle)
                                            .scrollbar_show(ScrollbarShow::Scrolling)
                                    )
                            )
                            .into_any_element()
                    )
                    .into_any_element(),
            )
        } else {
            None
        };

        let mut panel = h_flex()
            .h(px(80.))
            .w_full()
            .flex_none()
            .px_3()
            .gap_3()
            .border_b_1()
            .border_color(border_color)
            .bg(cx.theme().muted);

        panel = panel.child(cpu_card);
        panel = panel.child(mem_card);
        if let Some(card) = net_card {
            panel = panel.child(card);
        }
        if let Some(card) = disk_card {
            panel = panel.child(card);
        }
        panel
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let sessions = self.config.sessions().to_vec();
        let active_session_id = self.active_session_id().map(ToOwned::to_owned);

        v_flex()
            .gap_4()
            .w_full()
            .h_full()
            .min_w(px(0.))
            .p_4()
            .border_r_1()
            .border_color(cx.theme().sidebar_border)
            .bg(cx.theme().sidebar)
            .overflow_hidden()
            .child(
                v_flex()
                    .gap_1()
                    .min_w(px(0.))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .text_size(rems(1.667))
                                    .text_color(cx.theme().primary)
                                    .child("Ashell"),
                            )
                            .child(div().flex_1())
                            .child(
                                Button::new("sidebar-settings")
                                    .ghost()
                                    .small()
                                    .icon(IconName::Settings2)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.show_settings_dialog(window, cx)
                                    })),
                            )
                            .child(self.theme_dropdown(cx)),
                    )
                    .child(
                        div()
                            .text_size(rems(0.917))
                            .text_color(cx.theme().muted_foreground)
                            .child({
                                if let Some(kind) = self.active_kind() {
                                    match kind {
                                        TabKind::Local => t!("local_terminal").to_string(),
                                        TabKind::Ssh => {
                                            if let Some((_, session)) = self.active_ssh_session() {
                                                format!("ssh / {}", session.name)
                                            } else {
                                                "ssh".to_string()
                                            }
                                        }
                                    }
                                } else {
                                    self.active_title()
                                }
                            }),
                    ),
            )
            .child(
                Button::new("open-ssh-panel")
                    .primary()
                    .label(t!("add_ssh").to_string())
                    .on_click(
                        cx.listener(|this, _, window, cx| this.open_new_ssh_dialog(window, cx)),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_h(px(0.))
                    .gap_2()
                    .child(
                        div()
                            .text_size(rems(1.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(cx.theme().primary)
                            .child(t!("saved")),
                    )
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_h(px(0.))
                            .size_full()
                            .child(
                                v_flex()
                                    .size_full()
                                    .id("saved-sessions-scroll")
                                    .track_scroll(&self.saved_scroll_handle)
                                    .overflow_y_scroll()
                                    .gap_2()
                                    .children(sessions.into_iter().enumerate().map(
                                        |(ix, session)| {
                                            let connect_id = session.id.clone();
                                            let edit_id = session.id.clone();
                                            let delete_id = session.id.clone();
                                            let is_active = active_session_id.as_deref()
                                                == Some(session.id.as_str());
                                            let name = session.name.clone();
                                            let detail = self.session_detail(&session);
                                            div()
                                                .id(("saved-connect", ix))
                                                .w_full()
                                                .p_2()
                                                .rounded_md()
                                                .border_1()
                                                .border_color(if is_active {
                                                    cx.theme().primary
                                                } else {
                                                    cx.theme().border
                                                })
                                                .bg(if is_active {
                                                    cx.theme().tab_active
                                                } else {
                                                    cx.theme().muted
                                                })
                                                .cursor_pointer()
                                                .hover(|this| this.bg(cx.theme().secondary))
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener(move |this, _, _, cx| {
                                                        this.connect_saved_session(
                                                            connect_id.clone(),
                                                            cx,
                                                        )
                                                    }),
                                                )
                                                .context_menu({
                                                    let view = cx.entity();
                                                    move |menu, window, _| {
                                                        let edit_value = edit_id.clone();
                                                        let delete_value = delete_id.clone();
                                                        menu.item(
                                                            PopupMenuItem::new("Edit").on_click(
                                                                window.listener_for(
                                                                    &view,
                                                                    move |this, _, window, cx| {
                                                                        this.edit_saved_session(
                                                                            edit_value.clone(),
                                                                            window,
                                                                            cx,
                                                                        )
                                                                    },
                                                                ),
                                                            ),
                                                        )
                                                        .item(
                                                            PopupMenuItem::new("Delete").on_click(
                                                                window.listener_for(
                                                                    &view,
                                                                    move |this, _, _, cx| {
                                                                        this.remove_saved_session(
                                                                            delete_value.clone(),
                                                                            cx,
                                                                        )
                                                                    },
                                                                ),
                                                            ),
                                                        )
                                                    }
                                                })
                                                .child(
                                                    v_flex()
                                                        .gap_1()
                                                        .child(
                                                            div()
                                                                .text_size(rems(1.0))
                                                                .font_weight(FontWeight::SEMIBOLD)
                                                                .child(name),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_size(rems(0.917))
                                                                .text_color(
                                                                    cx.theme().muted_foreground,
                                                                )
                                                                .child(detail),
                                                        ),
                                                )
                                        },
                                    )),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .bottom_0()
                                    .left_0()
                                    .right_0()
                                    .child(
                                        gpui_component::scroll::Scrollbar::new(
                                            &self.saved_scroll_handle,
                                        )
                                        .id("saved-scrollbar")
                                        .axis(gpui_component::scroll::ScrollbarAxis::Vertical)
                                        .scrollbar_show(
                                            gpui_component::scroll::ScrollbarShow::Always,
                                        ),
                                    ),
                            ),
                    ),
            )
    }

    fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let active_tab_index = self
            .active_tab
            .as_ref()
            .and_then(|active_id| self.tabs.iter().position(|tab| &tab.id == active_id));
        v_flex()
            .h(px(32.))
            .w_full()
            .flex_none()
            .bg(cx.theme().tab_bar)
            .child(
                h_flex()
                    .h_full()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .h_full()
                            .overflow_x_hidden()
                            .child(
                                TabBar::new("ashell-tab-bar")
                                    .track_scroll(&self.tabs_scroll_handle)
                                    .selected_index(active_tab_index.unwrap_or(0))
                                    .children(self.tabs.iter().enumerate().map(|(ix, tab)| {
                                        let id = tab.id.clone();
                                        let close_id = tab.id.clone();
                                        let dot_color = if tab.connected {
                                            cx.theme().success
                                        } else {
                                            cx.theme().danger
                                        };
                                        Tab::new()
                                            .prefix(
                                                div()
                                                    .w(px(5.))
                                                    .h(px(32.))
                                                    .bg(dot_color),
                                            )
                                            .label(tab.title.clone())
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.activate_tab(id.clone(), window, cx)
                                            }))
                                            .suffix(
                                                Button::new(("tab-close", ix))
                                                    .ghost()
                                                    .xsmall()
                                                    .icon(IconName::Close)
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        |_, window, cx| {
                                                            window.prevent_default();
                                                            cx.stop_propagation();
                                                        },
                                                    )
                                                    .on_click(cx.listener(
                                                        move |this, _, window, cx| {
                                                            window.prevent_default();
                                                            cx.stop_propagation();
                                                            this.close_tab(close_id.clone(), cx)
                                                        },
                                                    )),
                                            )
                                    }))
                                    .last_empty_space(div().w_3())
                                    .w_full()
                                    .h_full(),
                            ),
                    )
                    .child(
                        h_flex()
                            .flex_none()
                            .items_center()
                            .gap_1()
                            .pr(px(6.))
                            .child(
                                Button::new("open-selector")
                                    .secondary()
                                    .small()
                                    .rounded(px(999.))
                                    .icon(IconName::Plus)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.show_selector_dialog(window, cx)
                                    })),
                            )
                            .child(
                                Button::new("split-horizontal")
                                    .secondary()
                                    .small()
                                    .rounded(px(999.))
                                    .icon(IconName::PanelBottom)
                                    .on_click(cx.listener(|_, _, _, _| {}))
                            )
                            .child(
                                Button::new("split-vertical")
                                    .secondary()
                                    .small()
                                    .rounded(px(999.))
                                    .icon(IconName::PanelRight)
                                    .on_click(cx.listener(|_, _, _, _| {}))
                            ),
                    ),
            )
    }

    fn render_terminal_panel(
        &mut self,
        terminal_snapshot: Option<terminal::RenderSnapshot>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex().size_full().child(
            div()
                .size_full()
                .overflow_hidden()
                .track_focus(&self.focus_handle)
                .key_context(TERMINAL_KEY_CONTEXT)
                .on_mouse_down(MouseButton::Left, cx.listener(Self::focus_terminal))
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(Self::on_terminal_right_click),
                )
                .on_mouse_move(cx.listener(Self::on_terminal_mouse_move))
                .on_mouse_up(MouseButton::Left, cx.listener(Self::on_terminal_mouse_up))
                .on_key_down(cx.listener(Self::on_terminal_key_down))
                .on_action(cx.listener(Self::on_terminal_tab_action))
                .on_action(cx.listener(Self::on_terminal_backtab_action))
                .on_scroll_wheel(cx.listener(Self::on_terminal_scroll))
                .child(match terminal_snapshot.clone() {
                    None => self.render_home_page(cx).into_any_element(),
                    Some(snapshot) => {
                        let view = cx.entity();
                        div()
                            .size_full()
                            .on_prepaint({
                                let view = view.clone();
                                move |bounds, _window, cx| {
                                    let _ = view.update(cx, |this, _| {
                                        this.terminal_bounds = Some(bounds);
                                    });
                                }
                            })
                            .child(terminal_element::TerminalElement::new(
                                cx.entity(),
                                self.focus_handle.clone(),
                                snapshot,
                                self.terminal_marked_text.clone(),
                                self.terminal_font_family.clone(),
                                px(self.terminal_font_size),
                                px(self.terminal_line_height()),
                                px(self.terminal_cell_width()),
                            ))
                            .vertical_scrollbar(&self.terminal_scrollbar)
                            .into_any_element()
                    }
                }),
        )
    }
}

impl Render for Ashell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self
            .active_tab
            .as_ref()
            .is_some_and(|active_id| !self.tabs.iter().any(|tab| &tab.id == active_id))
        {
            self.active_tab = self.tabs.first().map(|tab| tab.id.clone());
            self.cpu_history.clear();
            self.net_rx_history.clear();
            self.net_tx_history.clear();
            self.remote_sample_in_flight = false;
            self.request_active_system_snapshot();
        }
        self.sync_sftp_path_input(window, cx);
        self.sync_terminal_size(window, cx);
        if self.show_transfers_dialog {
            self.show_transfers_dialog = false;
            self.show_transfers_dialog(window, cx);
        }
        if let Some(new_display_offset) = self.terminal_scrollbar.future_display_offset.take() {
            if let Some(active_id) = self.active_tab.clone() {
                if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == active_id) {
                    let current = tab.render_snapshot().display_offset;
                    match new_display_offset.cmp(&current) {
                        std::cmp::Ordering::Greater => {
                            tab.scroll_up_by(new_display_offset - current)
                        }
                        std::cmp::Ordering::Less => {
                            tab.scroll_down_by(current - new_display_offset)
                        }
                        std::cmp::Ordering::Equal => {}
                    }
                }
            }
        }
        let terminal_snapshot = self.active_snapshot();
        if let Some(snapshot) = terminal_snapshot.as_ref() {
            self.terminal_scrollbar
                .update(snapshot, px(self.terminal_line_height()));
        }

        let sidebar_area = resizable_panel()
            .size(px(self
                .config
                .workspace_panels()
                .and_then(|s| s.first().copied())
                .unwrap_or(SIDEBAR_WIDTH)))
            .size_range(px(240.)..px(520.))
            .flex_none()
            .child(self.sidebar(cx));

        let monitoring_contents = v_flex()
            .size_full()
            .child(self.render_monitoring_panel(window.viewport_size().width, cx))
            .child(self.render_sftp_panel(window, cx));

        let body_panel = if self.sftp_panel_minimized {
            v_flex()
                .size_full()
                .child(
                    div().flex_1().min_h(px(0.)).child(
                        v_resizable("ashell-body")
                            .with_state(&self.body_panels)
                            .child(resizable_panel().child(self.render_terminal_panel(terminal_snapshot, cx)))
                    )
                )
                .child(
                    div()
                        .flex_none()
                        .h(px(114.))
                        .w_full()
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .child(monitoring_contents)
                )
                .into_any_element()
        } else {
            v_resizable("ashell-body")
                .with_state(&self.body_panels)
                .child(resizable_panel().child(self.render_terminal_panel(terminal_snapshot, cx)))
                .child(
                    resizable_panel()
                        .size(px(328.))
                        .size_range(px(260.)..px(1200.))
                        .child(monitoring_contents)
                )
                .into_any_element()
        };

        let main_area = resizable_panel().child(
            v_flex()
                .size_full()
                .relative()
                .overflow_hidden()
                .child(self.render_tab_bar(cx))
                .child(body_panel),
        );

        let workspace = h_resizable("ashell-workspace")
            .with_state(&self.workspace_panels)
            .child(sidebar_area)
            .child(main_area);

        div()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .font_family(self.ui_font_family.clone())
            .child(workspace)
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
            .when_some(self.sftp_context_menu.clone(), |this, menu| {
                let label = if menu.is_dir {
                    t!("download_folder").to_string()
                } else {
                    t!("download").to_string()
                };
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.dismiss_sftp_context_menu(cx);
                            }),
                        )
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(|this, _, _, cx| {
                                this.dismiss_sftp_context_menu(cx);
                            }),
                        )
                        .child(
                            div()
                                .absolute()
                                .left(menu.position.x)
                                .top(menu.position.y)
                                .w(px(172.))
                                .p_1()
                                .rounded_md()
                                .border_1()
                                .border_color(cx.theme().border)
                                .bg(cx.theme().popover)
                                .shadow_lg()
                                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                    window.prevent_default();
                                    cx.stop_propagation();
                                })
                                .on_mouse_down(MouseButton::Right, |_, window, cx| {
                                    window.prevent_default();
                                    cx.stop_propagation();
                                })
                                .child(
                                    v_flex()
                                        .w_full()
                                        .child(
                                            Button::new("sftp-context-download")
                                                .ghost()
                                                .w_full()
                                                .justify_start()
                                                .label(label)
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.trigger_sftp_context_download(window, cx);
                                                })),
                                        )
                                        .when(
                                            !menu.is_dir
                                                && is_editable_text_file(&menu.remote_path),
                                            |this| {
                                                this.child(
                                                    Button::new("sftp-context-edit")
                                                        .ghost()
                                                        .w_full()
                                                        .justify_start()
                                                        .label(t!("edit_file"))
                                                        .tooltip(
                                                            t!("edit_file_tooltip").to_string(),
                                                        )
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.trigger_sftp_context_edit(cx);
                                                        })),
                                                )
                                            },
                                        ),
                                ),
                        ),
                )
            })
            .when_some(self.connection_progress.clone(), |this, progress| {
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .bg(Hsla {
                            h: 0.0,
                            s: 0.0,
                            l: 0.0,
                            a: 0.48,
                        })
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .w(px(420.))
                                .p_5()
                                .rounded_lg()
                                .border_1()
                                .border_color(cx.theme().border)
                                .bg(cx.theme().popover)
                                .shadow_lg()
                                .child(
                                    v_flex()
                                        .gap_4()
                                        .child(
                                            Button::new("ssh-connect-progress")
                                                .primary()
                                                .loading(!progress.failed)
                                                .label(progress.title.clone()),
                                        )
                                        .child(div().max_h(px(220.)).overflow_y_scrollbar().child(
                                            v_flex().gap_2().children(
                                                progress.lines.iter().cloned().map(|line| {
                                                    div()
                                                        .text_size(rems(1.0))
                                                        .text_color(if progress.failed {
                                                            cx.theme().danger
                                                        } else {
                                                            cx.theme().muted_foreground
                                                        })
                                                        .child(line)
                                                }),
                                            ),
                                        ))
                                        .when(progress.failed, |this| {
                                            this.child(
                                                h_flex()
                                                    .justify_end()
                                                    .gap_2()
                                                    .child(
                                                        Button::new("ssh-connect-progress-retry")
                                                            .primary()
                                                            .label(t!("retry").to_string())
                                                            .on_click(cx.listener(
                                                                |this, _, _, cx| {
                                                                    this.retry_connection_progress(
                                                                        cx,
                                                                    )
                                                                },
                                                            )),
                                                    )
                                                    .child(
                                                        Button::new("ssh-connect-progress-close")
                                                            .label(t!("cancel").to_string())
                                                            .on_click(cx.listener(
                                                                |this, _, _, cx| {
                                                                    this.cancel_connection_progress(
                                                                        cx,
                                                                    )
                                                                },
                                                            )),
                                                    ),
                                            )
                                        }),
                                ),
                        ),
                )
            })
    }
}
