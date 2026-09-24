//! Tabbed sidebar view for Symbols, History, Inspector, and Sage Doctor.

use crate::model::SessionState;
use crate::theme::Colors;
use casette_kernel::BOOT_VARIABLE_NAMES;
use egui::{
    Button, Color32, CornerRadius, FontFamily, FontId, Frame, RichText, ScrollArea, Stroke,
    TextEdit, Ui,
};

/// Active tab within the sidebar panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    Symbols,
    History,
    Inspector,
    Doctor,
}

/// Renders the right sidebar panel.
#[allow(clippy::too_many_arguments)]
pub fn render_sidebar(
    ui: &mut Ui,
    state: &mut SessionState,
    current_tab: &mut SidebarTab,
    symbol_filter: &mut String,
    show_builtin_symbols: &mut bool,
    on_refresh_symbols: &mut dyn FnMut(),
    on_restart_worker: &mut dyn FnMut(),
    on_load_draft: &mut dyn FnMut(String),
) {
    Frame::new()
        .fill(Colors::SIDEBAR_BG)
        .stroke(Stroke::new(1.0_f32, Colors::BORDER_SUBTLE))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            // Tab Selector Bar
            ui.horizontal(|ui| {
                for (tab, label) in [
                    (SidebarTab::Symbols, "Symbols"),
                    (SidebarTab::History, "History"),
                    (SidebarTab::Inspector, "Inspector"),
                    (SidebarTab::Doctor, "Doctor"),
                ] {
                    let is_active = *current_tab == tab;
                    let text = RichText::new(label)
                        .color(if is_active {
                            Colors::TEXT_PRIMARY
                        } else {
                            Colors::TEXT_MUTED
                        })
                        .font(FontId::new(12.0, FontFamily::Proportional))
                        .strong();

                    let btn = Button::new(text)
                        .fill(if is_active {
                            Colors::CARD_BG
                        } else {
                            Color32::TRANSPARENT
                        })
                        .corner_radius(5);

                    if ui.add(btn).clicked() {
                        *current_tab = tab;
                    }
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Tab Content
            match current_tab {
                SidebarTab::Symbols => {
                    render_symbols_tab(
                        ui,
                        state,
                        symbol_filter,
                        show_builtin_symbols,
                        on_refresh_symbols,
                        on_load_draft,
                    );
                }
                SidebarTab::History => {
                    render_history_tab(ui, state, on_load_draft);
                }
                SidebarTab::Inspector => {
                    render_inspector_tab(ui, state);
                }
                SidebarTab::Doctor => {
                    render_doctor_tab(ui, state, on_restart_worker);
                }
            }
        });
}

fn render_symbols_tab(
    ui: &mut Ui,
    state: &SessionState,
    filter: &mut String,
    show_builtins: &mut bool,
    on_refresh: &mut dyn FnMut(),
    on_load_draft: &mut dyn FnMut(String),
) {
    ui.horizontal(|ui| {
        ui.add(
            TextEdit::singleline(filter)
                .hint_text("Filter symbols...")
                .desired_width(140.0),
        );

        if ui
            .add(
                Button::new(
                    RichText::new("Refresh")
                        .color(Colors::TEXT_SECONDARY)
                        .font(FontId::new(11.0, FontFamily::Proportional)),
                )
                .corner_radius(4),
            )
            .clicked()
        {
            on_refresh();
        }
    });

    ui.add_space(4.0);
    ui.checkbox(show_builtins, "Show Built-in Variables");
    ui.add_space(6.0);

    let filter_lower = filter.to_lowercase();
    let visible_symbols: Vec<_> = state
        .symbols
        .iter()
        .filter(|sym| {
            if !*show_builtins && BOOT_VARIABLE_NAMES.contains(&sym.name.as_str()) {
                // Hide default boot variables unless reassigned
                return false;
            }
            if !filter_lower.is_empty() && !sym.name.to_lowercase().contains(&filter_lower) {
                return false;
            }
            true
        })
        .collect();

    if visible_symbols.is_empty() {
        ui.label(
            RichText::new("No user symbols in session")
                .color(Colors::TEXT_MUTED)
                .italics(),
        );
        return;
    }

    ScrollArea::vertical().show(ui, |ui| {
        for sym in visible_symbols {
            Frame::new()
                .fill(Colors::CARD_BG)
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .link(
                                RichText::new(&sym.name)
                                    .color(Colors::ACCENT_PRIMARY)
                                    .font(FontId::new(12.5, FontFamily::Monospace))
                                    .strong(),
                            )
                            .on_hover_text("Insert symbol name into draft")
                            .clicked()
                        {
                            on_load_draft(sym.name.clone());
                        }

                        ui.label(
                            RichText::new(&sym.kind)
                                .color(Colors::TEXT_MUTED)
                                .font(FontId::new(10.5, FontFamily::Proportional)),
                        );
                    });

                    if !sym.summary.is_empty() {
                        ui.label(
                            RichText::new(&sym.summary)
                                .color(Colors::TEXT_SECONDARY)
                                .font(FontId::new(11.5, FontFamily::Monospace)),
                        );
                    }
                });
            ui.add_space(4.0);
        }
    });
}

