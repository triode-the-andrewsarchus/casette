//! Bottom input pane with real-time Friendly Compiler lowering and formula chips.

use crate::model::{SessionState, WorkerStatus};
use crate::theme::Colors;
use egui::{Align, Button, FontFamily, FontId, Frame, Key, Layout, RichText, Stroke, TextEdit, Ui};

/// Renders the bottom input editor pane.
pub fn render_input_pane(
    ui: &mut Ui,
    state: &mut SessionState,
    on_submit: &mut dyn FnMut(String, bool),
    on_interrupt: &mut dyn FnMut(),
) {
    Frame::new()
        .fill(Colors::CARD_BG)
        .stroke(Stroke::new(1.0_f32, Colors::BORDER_SUBTLE))
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            // 1. Live "Generated Sage" preview bar
            let tape_refs = state.tape_references();
            render_live_lowering_preview(ui, &state.draft_input, &tape_refs);

            ui.add_space(6.0);

            // 2. Formula quick chips bar
            if render_formula_chips(ui, &mut state.draft_input) {
                state.request_input_focus = true;
            }

            ui.add_space(6.0);

            // 3. Multi-line Input Editor
            let is_evaluating = state.worker_status == WorkerStatus::Busy;
            let mut submit_pressed = false;

            let response = ui.add(
                TextEdit::multiline(&mut state.draft_input)
                    .id_salt("casette_main_input_editor")
                    .hint_text("Type math or command (e.g. diff(#1, x), factor, matrix) - Enter to evaluate")
                    .font(FontId::new(14.0, FontFamily::Proportional))
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .lock_focus(true),
            );

            if state.request_input_focus {
                response.request_focus();
                state.request_input_focus = false;
            }

            // Keyboard navigation: Enter to submit, Up/Down for history
            if response.has_focus() {
                let input_state = ui.input(|i| i.clone());

                if input_state.key_pressed(Key::Enter) && !input_state.modifiers.shift {
                    submit_pressed = true;
                } else if input_state.key_pressed(Key::ArrowUp) {
                    navigate_history(state, true);
                } else if input_state.key_pressed(Key::ArrowDown) {
                    navigate_history(state, false);
                } else if input_state.key_pressed(Key::Escape) {
                    state.draft_input.clear();
                    state.history_pointer = None;
                }
            }

            ui.add_space(6.0);

            // 4. Action controls row
            ui.horizontal(|ui| {
                // Approximate toggle
                ui.checkbox(&mut state.force_numeric, "Approximate (≈)");

                ui.label(
                    RichText::new("Enter: evaluate · Shift+Enter: newline · Esc: clear")
                        .color(Colors::TEXT_MUTED)
                        .font(FontId::new(10.5, FontFamily::Proportional)),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Evaluate button
                    let can_submit = !state.draft_input.trim().is_empty() && !is_evaluating;
                    let eval_btn = Button::new(
                        RichText::new("Evaluate")
                            .color(if can_submit {
                                Colors::TEXT_PRIMARY
                            } else {
                                Colors::TEXT_MUTED
                            })
                            .font(FontId::new(12.5, FontFamily::Proportional))
                            .strong(),
                    )
                    .fill(if can_submit {
                        Colors::ACCENT_PRIMARY
                    } else {
                        Colors::INPUT_BG
                    })
                    .corner_radius(6);

                    if (ui.add_enabled(can_submit, eval_btn).clicked() || submit_pressed)
                        && can_submit
                    {
                        let input = state.draft_input.trim().to_string();
                        state.draft_input.clear();
                        state.history_pointer = None;
                        on_submit(input, state.force_numeric);
                    }

                    // Cancel button (prominent when calculating)
                    if is_evaluating {
                        let cancel_btn = Button::new(
                            RichText::new("Cancel")
                                .color(Colors::TEXT_PRIMARY)
                                .font(FontId::new(12.0, FontFamily::Proportional)),
                        )
                        .fill(Colors::ERROR_RED_BG)
                        .stroke(Stroke::new(1.0_f32, Colors::ERROR_RED))
                        .corner_radius(6);

                        if ui.add(cancel_btn).clicked() {
                            on_interrupt();
                        }
                    }
                });
            });
        });
}

