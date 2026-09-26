//! Interactive 2D plotting widget using `egui_plot`.
//!
//! Provides panning, zooming, continuous mouse X tracking with intersecting Y values,
//! guideline crosshairs, and magnetic snapping to local minima and maxima.

use crate::theme::Colors;
use casette_kernel::{ExtremumKind, LocalExtremum, PlotDataPayload};
use egui::{
    Color32, CornerRadius, FontFamily, FontId, Frame, Margin,
    RichText, Stroke, Ui,
};
use egui_plot::{
    CoordinatesFormatter, Corner, Legend, Line, LineStyle, MarkerShape, Plot, PlotBounds,
    PlotPoint, PlotPoints, Points,
};

/// Distance in screen pixels within which mouse X snaps to a local extremum.
const SNAP_THRESHOLD_PX: f32 = 18.0;

/// State for an interactive plot card stored in egui persistent memory.
#[derive(Clone, Copy, Debug)]
pub struct InteractivePlotState {
    pub snap_enabled: bool,
    pub reset_requested: bool,
}

impl Default for InteractivePlotState {
    fn default() -> Self {
        Self {
            snap_enabled: true,
            reset_requested: false,
        }
    }
}

/// Linearly interpolates the Y coordinate for a given X from a sorted slice of points.
pub fn interpolate_y(points: &[[f64; 2]], target_x: f64) -> Option<f64> {
    if points.is_empty() {
        return None;
    }
    if target_x < points[0][0] || target_x > points[points.len() - 1][0] {
        return None;
    }

    // Binary search for the segment enclosing target_x
    let idx = match points.binary_search_by(|p| p[0].partial_cmp(&target_x).unwrap_or(std::cmp::Ordering::Equal)) {
        Ok(exact) => return Some(points[exact][1]),
        Err(insert_idx) => {
            if insert_idx == 0 {
                return Some(points[0][1]);
            }
            if insert_idx >= points.len() {
                return Some(points[points.len() - 1][1]);
            }
            insert_idx - 1
        }
    };

    let p0 = points[idx];
    let p1 = points[idx + 1];
    let dx = p1[0] - p0[0];
    if dx.abs() < 1e-12 {
        Some(p0[1])
    } else {
        let t = (target_x - p0[0]) / dx;
        Some(p0[1] + t * (p1[1] - p0[1]))
    }
}