fn render_history_tab(ui: &mut Ui, state: &SessionState, on_load_draft: &mut dyn FnMut(String)) {
    if state.history.is_empty() {
        ui.label(
            RichText::new("No calculation history")
                .color(Colors::TEXT_MUTED)
                .italics(),
        );
        return;
    }

    ScrollArea::vertical().show(ui, |ui| {
        for (idx, item) in state.history.iter().rev().enumerate() {
            Frame::new()
                .fill(Colors::CARD_BG)
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("#{}:", state.history.len() - idx))
                                .color(Colors::TEXT_MUTED)
                                .font(FontId::new(10.5, FontFamily::Monospace)),
                        );
                        if ui
                            .link(
                                RichText::new(item)
                                    .color(Colors::TEXT_PRIMARY)
                                    .font(FontId::new(12.0, FontFamily::Proportional)),
                            )
                            .on_hover_text("Click to copy into draft input")
                            .clicked()
                        {
                            on_load_draft(item.clone());
                        }
                    });
                });
            ui.add_space(4.0);
        }
    });
}

fn render_inspector_tab(ui: &mut Ui, state: &SessionState) {
    let Some(selected_idx) = state.selected_row_index else {
        ui.label(
            RichText::new("Select a row on the tape to inspect metadata")
                .color(Colors::TEXT_MUTED)
                .italics(),
        );
        return;
    };

    let Some(row) = state.rows.get(selected_idx) else {
        return;
    };

    ScrollArea::vertical().show(ui, |ui| {
        ui.label(
            RichText::new(format!("Row #{}", row.row_number))
                .color(Colors::TEXT_PRIMARY)
                .font(FontId::new(15.0, FontFamily::Proportional))
                .strong(),
        );

        ui.add_space(8.0);
        ui.label(
            RichText::new("Expression:")
                .color(Colors::TEXT_MUTED)
                .font(FontId::new(11.0, FontFamily::Proportional)),
        );
        ui.label(
            RichText::new(&row.input_raw)
                .color(Colors::TEXT_PRIMARY)
                .font(FontId::new(12.5, FontFamily::Monospace)),
        );

        if let Some(res) = &row.result {
            ui.add_space(8.0);
            ui.label(
                RichText::new("Classification Kind:")
                    .color(Colors::TEXT_MUTED)
                    .font(FontId::new(11.0, FontFamily::Proportional)),
            );
            ui.label(
                RichText::new(res.kind.as_deref().unwrap_or("unknown"))
                    .color(Colors::ACCENT_PRIMARY)
                    .font(FontId::new(12.5, FontFamily::Proportional)),
            );

            if let Some(plain) = &res.plain {
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Plain Text:")
                        .color(Colors::TEXT_MUTED)
                        .font(FontId::new(11.0, FontFamily::Proportional)),
                );
                ui.label(
                    RichText::new(plain)
                        .color(Colors::TEXT_PRIMARY)
                        .font(FontId::new(12.0, FontFamily::Monospace)),
                );
            }

            if let Some(latex) = &res.latex {
                ui.add_space(8.0);
                ui.label(
                    RichText::new("LaTeX Output:")
                        .color(Colors::TEXT_MUTED)
                        .font(FontId::new(11.0, FontFamily::Proportional)),
                );
                ui.label(
                    RichText::new(latex)
                        .color(Colors::ACCENT_PURPLE)
                        .font(FontId::new(11.5, FontFamily::Monospace)),
                );
            }

            // Matrix Properties section
            if let Some(props) = &res.matrix_properties {
                ui.add_space(10.0);
                ui.label(
                    RichText::new("Matrix Properties")
                        .color(Colors::TEXT_PRIMARY)
                        .font(FontId::new(13.0, FontFamily::Proportional))
                        .strong(),
                );
                ui.add_space(4.0);

                for prop in props {
                    let (indicator, color) = match prop.value {
                        Some(true) => ("✓", Colors::EXACT_GREEN),
                        Some(false) => ("✗", Colors::TEXT_MUTED),
                        None => ("-", Colors::NUMERIC_AMBER),
                    };

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(indicator)
                                .color(color)
                                .font(FontId::new(12.0, FontFamily::Monospace))
                                .strong(),
                        );
                        ui.label(
                            RichText::new(&prop.label)
                                .color(Colors::TEXT_SECONDARY)
                                .font(FontId::new(11.5, FontFamily::Proportional)),
                        );
                    });
                }
            }
        }
    });
}

