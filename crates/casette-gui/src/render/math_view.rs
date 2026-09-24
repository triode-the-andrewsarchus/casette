//! Rich mathematical expression, matrix grid, and plot rendering in egui.

use crate::model::TapeRow;
use crate::render::texture_cache::TextureCache;
use crate::theme::Colors;
use casette_kernel::WorkerResponse;
use egui::{vec2, Button, Color32, CornerRadius, FontFamily, FontId, Frame, RichText, Stroke, Ui};

/// Renders the result content of a tape row.
pub fn render_result(
    ui: &mut Ui,
    row: &TapeRow,
    texture_cache: &mut TextureCache,
    on_action: &mut dyn FnMut(&str, &str),
) {
    if row.is_evaluating {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(
                RichText::new("Evaluating in SageMath...")
                    .color(Colors::TEXT_SECONDARY)
                    .italics(),
            );
        });
        return;
    }

    let Some(result) = &row.result else {
        ui.label(
            RichText::new("No result available")
                .color(Colors::TEXT_MUTED)
                .italics(),
        );
        return;
    };

    if !result.ok {
        render_error_card(ui, result);
        return;
    }

    // Check if this row produced plot artifacts
    let has_plot_artifact = result.artifacts.iter().any(|a| a.r#type == "image");
    if has_plot_artifact || result.kind.as_deref() == Some("plot") {
        render_plot_card(ui, result, texture_cache);
    }

    // Render matrix or primary math/plain text
    if result.kind.as_deref() == Some("matrix") {
        render_matrix_view(ui, result);
    } else if result.kind.as_deref() != Some("plot") {
        render_primary_math(ui, result, texture_cache);
    }

    // Secondary approximation line if available
    if let Some(approx) = &result.approx {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("≈")
                    .color(Colors::NUMERIC_AMBER)
                    .font(FontId::new(14.0, FontFamily::Proportional)),
            );
            ui.label(
                RichText::new(approx)
                    .color(Colors::TEXT_SECONDARY)
                    .font(FontId::new(13.0, FontFamily::Monospace)),
            );
            if let Some(digits) = result.approx_digits {
                render_pill(
                    ui,
                    &format!("{} digits", digits),
                    Colors::NUMERIC_AMBER_BG,
                    Colors::NUMERIC_AMBER,
                );
            }
        });
    }

    // Action buttons bar
    render_action_bar(ui, row, result, on_action);
}

fn render_primary_math(ui: &mut Ui, result: &WorkerResponse, texture_cache: &mut TextureCache) {
    let ctx = ui.ctx().clone();

    // 1. Try to display pre-rendered math SVG artifact from worker
    let math_art_path = result
        .artifacts
        .iter()
        .find(|a| a.r#type == "math")
        .and_then(|a| a.path.as_deref());

    let math_texture = if let Some(path) = math_art_path {
        texture_cache.get_or_load(&ctx, path)
    } else if let Some(latex) = result.latex.as_deref() {
        texture_cache.get_or_render_latex(&ctx, latex)
    } else {
        None
    };

    if let Some(texture) = math_texture {
        let size = texture.size_vec2();
        // Texture is rasterized at 2.0x scale for crisp HiDPI typography
        let logical_w = size.x / 2.0;
        let logical_h = size.y / 2.0;

        let max_w = (ui.available_width() - 24.0).max(50.0);
        let (display_w, display_h) = if logical_w > max_w {
            let scale = max_w / logical_w;
            (max_w, logical_h * scale)
        } else {
            (logical_w, logical_h)
        };

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.image((texture.id(), vec2(display_w, display_h)));
        });
        ui.add_space(4.0);
    } else if let Some(plain) = &result.plain {
        if !plain.is_empty() {
            render_primary_text(ui, plain, result.latex.as_deref());
        }
    }
}

fn render_primary_text(ui: &mut Ui, plain: &str, _latex: Option<&str>) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(plain)
                .color(Colors::TEXT_PRIMARY)
                .font(FontId::new(16.0, FontFamily::Proportional))
                .strong(),
        );
    });
}

fn render_matrix_view(ui: &mut Ui, result: &WorkerResponse) {
    let plain = result.plain.as_deref().unwrap_or("");
    let lines: Vec<&str> = plain.lines().filter(|l| !l.trim().is_empty()).collect();

    if lines.is_empty() {
        return;
    }

    // Parse matrix lines like "[1  2]\n[3  4]"
    let mut matrix_rows: Vec<Vec<String>> = Vec::new();
    for line in &lines {
        let trimmed = line.trim().trim_start_matches('[').trim_end_matches(']');
        let cells: Vec<String> = trimmed.split_whitespace().map(|s| s.to_string()).collect();
        if !cells.is_empty() {
            matrix_rows.push(cells);
        }
    }

    let num_rows = matrix_rows.len();
    let num_cols = matrix_rows.first().map(|r| r.len()).unwrap_or(0);

    // Draw stylized matrix box
    Frame::new()
        .fill(Colors::INPUT_BG)
        .corner_radius(CornerRadius::same(6))
        .stroke(Stroke::new(1.0_f32, Colors::BORDER_SUBTLE))
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Left bracket
                ui.label(
                    RichText::new("[")
                        .color(Colors::ACCENT_PRIMARY)
                        .font(FontId::new(18.0, FontFamily::Proportional)),
                );

                // Grid of cells
                egui::Grid::new(format!("matrix_grid_{}", plain.len()))
                    .spacing(vec2(16.0, 6.0))
                    .show(ui, |ui| {
                        for row in &matrix_rows {
                            for cell in row {
                                let is_zero = cell == "0";
                                let text_color = if is_zero {
                                    Colors::TEXT_MUTED
                                } else {
                                    Colors::TEXT_PRIMARY
                                };

                                ui.label(
                                    RichText::new(cell)
                                        .color(text_color)
                                        .font(FontId::new(13.5, FontFamily::Monospace)),
                                );
                            }
                            ui.end_row();
                        }
                    });

                // Right bracket
                ui.label(
                    RichText::new("]")
                        .color(Colors::ACCENT_PRIMARY)
                        .font(FontId::new(18.0, FontFamily::Proportional)),
                );

                ui.add_space(8.0);
                render_pill(
                    ui,
                    &format!("{}×{} matrix", num_rows, num_cols),
                    Colors::BADGE_BG,
                    Colors::TEXT_MUTED,
                );
            });
        });
}

