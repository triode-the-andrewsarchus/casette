//! Bottom input pane with real-time Friendly Compiler lowering and formula chips.

use crate::model::{SessionState, WorkerStatus};
use crate::theme::Colors;
use egui::{Align, Button, FontFamily, FontId, Frame, Key, Layout, RichText, Stroke, TextEdit, Ui};
use friendly_compiler::ir::limit::LimitDirection;
use friendly_compiler::ir::FormulaIR;

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

            // 1b. Interactive FormulaIR structured parameter expander
            render_formula_ir_expander(ui, &mut state.draft_input);

            ui.add_space(6.0);

            // 2. Formula quick chips bar
            if render_formula_chips(ui, &mut state.draft_input) {
                state.request_input_focus = true;
            }

            // 2b. Live autocomplete suggestions bar (Tab to complete)
            if render_completion_suggestions(ui, &mut state.draft_input, &state.symbols) {
                state.request_input_focus = true;
            }

            ui.add_space(6.0);

            // 3. Multi-line Input Editor
            let is_evaluating = state.worker_status == WorkerStatus::Busy;
            let mut submit_pressed = false;

            let response = ui.add(
                TextEdit::multiline(&mut state.draft_input)
                    .id_salt("casette_main_input_editor")
                    .hint_text("Type math or command (e.g. derivative #1 wrt x, factor x^4 - 1, [1 2; 3 4]) - Enter to evaluate")
                    .font(FontId::new(14.0, FontFamily::Proportional))
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .lock_focus(true),
            );

            if state.request_input_focus {
                response.request_focus();
                state.request_input_focus = false;
            }

            // Keyboard navigation: Enter to submit, Tab to complete, Up/Down for history
            if response.has_focus() {
                let input_state = ui.input(|i| i.clone());

                let is_single_line = !state.draft_input.contains('\n');
                if input_state.key_pressed(Key::Tab) {
                    state.draft_input = state.draft_input.replace('\t', "");
                    if let Some((completed, _)) =
                        complete_input_prefix(&state.draft_input, &state.symbols)
                    {
                        state.draft_input = completed;
                    }
                } else if input_state.key_pressed(Key::Enter) && !input_state.modifiers.shift {
                    submit_pressed = true;
                } else if input_state.key_pressed(Key::ArrowUp)
                    && (is_single_line || input_state.modifiers.ctrl)
                {
                    navigate_history(state, true);
                } else if input_state.key_pressed(Key::ArrowDown)
                    && (is_single_line || input_state.modifiers.ctrl)
                {
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
                    RichText::new("Enter: evaluate · Tab: complete · Shift+Enter: newline · Esc: clear")
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

fn ir_field(ui: &mut Ui, label: &str, value: &mut String, width: f32, hint: &str) -> bool {
    ui.label(
        RichText::new(label)
            .color(Colors::TEXT_MUTED)
            .font(FontId::new(10.5, FontFamily::Proportional)),
    );
    ui.add(
        TextEdit::singleline(value)
            .desired_width(width)
            .hint_text(hint)
            .font(FontId::new(11.5, FontFamily::Monospace)),
    )
    .changed()
}

fn ir_opt_field(
    ui: &mut Ui,
    label: &str,
    value: &mut Option<String>,
    width: f32,
    hint: &str,
) -> bool {
    let mut text = value.clone().unwrap_or_default();
    if ir_field(ui, label, &mut text, width, hint) {
        if text.trim().is_empty() {
            *value = None;
        } else {
            *value = Some(text);
        }
        true
    } else {
        false
    }
}

fn render_formula_ir_expander(ui: &mut Ui, draft: &mut String) {
    if draft.contains('\n') {
        return;
    }
    let Some(mut ir) = FormulaIR::parse(draft) else {
        return;
    };

    let mut changed = false;

    Frame::new()
        .fill(Colors::INPUT_BG)
        .stroke(Stroke::new(1.0_f32, Colors::BORDER_SUBTLE))
        .corner_radius(4)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| match &mut ir {
                FormulaIR::Integral(int_ir) => {
                    ui.label(
                        RichText::new("∫ INTEGRAL")
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(10.5, FontFamily::Proportional))
                            .strong(),
                    );
                    changed |= ir_field(ui, "f:", &mut int_ir.expression, 120.0, "x^2");
                    if ir_field(ui, "var:", &mut int_ir.variable, 36.0, "x") {
                        int_ir.has_explicit_variable = !int_ir.variable.trim().is_empty();
                        changed = true;
                    }
                    changed |= ir_opt_field(ui, "from:", &mut int_ir.lower_bound, 48.0, "0");
                    changed |= ir_opt_field(ui, "to:", &mut int_ir.upper_bound, 48.0, "1");
                }
                FormulaIR::Derivative(der_ir) => {
                    ui.label(
                        RichText::new("d/dx DERIVATIVE")
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(10.5, FontFamily::Proportional))
                            .strong(),
                    );
                    changed |= ir_field(ui, "expr:", &mut der_ir.expression, 130.0, "sin(x)");
                    if ir_field(ui, "wrt:", &mut der_ir.variable, 40.0, "x") {
                        der_ir.has_explicit_variable = !der_ir.variable.trim().is_empty();
                        changed = true;
                    }
                    changed |= ir_opt_field(ui, "order:", &mut der_ir.order, 36.0, "1");
                }
                FormulaIR::Solve(solve_ir) => {
                    ui.label(
                        RichText::new("SOLVE")
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(10.5, FontFamily::Proportional))
                            .strong(),
                    );
                    changed |= ir_field(ui, "eq:", &mut solve_ir.equation, 140.0, "x^2 - 4 = 0");
                    if ir_field(ui, "for:", &mut solve_ir.variable, 60.0, "x") {
                        solve_ir.has_explicit_variable = !solve_ir.variable.trim().is_empty();
                        changed = true;
                    }
                }
                FormulaIR::Limit(lim_ir) => {
                    ui.label(
                        RichText::new("lim LIMIT")
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(10.5, FontFamily::Proportional))
                            .strong(),
                    );
                    changed |= ir_field(ui, "expr:", &mut lim_ir.expression, 120.0, "sin(x)/x");
                    changed |= ir_field(ui, "var:", &mut lim_ir.variable, 36.0, "x");
                    changed |= ir_field(ui, "→", &mut lim_ir.point, 48.0, "0");
                    let dir_label = match lim_ir.direction {
                        LimitDirection::Both => "two-sided",
                        LimitDirection::Right => "from right (+)",
                        LimitDirection::Left => "from left (-)",
                    };
                    if ui
                        .add(
                            Button::new(
                                RichText::new(dir_label)
                                    .color(Colors::TEXT_SECONDARY)
                                    .font(FontId::new(10.5, FontFamily::Proportional)),
                            )
                            .fill(Colors::CARD_BG)
                            .corner_radius(4),
                        )
                        .clicked()
                    {
                        lim_ir.direction = match lim_ir.direction {
                            LimitDirection::Both => LimitDirection::Right,
                            LimitDirection::Right => LimitDirection::Left,
                            LimitDirection::Left => LimitDirection::Both,
                        };
                        changed = true;
                    }
                }
                FormulaIR::Taylor(tay_ir) => {
                    ui.label(
                        RichText::new("TAYLOR")
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(10.5, FontFamily::Proportional))
                            .strong(),
                    );
                    changed |= ir_field(ui, "expr:", &mut tay_ir.expression, 110.0, "sin(x)");
                    changed |= ir_field(ui, "var:", &mut tay_ir.variable, 36.0, "x");
                    changed |= ir_field(ui, "at:", &mut tay_ir.center, 44.0, "0");
                    changed |= ir_field(ui, "order:", &mut tay_ir.order, 36.0, "5");
                }
                FormulaIR::SeriesRange(ser_ir) => {
                    ui.label(
                        RichText::new(ser_ir.kind.title())
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(10.5, FontFamily::Proportional))
                            .strong(),
                    );
                    changed |= ir_field(ui, "term:", &mut ser_ir.expression, 110.0, "k^2");
                    changed |= ir_field(ui, "index:", &mut ser_ir.index_variable, 36.0, "k");
                    changed |= ir_field(ui, "from:", &mut ser_ir.lower_bound, 44.0, "1");
                    changed |= ir_field(ui, "to:", &mut ser_ir.upper_bound, 48.0, "10");
                }
                FormulaIR::Plot(plot_ir) => {
                    ui.label(
                        RichText::new("PLOT")
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(10.5, FontFamily::Proportional))
                            .strong(),
                    );
                    changed |= ir_field(ui, "f:", &mut plot_ir.expression, 120.0, "sin(x)");
                    if ir_field(ui, "var:", &mut plot_ir.variable, 36.0, "x") {
                        plot_ir.has_explicit_variable = !plot_ir.variable.trim().is_empty();
                        changed = true;
                    }
                    changed |= ir_opt_field(ui, "min:", &mut plot_ir.lower_bound, 48.0, "-pi");
                    changed |= ir_opt_field(ui, "max:", &mut plot_ir.upper_bound, 48.0, "pi");
                }
                FormulaIR::ImplicitPlot(imp_ir) => {
                    ui.label(
                        RichText::new("IMPLICIT PLOT")
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(10.5, FontFamily::Proportional))
                            .strong(),
                    );
                    changed |= ir_field(ui, "eq:", &mut imp_ir.equation, 110.0, "x^2 + y^2 = 1");
                    changed |= ir_field(ui, "x:", &mut imp_ir.x_variable, 30.0, "x");
                    changed |= ir_opt_field(ui, "xmin:", &mut imp_ir.x_lower, 40.0, "-2");
                    changed |= ir_opt_field(ui, "xmax:", &mut imp_ir.x_upper, 40.0, "2");
                    changed |= ir_field(ui, "y:", &mut imp_ir.y_variable, 30.0, "y");
                    changed |= ir_opt_field(ui, "ymin:", &mut imp_ir.y_lower, 40.0, "-2");
                    changed |= ir_opt_field(ui, "ymax:", &mut imp_ir.y_upper, 40.0, "2");
                }
                FormulaIR::ParametricPlot(par_ir) => {
                    ui.label(
                        RichText::new("PARAMETRIC PLOT")
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(10.5, FontFamily::Proportional))
                            .strong(),
                    );
                    changed |= ir_field(ui, "x(t):", &mut par_ir.x_expression, 85.0, "cos(t)");
                    changed |= ir_field(ui, "y(t):", &mut par_ir.y_expression, 85.0, "sin(t)");
                    if ir_field(ui, "param:", &mut par_ir.variable, 32.0, "t") {
                        par_ir.has_explicit_variable = !par_ir.variable.trim().is_empty();
                        changed = true;
                    }
                    changed |= ir_opt_field(ui, "min:", &mut par_ir.lower_bound, 44.0, "0");
                    changed |= ir_opt_field(ui, "max:", &mut par_ir.upper_bound, 44.0, "2*pi");
                }
                FormulaIR::Unary(un_ir) => {
                    ui.label(
                        RichText::new(un_ir.kind.title())
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(10.5, FontFamily::Proportional))
                            .strong(),
                    );
                    changed |= ir_field(ui, "expr:", &mut un_ir.expression, 180.0, "x^4 - 1");
                }
                FormulaIR::MatrixOp(mat_ir) => {
                    ui.label(
                        RichText::new(mat_ir.kind.title())
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(10.5, FontFamily::Proportional))
                            .strong(),
                    );
                    changed |=
                        ir_field(ui, "matrix:", &mut mat_ir.expression, 180.0, "[1, 2; 3, 4]");
                }
                FormulaIR::VectorCalculus(vc_ir) => {
                    ui.label(
                        RichText::new(vc_ir.kind.title())
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(10.5, FontFamily::Proportional))
                            .strong(),
                    );
                    changed |= ir_field(ui, "expr:", &mut vc_ir.expression, 140.0, "x^2 + y^2");
                    if vc_ir.kind
                        != friendly_compiler::ir::vector_calculus::VectorCalculusKind::Hessian
                    {
                        changed |= ir_field(ui, "vars:", &mut vc_ir.variables, 70.0, "[x, y]");
                    }
                }
                FormulaIR::Subs(subs_ir) => {
                    ui.label(
                        RichText::new("SUBS")
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(10.5, FontFamily::Proportional))
                            .strong(),
                    );
                    changed |= ir_field(ui, "expr:", &mut subs_ir.expression, 130.0, "x^2 + y");
                    changed |= ir_field(ui, "where:", &mut subs_ir.bindings, 110.0, "x=2, y=3");
                }
                FormulaIR::Numeric(num_ir) => {
                    ui.label(
                        RichText::new("NUMERIC")
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(10.5, FontFamily::Proportional))
                            .strong(),
                    );
                    changed |= ir_field(ui, "expr:", &mut num_ir.expression, 140.0, "pi");
                    changed |= ir_field(ui, "digits:", &mut num_ir.digits, 48.0, "50");
                }
                FormulaIR::VarDeclaration(var_ir) => {
                    ui.label(
                        RichText::new("VAR")
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(10.5, FontFamily::Proportional))
                            .strong(),
                    );
                    changed |= ir_field(ui, "symbols:", &mut var_ir.names, 160.0, "x, y, z");
                }
                FormulaIR::Assume(ass_ir) => {
                    ui.label(
                        RichText::new("ASSUME")
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(10.5, FontFamily::Proportional))
                            .strong(),
                    );
                    changed |= ir_field(ui, "condition:", &mut ass_ir.condition, 160.0, "x > 0");
                }
                FormulaIR::Binary(bin_ir) => {
                    ui.label(
                        RichText::new(bin_ir.kind.title())
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(10.5, FontFamily::Proportional))
                            .strong(),
                    );
                    changed |= ir_field(ui, "a:", &mut bin_ir.first, 80.0, "10");
                    changed |= ir_field(ui, "b:", &mut bin_ir.second, 80.0, "3");
                }
                FormulaIR::Stats(stats_ir) => {
                    ui.label(
                        RichText::new(stats_ir.kind.command())
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(10.5, FontFamily::Proportional))
                            .strong(),
                    );
                    if stats_ir.positional.is_empty() {
                        stats_ir.positional.push(String::new());
                    }
                    for (idx, pos) in stats_ir.positional.iter_mut().enumerate() {
                        let lbl = if idx == 0 { "arg:" } else { "arg2:" };
                        changed |= ir_field(ui, lbl, pos, 54.0, "0");
                    }
                    for field in stats_ir.kind.named_fields() {
                        let mut val = stats_ir.named.get(field.key).cloned().unwrap_or_default();
                        if ir_field(
                            ui,
                            &format!("{}:", field.title),
                            &mut val,
                            48.0,
                            field.prompt,
                        ) {
                            if val.trim().is_empty() {
                                stats_ir.named.remove(field.key);
                            } else {
                                stats_ir.named.insert(field.key.to_string(), val);
                            }
                            changed = true;
                        }
                    }
                }
            });
        });

    if changed {
        *draft = ir.friendly_input();
    }
}

