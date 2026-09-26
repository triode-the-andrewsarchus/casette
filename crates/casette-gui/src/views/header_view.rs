//! Top navigation and toolbar header for Casette.

use crate::model::{SessionState, WorkerStatus};
use crate::theme::Colors;
use egui::{
    vec2, Align, Button, ComboBox, CornerRadius, FontFamily, FontId, Frame, Layout, RichText,
    Stroke, Ui,
};

/// Renders the top toolbar header.
pub fn render_header(
    ui: &mut Ui,
    state: &mut SessionState,
    sidebar_visible: &mut bool,
    doctor_open: &mut bool,
    on_precision_change: &mut dyn FnMut(u32),
    on_clear_tape: &mut dyn FnMut(),
) {
    Frame::new()
        .fill(Colors::CARD_BG)
        .inner_margin(egui::Margin::symmetric(14, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Brand / Title
                ui.label(
                    RichText::new("Casette")
                        .color(Colors::TEXT_PRIMARY)
                        .font(FontId::new(17.0, FontFamily::Proportional))
                        .strong(),
                );

                ui.add_space(8.0);

                // Worker Status Indicator
                render_status_badge(ui, state.worker_status, state.worker_pid);

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Sidebar toggle button
                    let sidebar_text = if *sidebar_visible {
                        "Hide Sidebar"
                    } else {
                        "Sidebar"
                    };
                    if ui
                        .add(
                            Button::new(
                                RichText::new(sidebar_text)
                                    .color(Colors::TEXT_SECONDARY)
                                    .font(FontId::new(12.0, FontFamily::Proportional)),
                            )
                            .corner_radius(5),
                        )
                        .clicked()
                    {
                        *sidebar_visible = !*sidebar_visible;
                    }

                    // Sage Doctor button
                    let doc_btn = Button::new(
                        RichText::new("🩺 Doctor")
                            .color(if *doctor_open {
                                Colors::ACCENT_PRIMARY
                            } else {
                                Colors::TEXT_SECONDARY
                            })
                            .font(FontId::new(12.0, FontFamily::Proportional)),
                    )
                    .fill(if *doctor_open {
                        Colors::CARD_BG_SELECTED
                    } else {
                        Colors::CARD_BG
                    })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if *doctor_open {
                            Colors::ACCENT_PRIMARY
                        } else {
                            Colors::BORDER_SUBTLE
                        },
                    ))
                    .corner_radius(5);

                    if ui
                        .add(doc_btn)
                        .on_hover_text("View Sage environment diagnostics and worker controls")
                        .clicked()
                    {
                        *doctor_open = !*doctor_open;
                    }

                    // Clear tape button
                    if ui
                        .add(
                            Button::new(
                                RichText::new("Clear Tape")
                                    .color(Colors::TEXT_MUTED)
                                    .font(FontId::new(12.0, FontFamily::Proportional)),
                            )
                            .frame(false),
                        )
                        .on_hover_text("Clear all rows from session tape")
                        .clicked()
                    {
                        on_clear_tape();
                    }

                    ui.separator();

                    // Precision selector dropdown
                    ui.label(
                        RichText::new("Precision:")
                            .color(Colors::TEXT_MUTED)
                            .font(FontId::new(11.5, FontFamily::Proportional)),
                    );

                    let prev_prec = state.precision_digits;
                    ComboBox::from_id_salt("precision_combo")
                        .selected_text(format!("{} digits", state.precision_digits))
                        .width(90.0)
                        .show_ui(ui, |ui| {
                            for digits in [2, 3, 5, 10, 15, 20, 50] {
                                ui.selectable_value(
                                    &mut state.precision_digits,
                                    digits,
                                    format!("{} digits", digits),
                                );
                            }
                        });

                    if state.precision_digits != prev_prec {
                        on_precision_change(state.precision_digits);
                    }
                });
            });
        });
}

fn render_status_badge(ui: &mut Ui, status: WorkerStatus, pid: Option<u32>) {
    let (dot_color, label) = match status {
        WorkerStatus::Starting => (Colors::NUMERIC_AMBER, "Booting Sage...".to_string()),
        WorkerStatus::Ready => {
            let pid_str = pid.map(|p| format!(" (pid {})", p)).unwrap_or_default();
            (Colors::EXACT_GREEN, format!("Sage Ready{}", pid_str))
        }
        WorkerStatus::Busy => (Colors::ACCENT_PRIMARY, "Calculating...".to_string()),
        WorkerStatus::Restarting => (Colors::NUMERIC_AMBER, "Restarting...".to_string()),
        WorkerStatus::Error => (Colors::ERROR_RED, "Worker Error".to_string()),
    };

    Frame::new()
        .fill(Colors::BADGE_BG)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Colored dot
                let (rect, _) = ui.allocate_exact_size(vec2(7.0, 7.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 3.5, dot_color);

                ui.label(
                    RichText::new(label)
                        .color(Colors::TEXT_SECONDARY)
                        .font(FontId::new(11.0, FontFamily::Proportional)),
                );
            });
        });
}