fn render_doctor_tab(ui: &mut Ui, state: &SessionState, on_restart: &mut dyn FnMut()) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.label(
            RichText::new("Sage Doctor Diagnostics")
                .color(Colors::TEXT_PRIMARY)
                .font(FontId::new(14.0, FontFamily::Proportional))
                .strong(),
        );

        ui.add_space(8.0);
        render_diagnostic_row(
            ui,
            "Sage Version",
            if state.sage_version.is_empty() {
                "Not detected"
            } else {
                &state.sage_version
            },
        );

        render_diagnostic_row(
            ui,
            "Worker PID",
            &state
                .worker_pid
                .map(|p| p.to_string())
                .unwrap_or_else(|| "None".to_string()),
        );

        render_diagnostic_row(
            ui,
            "Status",
            match state.worker_status {
                crate::model::WorkerStatus::Ready => "Online and Ready",
                crate::model::WorkerStatus::Starting => "Booting",
                crate::model::WorkerStatus::Busy => "Calculating",
                crate::model::WorkerStatus::Restarting => "Restarting",
                crate::model::WorkerStatus::Error => "Error / Disconnected",
            },
        );

        ui.add_space(12.0);
        if ui
            .add(
                Button::new(
                    RichText::new("Restart Sage Worker")
                        .color(Colors::TEXT_PRIMARY)
                        .font(FontId::new(12.0, FontFamily::Proportional)),
                )
                .fill(Colors::CARD_BG)
                .stroke(Stroke::new(1.0_f32, Colors::ACCENT_PRIMARY))
                .corner_radius(6),
            )
            .clicked()
        {
            on_restart();
        }
    });
}

fn render_diagnostic_row(ui: &mut Ui, title: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{}:", title))
                .color(Colors::TEXT_MUTED)
                .font(FontId::new(11.5, FontFamily::Proportional)),
        );
        ui.label(
            RichText::new(value)
                .color(Colors::TEXT_PRIMARY)
                .font(FontId::new(12.0, FontFamily::Monospace)),
        );
    });
    ui.add_space(4.0);
}