/// Builds the friendly command string when a quick-template chip is clicked.
#[must_use]
pub fn apply_template_chip(label: &str, current_draft: &str) -> String {
    let expr = current_draft.trim();
    if expr.is_empty() {
        return match label {
            "factor" => "factor x^4 - 1".to_string(),
            "expand" => "expand (x + 1)^3".to_string(),
            "diff" => "derivative sin(x) wrt x".to_string(),
            "integral" => "integral x^2, x=0..1".to_string(),
            "solve" => "solve x^2 + 5*x + 6 = 0 for x".to_string(),
            "matrix" => "matrix [1, 2; 3, 4]".to_string(),
            "plot" => "plot sin(x), x=-pi..pi".to_string(),
            "taylor" => "taylor sin(x), x=0, order=5".to_string(),
            "sum" => "sum k^2, k=1..10".to_string(),
            "limit" => "limit sin(x)/x, x->0".to_string(),
            other => format!("{other} x"),
        };
    }

    match label {
        "factor" => format!("factor {expr}"),
        "expand" => format!("expand {expr}"),
        "diff" => format!("derivative {expr} wrt x"),
        "integral" => format!("integral {expr}, x=0..1"),
        "solve" => {
            if expr.contains('=') {
                format!("solve {expr} for x")
            } else {
                format!("solve {expr} = 0 for x")
            }
        }
        "matrix" => {
            if expr.starts_with('[') {
                format!("matrix {expr}")
            } else {
                format!("matrix [{expr}]")
            }
        }
        "plot" => format!("plot {expr}, x=-pi..pi"),
        "taylor" => format!("taylor {expr}, x=0, order=5"),
        "sum" => format!("sum {expr}, k=1..10"),
        "limit" => format!("limit {expr}, x->0"),
        other => format!("{other} {expr}"),
    }
}