fn render_plot_card(ui: &mut Ui, result: &WorkerResponse, texture_cache: &mut TextureCache) {
    let ctx = ui.ctx().clone();
    for artifact in &result.artifacts {
        if artifact.r#type == "image" {
            if let Some(path) = &artifact.path {
                if let Some(texture) = texture_cache.get_or_load(&ctx, path) {
                    let size = texture.size_vec2();
                    let aspect = size.x / size.y.max(1.0);
                    let target_width = (460.0_f32).min(ui.available_width() - 20.0).max(100.0);
                    let target_height = target_width / aspect;

                    ui.add_space(6.0);
                    Frame::new()
                        .fill(Color32::from_rgb(255, 255, 255))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            ui.image((texture.id(), vec2(target_width, target_height)));
                        });
                    ui.add_space(6.0);
                }
            }
        }
    }
}

fn render_error_card(ui: &mut Ui, result: &WorkerResponse) {
    let err_type = result
        .error
        .as_ref()
        .map(|e| e.r#type.as_str())
        .unwrap_or("Error");
    let err_msg = result
        .error
        .as_ref()
        .map(|e| e.message.as_str())
        .unwrap_or_else(|| result.plain.as_deref().unwrap_or("Evaluation failed"));

    Frame::new()
        .fill(Colors::ERROR_RED_BG)
        .stroke(Stroke::new(1.0_f32, Colors::ERROR_RED))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("⚠")
                        .color(Colors::ERROR_RED)
                        .font(FontId::new(15.0, FontFamily::Proportional)),
                );
                ui.label(
                    RichText::new(format!("{}: {}", err_type, err_msg))
                        .color(Colors::TEXT_PRIMARY)
                        .font(FontId::new(13.0, FontFamily::Monospace)),
                );
            });
        });
}

fn render_action_bar(
    ui: &mut Ui,
    row: &TapeRow,
    result: &WorkerResponse,
    on_action: &mut dyn FnMut(&str, &str),
) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        // Quick copy buttons
        if let Some(plain) = &result.plain {
            if ui
                .add(
                    Button::new(
                        RichText::new("Copy").font(FontId::new(11.0, FontFamily::Proportional)),
                    )
                    .frame(false),
                )
                .on_hover_text("Copy plain text output")
                .clicked()
            {
                ui.ctx().copy_text(plain.clone());
            }
        }

        if let Some(latex) = &result.latex {
            if ui
                .add(
                    Button::new(
                        RichText::new("Copy LaTeX")
                            .font(FontId::new(11.0, FontFamily::Proportional)),
                    )
                    .frame(false),
                )
                .on_hover_text("Copy LaTeX expression")
                .clicked()
            {
                ui.ctx().copy_text(latex.clone());
            }
        }

        // Mathematical quick transformations
        let row_ref = format!("#{}", row.row_number);

        if ui
            .add(
                Button::new(
                    RichText::new("Differentiate")
                        .color(Colors::ACCENT_PRIMARY)
                        .font(FontId::new(11.0, FontFamily::Proportional)),
                )
                .frame(false),
            )
            .clicked()
        {
            on_action("diff", &format!("diff({}, x)", row_ref));
        }

        if ui
            .add(
                Button::new(
                    RichText::new("Integrate")
                        .color(Colors::ACCENT_PRIMARY)
                        .font(FontId::new(11.0, FontFamily::Proportional)),
                )
                .frame(false),
            )
            .clicked()
        {
            on_action("integral", &format!("integral({}, x)", row_ref));
        }

        if ui
            .add(
                Button::new(
                    RichText::new("Simplify")
                        .color(Colors::ACCENT_PRIMARY)
                        .font(FontId::new(11.0, FontFamily::Proportional)),
                )
                .frame(false),
            )
            .clicked()
        {
            on_action("simplify", &format!("simplify({})", row_ref));
        }
    });
}

fn render_pill(ui: &mut Ui, text: &str, bg_color: Color32, text_color: Color32) {
    Frame::new()
        .fill(bg_color)
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .color(text_color)
                    .font(FontId::new(10.5, FontFamily::Proportional)),
            );
        });
}
