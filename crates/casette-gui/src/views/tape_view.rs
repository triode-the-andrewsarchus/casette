//! Main session tape view rendering calculation history and results.

use crate::model::SessionState;
use crate::render::render_result;
use crate::render::texture_cache::TextureCache;
use crate::theme::Colors;
use egui::{CornerRadius, FontFamily, FontId, Frame, RichText, ScrollArea, Stroke, Ui};

/// Renders the central scrollable session tape.
pub fn render_tape(
    ui: &mut Ui,
    state: &mut SessionState,
    texture_cache: &mut TextureCache,
    on_action: &mut dyn FnMut(&str, &str),
) {
    if state.rows.is_empty() {
        render_empty_tape_placeholder(ui);
        return;
    }

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            ui.add_space(8.0);

            for (idx, row) in state.rows.iter().enumerate() {
                let is_selected = state.selected_row_index == Some(idx);
                let border_stroke = if is_selected {
                    Stroke::new(1.5_f32, Colors::ACCENT_PRIMARY)
                } else {
                    Stroke::new(1.0_f32, Colors::BORDER_SUBTLE)
                };

                let card_response = Frame::new()
                    .fill(Colors::CARD_BG)
                    .stroke(border_stroke)
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(14, 12))
                    .show(ui, |ui| {
                        // 1. Row Header: Badge `#1`, duration, status
                        ui.horizontal(|ui| {
                            // Row number badge
                            Frame::new()
                                .fill(Colors::BADGE_BG)
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(egui::Margin::symmetric(6, 2))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!("#{}", row.row_number))
                                            .color(Colors::TEXT_MUTED)
                                            .font(FontId::new(11.0, FontFamily::Monospace))
                                            .strong(),
                                    );
                                });

                            if let Some(dur) = row.duration {
                                let dur_str = if dur.as_millis() < 1000 {
                                    format!("{} ms", dur.as_millis())
                                } else {
                                    format!("{:.2} s", dur.as_secs_f64())
                                };
                                ui.label(
                                    RichText::new(dur_str)
                                        .color(Colors::TEXT_MUTED)
                                        .font(FontId::new(11.0, FontFamily::Proportional)),
                                );
                            }

                            if !row.is_live {
                                ui.label(
                                    RichText::new("Restored (not live)")
                                        .color(Colors::NUMERIC_AMBER)
                                        .font(FontId::new(11.0, FontFamily::Proportional)),
                                );
                            }
                        });

                        ui.add_space(6.0);

                        // 2. User Input Expression
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("›")
                                    .color(Colors::ACCENT_PRIMARY)
                                    .font(FontId::new(15.0, FontFamily::Proportional))
                                    .strong(),
                            );
                            ui.label(
                                RichText::new(&row.input_raw)
                                    .color(Colors::TEXT_PRIMARY)
                                    .font(FontId::new(14.0, FontFamily::Proportional)),
                            );
                        });

                        // 3. Compiled Sage representation (if different from input)
                        if !row.compiled_sage.is_empty() && row.compiled_sage != row.input_raw {
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Sage:")
                                        .color(Colors::TEXT_MUTED)
                                        .font(FontId::new(10.5, FontFamily::Proportional)),
                                );
                                ui.label(
                                    RichText::new(&row.compiled_sage)
                                        .color(Colors::ACCENT_PURPLE)
                                        .font(FontId::new(11.0, FontFamily::Monospace)),
                                );
                            });
                        }

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // 4. Result output
                        render_result(ui, row, texture_cache, on_action);
                    });

                // Clicking on row card selects it for inspector without intercepting child buttons
                let card_rect = card_response.response.rect;
                if ui.rect_contains_pointer(card_rect) && ui.input(|i| i.pointer.primary_clicked()) {
                    state.selected_row_index = Some(idx);
                }

                ui.add_space(10.0);
            }
        });
}

fn render_empty_tape_placeholder(ui: &mut Ui) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            ui.label(
                RichText::new("Casette Session Tape")
                    .color(Colors::TEXT_MUTED)
                    .font(FontId::new(20.0, FontFamily::Proportional))
                    .strong(),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "Type mathematical expressions below. Use #1, #2 to reference earlier rows.\nTry 'diff(sin(x), x)', 'factor(x^4 - 1)', '[1 2; 3 4]', or 'plot(sin(x), -pi..pi)'.",
                )
                .color(Colors::TEXT_MUTED)
                .font(FontId::new(13.0, FontFamily::Proportional)),
            );
        });
    });
}