fn render_formula_chips(ui: &mut Ui, draft: &mut String) -> bool {
    let chips = [
        "factor", "expand", "diff", "integral", "solve", "matrix", "plot", "taylor", "sum", "limit",
    ];

    let mut clicked = false;

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Templates:")
                .color(Colors::TEXT_MUTED)
                .font(FontId::new(10.5, FontFamily::Proportional)),
        );

        for label in chips {
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
                *draft = apply_template_chip(label, draft);
            }
        }
    });

    clicked
}

/// Completes the trailing identifier token in `draft` against friendly command keywords
/// (when completing the leading command word) and defined session symbols.
///
/// Returns `Some((completed_draft, all_candidates))` when at least one candidate matches.
#[must_use]
pub fn complete_input_prefix(
    draft: &str,
    symbols: &[casette_kernel::SymbolPayload],
) -> Option<(String, Vec<String>)> {
    if draft.is_empty() || draft.ends_with(char::is_whitespace) {
        return None;
    }

    let token_start = draft
        .rfind(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let prefix = &draft[token_start..];
    if prefix.is_empty() || prefix.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return None;
    }

    let before_token = &draft[..token_start];
    let is_leading_word = before_token.trim().is_empty();
    let lower_prefix = prefix.to_ascii_lowercase();

    let mut candidates: Vec<(String, bool)> = Vec::new();

    if is_leading_word {
        for kw in friendly_compiler::Keyword::ALL {
            let canon = kw.canonical();
            if canon.starts_with(&lower_prefix) {
                candidates.push((canon.to_string(), true));
            }
        }
    }

    for sym in symbols {
        if sym.name.to_ascii_lowercase().starts_with(&lower_prefix)
            && !candidates.iter().any(|(c, _)| c == &sym.name)
        {
            candidates.push((sym.name.clone(), false));
        }
    }

    if !is_leading_word {
        for builtin in [
            "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "exp", "log",
            "sqrt", "abs", "pi", "infinity",
        ] {
            if builtin.starts_with(&lower_prefix) && !candidates.iter().any(|(c, _)| c == builtin) {
                candidates.push((builtin.to_string(), false));
            }
        }
    }

    if candidates.is_empty() {
        return None;
    }

    let names: Vec<String> = candidates.iter().map(|(c, _)| c.clone()).collect();

    if candidates.len() == 1 {
        let (chosen, is_cmd) = &candidates[0];
        let suffix = if *is_cmd { " " } else { "" };
        return Some((format!("{before_token}{chosen}{suffix}"), names));
    }

    // Compute longest common prefix across candidates
    let first = &candidates[0].0;
    let mut lcp_len = first.len();
    for (cand, _) in &candidates[1..] {
        lcp_len = first
            .chars()
            .zip(cand.chars())
            .take(lcp_len)
            .take_while(|(a, b)| a == b)
            .count();
    }
    let lcp: String = first.chars().take(lcp_len).collect();

    let completed = if lcp.len() > prefix.len() {
        format!("{before_token}{lcp}")
    } else {
        let (chosen, is_cmd) = &candidates[0];
        let suffix = if *is_cmd { " " } else { "" };
        format!("{before_token}{chosen}{suffix}")
    };

    Some((completed, names))
}