/// Renders an interactive 2D Cartesian plot card.
pub fn render_interactive_plot_card(ui: &mut Ui, plot_data: &PlotDataPayload, row_id: usize) {
    let state_id = ui.make_persistent_id(format!("interactive_plot_state_{row_id}"));
    let mut state = ui.data_mut(|d| *d.get_temp_mut_or_default::<InteractivePlotState>(state_id));

    ui.add_space(6.0);

    // Card frame matching Casette dark aesthetic
    Frame::new()
        .fill(Color32::from_rgb(20, 20, 24))
        .stroke(Stroke::new(1.0_f32, Colors::BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            let mut active_ext = None;
            let mut active_coords: Option<(f64, Vec<f64>)> = None;

            // 1. Plot Header & Controls Bar
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("📈 2D Plot")
                        .font(FontId::new(13.0, FontFamily::Proportional))
                        .color(Colors::TEXT_PRIMARY),
                );

                ui.add_space(8.0);

                // Snap toggle button
                let snap_text = if state.snap_enabled {
                    "🧲 Snap Extrema: ON"
                } else {
                    "🧲 Snap Extrema: OFF"
                };
                let snap_color = if state.snap_enabled {
                    Colors::ACCENT_PRIMARY
                } else {
                    Colors::TEXT_MUTED
                };
                if ui
                    .button(
                        RichText::new(snap_text)
                            .font(FontId::new(11.0, FontFamily::Proportional))
                            .color(snap_color),
                    )
                    .clicked()
                {
                    state.snap_enabled = !state.snap_enabled;
                }

                // Reset view button
                if ui
                    .button(
                        RichText::new("↺ Reset")
                            .font(FontId::new(11.0, FontFamily::Proportional))
                            .color(Colors::TEXT_SECONDARY),
                    )
                    .on_hover_text("Reset plot view to initial domain and range")
                    .clicked()
                {
                    state.reset_requested = true;
                }

                // Right-aligned status / coordinate readout
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let temp_coord_id = ui.make_persistent_id(format!("plot_last_coord_{row_id}"));
                    let readout: Option<String> = ui.data(|d| d.get_temp(temp_coord_id));

                    if let Some(text) = readout {
                        ui.label(
                            RichText::new(text)
                                .font(FontId::new(12.0, FontFamily::Monospace))
                                .color(Colors::TEXT_PRIMARY),
                        );
                    } else {
                        ui.label(
                            RichText::new("Hover to inspect coordinates & extrema")
                                .font(FontId::new(11.0, FontFamily::Proportional))
                                .color(Colors::TEXT_MUTED)
                                .italics(),
                        );
                    }
                });
            });

            ui.add_space(6.0);

            // 2. Interactive Plot Canvas
            let plot_height = 280.0_f32;
            let available_width = ui.available_width().max(200.0);

            let plot_id = ui.make_persistent_id(format!("egui_plot_{row_id}"));
            let plot = Plot::new(plot_id)
                .height(plot_height)
                .width(available_width)
                .allow_drag(true)
                .allow_zoom(true)
                .allow_boxed_zoom(true)
                .allow_scroll(false) // Keep session tape scroll responsive
                .legend(Legend::default().position(Corner::RightTop))
                .coordinates_formatter(Corner::LeftBottom, CoordinatesFormatter::default());

            plot.show(ui, |plot_ui| {
                if state.reset_requested {
                    plot_ui.set_plot_bounds(PlotBounds::from_min_max(
                        [plot_data.x_range[0], plot_data.y_range[0]],
                        [plot_data.x_range[1], plot_data.y_range[1]],
                    ));
                    state.reset_requested = false;
                }

                // 2a. Draw curve lines & points
                for (s_idx, series) in plot_data.series.iter().enumerate() {
                    let default_colors = [
                        Colors::ACCENT_PRIMARY,
                        Colors::EXACT_GREEN,
                        Colors::NUMERIC_AMBER,
                        Colors::ACCENT_PURPLE,
                    ];
                    let line_color = if series.color != [0, 0, 0] {
                        Color32::from_rgb(series.color[0], series.color[1], series.color[2])
                    } else {
                        default_colors[s_idx % default_colors.len()]
                    };

                    let line_name = series
                        .label
                        .clone()
                        .unwrap_or_else(|| format!("y_{{{}}}", s_idx + 1));

                    if series.is_scatter {
                        plot_ui.points(
                            Points::new(series.points.clone())
                                .name(line_name)
                                .color(line_color)
                                .radius(3.0_f32)
                                .shape(MarkerShape::Circle),
                        );
                    } else {
                        let line_points = PlotPoints::new(series.points.clone());
                        plot_ui.line(
                            Line::new(line_points)
                                .name(line_name)
                                .color(line_color)
                                .width(2.0_f32),
                        );
                    }

                    // 2b. Draw subtle markers for local extrema
                    for ext in &series.extrema {
                        let ext_color = match ext.kind {
                            ExtremumKind::Maximum => Colors::NUMERIC_AMBER,
                            ExtremumKind::Minimum => Colors::ACCENT_PRIMARY,
                        };
                        let shape = match ext.kind {
                            ExtremumKind::Maximum => MarkerShape::Diamond,
                            ExtremumKind::Minimum => MarkerShape::Down,
                        };
                        plot_ui.points(
                            Points::new(vec![[ext.x, ext.y]])
                                .color(ext_color)
                                .radius(3.5_f32)
                                .shape(shape),
                        );
                    }
                }

                // 2c. Pointer interaction & Extrema Snapping
                if let Some(pointer_coord) = plot_ui.pointer_coordinate() {
                    let mouse_x = pointer_coord.x;
                    let mouse_screen_x = plot_ui.screen_from_plot(PlotPoint::new(mouse_x, 0.0)).x;

                    // Search for closest extremum within horizontal screen threshold
                    let mut closest_ext: Option<LocalExtremum> = None;
                    let mut min_screen_dist = SNAP_THRESHOLD_PX;

                    if state.snap_enabled {
                        for series in &plot_data.series {
                            for ext in &series.extrema {
                                let ext_screen_x = plot_ui.screen_from_plot(PlotPoint::new(ext.x, 0.0)).x;
                                let dist = (ext_screen_x - mouse_screen_x).abs();
                                if dist < min_screen_dist {
                                    min_screen_dist = dist;
                                    closest_ext = Some(ext.clone());
                                }
                            }
                        }
                    }

                    let active_x = if let Some(ref ext) = closest_ext {
                        ext.x
                    } else {
                        mouse_x
                    };

                    active_ext = closest_ext;

                    // Evaluate Y on all curves at active_x
                    let mut y_values = Vec::new();
                    let bounds = plot_ui.plot_bounds();

                    for (s_idx, series) in plot_data.series.iter().enumerate() {
                        if let Some(y) = interpolate_y(&series.points, active_x) {
                            y_values.push(y);

                            let default_colors = [
                                Colors::ACCENT_PRIMARY,
                                Colors::EXACT_GREEN,
                                Colors::NUMERIC_AMBER,
                                Colors::ACCENT_PURPLE,
                            ];
                            let color = if series.color != [0, 0, 0] {
                                Color32::from_rgb(series.color[0], series.color[1], series.color[2])
                            } else {
                                default_colors[s_idx % default_colors.len()]
                            };

                            // Horizontal guideline to Y axis
                            plot_ui.line(
                                Line::new(vec![[bounds.min()[0], y], [active_x, y]])
                                    .color(Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 70))
                                    .style(LineStyle::Dotted { spacing: 4.0_f32 })
                                    .width(1.0_f32),
                            );

                            // Intersection circle on the curve
                            let is_snapped = active_ext.is_some();
                            plot_ui.points(
                                Points::new(vec![[active_x, y]])
                                    .color(color)
                                    .radius(if is_snapped { 6.0_f32 } else { 4.5_f32 })
                                    .filled(true)
                                    .shape(MarkerShape::Circle),
                            );
                        }
                    }

                    // Vertical crosshair guideline across plot Y bounds
                    plot_ui.line(
                        Line::new(vec![[active_x, bounds.min()[1]], [active_x, bounds.max()[1]]])
                            .color(Color32::from_rgba_unmultiplied(220, 220, 240, 110))
                            .style(LineStyle::Dashed { length: 5.0_f32 })
                            .width(1.2_f32),
                    );

                    active_coords = Some((active_x, y_values));
                }
            });

            // 3. Update persistent coordinate readout badge
            let temp_coord_id = ui.make_persistent_id(format!("plot_last_coord_{row_id}"));
            if let Some((x, ys)) = active_coords {
                let badge_text = if let Some(ext) = active_ext {
                    match ext.kind {
                        ExtremumKind::Maximum => {
                            format!("▲ Local Max: x = {:.4}, y = {:.4}", ext.x, ext.y)
                        }
                        ExtremumKind::Minimum => {
                            format!("▼ Local Min: x = {:.4}, y = {:.4}", ext.x, ext.y)
                        }
                    }
                } else if !ys.is_empty() {
                    if ys.len() == 1 {
                        format!("x = {:.4}, y = {:.4}", x, ys[0])
                    } else {
                        let y_strs: Vec<String> = ys.iter().map(|y| format!("{:.3}", y)).collect();
                        format!("x = {:.4}, y = [{}]", x, y_strs.join(", "))
                    }
                } else {
                    format!("x = {:.4}", x)
                };
                ui.data_mut(|d| d.insert_temp(temp_coord_id, badge_text));
            } else {
                ui.data_mut(|d| d.remove::<String>(temp_coord_id));
            }
        });

    ui.data_mut(|d| d.insert_temp(state_id, state));
    ui.add_space(4.0);
}