fn render_live_lowering_preview(
    ui: &mut Ui,
    draft: &str,
    refs: &friendly_compiler::TapeReferenceTable,
) {
    let trimmed = draft.trim();
    if trimmed.is_empty() || trimmed.contains('\n') {
        return;
    }

    match friendly_compiler::compile_with_refs(trimmed, refs) {
        friendly_compiler::CompileResult::Success {
            generated_sage,
            required_variables,
        } => {
            if generated_sage != trimmed {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Sage:")
                            .color(Colors::TEXT_MUTED)
                            .font(FontId::new(11.0, FontFamily::Proportional)),
                    );
                    ui.label(
                        RichText::new(&generated_sage)
                            .color(Colors::ACCENT_PURPLE)
                            .font(FontId::new(11.5, FontFamily::Monospace)),
                    );

                    if !required_variables.is_empty() {
                        let vars_str = required_variables.join(", ");
                        ui.label(
                            RichText::new(format!("(vars: {})", vars_str))
                                .color(Colors::TEXT_MUTED)
                                .font(FontId::new(10.5, FontFamily::Proportional)),
                        );
                    }
                });
            }
        }
        friendly_compiler::CompileResult::Bypass { .. } => {}
        friendly_compiler::CompileResult::Error(err) => {
            ui.horizontal(|ui| {
                let mut msg = format!("Hint: {}", err.message);
                if let Some(sug) = &err.suggestion {
                    msg.push_str(&format!(" ({})", sug));
                }
                ui.label(
                    RichText::new(msg)
                        .color(Colors::NUMERIC_AMBER)
                        .font(FontId::new(11.0, FontFamily::Proportional))
                        .italics(),
                );
            });
        }
        friendly_compiler::CompileResult::Ambiguous { candidates } => {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Ambiguous (candidates: {})", candidates.join(", ")))
                        .color(Colors::NUMERIC_AMBER)
                        .font(FontId::new(11.0, FontFamily::Proportional))
                        .italics(),
                );
            });
        }
    }
}

fn render_formula_chips(ui: &mut Ui, draft: &mut String) -> bool {
    let chips = [
        ("factor", "factor(EXPR)"),
        ("expand", "expand(EXPR)"),
        ("diff", "diff(EXPR, x)"),
        ("integral", "integral(EXPR, x)"),
        ("solve", "solve(EXPR == 0, x)"),
        ("matrix", "[1 2; 3 4]"),
        ("plot", "plot(sin(x), -pi..pi)"),
        ("taylor", "taylor(EXPR, x, 0, 5)"),
        ("sum", "sum(k^2, k, 1..10)"),
    ];

    let mut clicked = false;

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Templates:")
                .color(Colors::TEXT_MUTED)
                .font(FontId::new(10.5, FontFamily::Proportional)),
        );

        for (label, snippet) in chips {
            if ui
                .add(
                    Button::new(
                        RichText::new(label)
                            .color(Colors::TEXT_SECONDARY)
                            .font(FontId::new(11.0, FontFamily::Proportional)),
                    )
                    .fill(Colors::INPUT_BG)
                    .corner_radius(4),
                )
                .clicked()
            {
                clicked = true;
                if draft.is_empty() {
                    *draft = snippet.to_string();
                } else {
                    *draft = format!("{}({})", label, draft.trim());
                }
            }
        }
    });

    clicked
}

fn navigate_history(state: &mut SessionState, up: bool) {
    if state.history.is_empty() {
        return;
    }

    let len = state.history.len();
    if up {
        match state.history_pointer {
            None => {
                state.history_pointer = Some(len - 1);
                state.draft_input = state.history[len - 1].clone();
            }
            Some(idx) if idx > 0 => {
                let next = idx - 1;
                state.history_pointer = Some(next);
                state.draft_input = state.history[next].clone();
            }
            _ => {}
        }
    } else {
        match state.history_pointer {
            Some(idx) if idx + 1 < len => {
                let next = idx + 1;
                state.history_pointer = Some(next);
                state.draft_input = state.history[next].clone();
            }
            Some(_) => {
                state.history_pointer = None;
                state.draft_input.clear();
            }
            None => {}
        }
    }
}