fn render_completion_suggestions(
    ui: &mut Ui,
    draft: &mut String,
    symbols: &[casette_kernel::SymbolPayload],
) -> bool {
    let Some((_, candidates)) = complete_input_prefix(draft, symbols) else {
        return false;
    };

    let token_start = draft
        .rfind(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let prefix = &draft[token_start..];
    if candidates.len() == 1 && candidates[0] == prefix {
        return false;
    }

    let before_token = draft[..token_start].to_string();
    let is_leading_word = before_token.trim().is_empty();
    let mut clicked = false;

    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new("Tab ↹:")
                .color(Colors::TEXT_MUTED)
                .font(FontId::new(10.5, FontFamily::Proportional)),
        );

        for cand in candidates.iter().take(8) {
            if ui
                .add(
                    Button::new(
                        RichText::new(cand)
                            .color(Colors::ACCENT_PRIMARY)
                            .font(FontId::new(11.0, FontFamily::Monospace)),
                    )
                    .fill(Colors::INPUT_BG)
                    .corner_radius(4),
                )
                .clicked()
            {
                let is_cmd = is_leading_word
                    && friendly_compiler::Keyword::ALL
                        .iter()
                        .any(|kw| kw.canonical() == cand);
                let suffix = if is_cmd { " " } else { "" };
                *draft = format!("{before_token}{cand}{suffix}");
                clicked = true;
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
