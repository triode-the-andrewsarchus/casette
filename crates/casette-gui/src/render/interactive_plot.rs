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
    CoordinatesFormatter, Corner, HLine, Legend, Line, LineStyle, MarkerShape, Plot, PlotBounds,
    PlotPoint, PlotPoints, Points, VLine,
};

/// Distance in screen pixels within which mouse X snaps to a local extremum.
const SNAP_THRESHOLD_PX: f32 = 18.0;

/// State for an interactive plot card stored in egui persistent memory.
#[derive(Clone, Copy, Debug)]
pub struct InteractivePlotState {
    pub initialized: bool,
    pub snap_enabled: bool,
    pub reset_requested: bool,
    pub zoom_delta: Option<f32>,
}

impl Default for InteractivePlotState {
    fn default() -> Self {
        Self {
            initialized: false,
            snap_enabled: true,
            reset_requested: false,
            zoom_delta: None,
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
        Ok(exact) => {
            let y = points[exact][1];
            return if y.is_finite() { Some(y) } else { None };
        }
        Err(insert_idx) => {
            if insert_idx == 0 {
                let y = points[0][1];
                return if y.is_finite() { Some(y) } else { None };
            }
            if insert_idx >= points.len() {
                let y = points[points.len() - 1][1];
                return if y.is_finite() { Some(y) } else { None };
            }
            insert_idx - 1
        }
    };

    let p0 = points[idx];
    let p1 = points[idx + 1];
    if !p0[1].is_finite() || !p1[1].is_finite() {
        return None;
    }
    let dx = p1[0] - p0[0];
    if dx.abs() < 1e-12 {
        if p0[1].is_finite() {
            Some(p0[1])
        } else {
            None
        }
    } else {
        let t = (target_x - p0[0]) / dx;
        let y = p0[1] + t * (p1[1] - p0[1]);
        if y.is_finite() {
            Some(y)
        } else {
            None
        }
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
                    "🧲 Snap (Extrema & Zeros): ON"
                } else {
                    "🧲 Snap (Extrema & Zeros): OFF"
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

                // Zoom in button
                if ui
                    .button(
                        RichText::new("➕")
                            .font(FontId::new(11.0, FontFamily::Proportional))
                            .color(Colors::TEXT_PRIMARY),
                    )
                    .on_hover_text("Zoom in (or scroll wheel up / pinch)")
                    .clicked()
                {
                    state.zoom_delta = Some(1.3_f32);
                }

                // Zoom out button
                if ui
                    .button(
                        RichText::new("➖")
                            .font(FontId::new(11.0, FontFamily::Proportional))
                            .color(Colors::TEXT_PRIMARY),
                    )
                    .on_hover_text("Zoom out (or scroll wheel down / pinch)")
                    .clicked()
                {
                    state.zoom_delta = Some(0.77_f32);
                }

                // Reset view button
                if ui
                    .button(
                        RichText::new("↺ Reset")
                            .font(FontId::new(11.0, FontFamily::Proportional))
                            .color(Colors::TEXT_SECONDARY),
                    )
                    .on_hover_text("Reset plot view to initial domain and range (or double-click canvas)")
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
                            RichText::new("Pan: drag · Zoom: scroll/[+][−] · Reset: double-click")
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

            let snapped_coords = std::rc::Rc::new(std::cell::Cell::new(None::<(f64, f64, Option<&'static str>)>));
            let snapped_coords_clone = snapped_coords.clone();

            let plot_id = ui.make_persistent_id(format!("egui_plot_{row_id}"));
            let plot = Plot::new(plot_id)
                .height(plot_height)
                .width(available_width)
                .allow_drag(true)
                .allow_zoom(true)
                .allow_scroll(true)
                .allow_double_click_reset(true)
                .allow_boxed_zoom(true)
                .legend(Legend::default().position(Corner::RightTop))
                .coordinates_formatter(Corner::LeftBottom, CoordinatesFormatter::new(move |point, _bounds| {
                    if let Some((sx, sy, label)) = snapped_coords_clone.get() {
                        if let Some(lbl) = label {
                            format!("x: {:.4}\ny: {:.4} ({})", sx, sy, lbl)
                        } else {
                            format!("x: {:.4}\ny: {:.4}", sx, sy)
                        }
                    } else {
                        format!("x: {:.4}\ny: {:.4}", point.x, point.y)
                    }
                }));

            let plot_resp = plot.show(ui, |plot_ui| {
                if !state.initialized || state.reset_requested {
                    let min_x = if plot_data.x_range[0].is_finite() {
                        plot_data.x_range[0]
                    } else {
                        -10.0
                    };
                    let max_x = if plot_data.x_range[1].is_finite() {
                        plot_data.x_range[1]
                    } else {
                        10.0
                    };
                    let (min_y, max_y) = if plot_data.y_range[0].is_finite()
                        && plot_data.y_range[1].is_finite()
                        && plot_data.y_range[0] < plot_data.y_range[1]
                    {
                        (plot_data.y_range[0], plot_data.y_range[1])
                    } else {
                        (-5.0, 5.0)
                    };
                    plot_ui.set_plot_bounds(PlotBounds::from_min_max(
                        [min_x, min_y],
                        [max_x, max_y],
                    ));
                    state.initialized = true;
                    state.reset_requested = false;
                } else if let Some(factor) = state.zoom_delta.take() {
                    let bounds = plot_ui.plot_bounds();
                    let center_x = (bounds.min()[0] + bounds.max()[0]) / 2.0;
                    let center_y = (bounds.min()[1] + bounds.max()[1]) / 2.0;
                    let half_w = (bounds.max()[0] - bounds.min()[0]) / 2.0 / (factor as f64);
                    let half_h = (bounds.max()[1] - bounds.min()[1]) / 2.0 / (factor as f64);
                    plot_ui.set_plot_bounds(PlotBounds::from_min_max(
                        [center_x - half_w, center_y - half_h],
                        [center_x + half_w, center_y + half_h],
                    ));
                }

                // 2a. Draw curve lines & points
                for (s_idx, series) in plot_data.series.iter().enumerate() {
                    let default_colors = [
                        Colors::ACCENT_PRIMARY,
                        Colors::EXACT_GREEN,
                        Colors::NUMERIC_AMBER,
                        Colors::ACCENT_PURPLE,
                    ];
                    let mut line_color = if series.color != [0, 0, 0] {
                        Color32::from_rgb(series.color[0], series.color[1], series.color[2])
                    } else {
                        default_colors[s_idx % default_colors.len()]
                    };

                    // Detect Sage's default dark blue [0, 0, 204] or any low-contrast dark color on dark theme
                    let lum = (line_color.r() as u32 * 299 + line_color.g() as u32 * 587 + line_color.b() as u32 * 114) / 1000;
                    if (line_color.r() <= 30 && line_color.g() <= 30 && line_color.b() >= 150) || lum < 70 {
                        line_color = default_colors[s_idx % default_colors.len()];
                    }

                    let line_name = series
                        .label
                        .clone()
                        .unwrap_or_else(|| format!("y_{{{}}}", s_idx + 1));

                    if series.is_scatter {
                        let finite_pts: Vec<[f64; 2]> = series
                            .points
                            .iter()
                            .copied()
                            .filter(|pt| pt[0].is_finite() && pt[1].is_finite())
                            .collect();
                        if !finite_pts.is_empty() {
                            plot_ui.points(
                                Points::new(finite_pts)
                                    .name(line_name)
                                    .color(line_color)
                                    .radius(3.5_f32)
                                    .shape(MarkerShape::Circle),
                            );
                        }
                    } else {
                        // Split into contiguous segments of finite points so NaN coordinates never
                        // break tessellation or cull the entire curve!
                        let mut current_segment = Vec::new();
                        let mut drawn_any = false;
                        for pt in &series.points {
                            if pt[0].is_finite() && pt[1].is_finite() {
                                current_segment.push(*pt);
                            } else if !current_segment.is_empty() {
                                let seg = std::mem::take(&mut current_segment);
                                if seg.len() >= 2 {
                                    let mut line = Line::new(PlotPoints::new(seg))
                                        .color(line_color)
                                        .width(2.5_f32);
                                    if !drawn_any {
                                        line = line.name(line_name.clone());
                                        drawn_any = true;
                                    }
                                    plot_ui.line(line);
                                } else if seg.len() == 1 {
                                    plot_ui.points(
                                        Points::new(seg)
                                            .color(line_color)
                                            .radius(2.5_f32)
                                            .shape(MarkerShape::Circle),
                                    );
                                }
                            }
                        }
                        if !current_segment.is_empty() {
                            if current_segment.len() >= 2 {
                                let mut line = Line::new(PlotPoints::new(current_segment))
                                    .color(line_color)
                                    .width(2.5_f32);
                                if !drawn_any {
                                    line = line.name(line_name.clone());
                                }
                                plot_ui.line(line);
                            } else if current_segment.len() == 1 {
                                plot_ui.points(
                                    Points::new(current_segment)
                                        .color(line_color)
                                        .radius(2.5_f32)
                                        .shape(MarkerShape::Circle),
                                );
                            }
                        }
                    }

                    // 2b. Draw subtle markers for local extrema & zero crossings
                    for ext in &series.extrema {
                        let ext_color = match ext.kind {
                            ExtremumKind::Maximum => Colors::NUMERIC_AMBER,
                            ExtremumKind::Minimum => Colors::ACCENT_PRIMARY,
                            ExtremumKind::ZeroCrossing => Colors::EXACT_GREEN,
                        };
                        let shape = match ext.kind {
                            ExtremumKind::Maximum => MarkerShape::Diamond,
                            ExtremumKind::Minimum => MarkerShape::Down,
                            ExtremumKind::ZeroCrossing => MarkerShape::Circle,
                        };
                        plot_ui.points(
                            Points::new(vec![[ext.x, ext.y]])
                                .color(ext_color)
                                .radius(3.5_f32)
                                .shape(shape),
                        );
                    }
                }

                // 2c. Pointer interaction & Extrema / Zero Snapping
                if let Some(pointer_coord) = plot_ui.pointer_coordinate() {
                    let mouse_x = pointer_coord.x;
                    let mouse_screen_x = plot_ui.screen_from_plot(PlotPoint::new(mouse_x, 0.0)).x;

                    // Search for closest extremum or zero crossing within horizontal screen threshold
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

                    if let Some(ref ext) = closest_ext {
                        let tag = match ext.kind {
                            ExtremumKind::Maximum => Some("Max"),
                            ExtremumKind::Minimum => Some("Min"),
                            ExtremumKind::ZeroCrossing => Some("Zero"),
                        };
                        snapped_coords.set(Some((ext.x, ext.y, tag)));
                    } else {
                        snapped_coords.set(None);
                    }

                    let active_x = if let Some(ref ext) = closest_ext {
                        ext.x
                    } else {
                        mouse_x
                    };

                    active_ext = closest_ext;

                    // Evaluate Y on all curves at active_x
                    let mut y_values = Vec::new();

                    for (s_idx, series) in plot_data.series.iter().enumerate() {
                        if let Some(y) = interpolate_y(&series.points, active_x) {
                            y_values.push(y);

                            let default_colors = [
                                Colors::ACCENT_PRIMARY,
                                Colors::EXACT_GREEN,
                                Colors::NUMERIC_AMBER,
                                Colors::ACCENT_PURPLE,
                            ];
                            let mut color = if series.color != [0, 0, 0] {
                                Color32::from_rgb(series.color[0], series.color[1], series.color[2])
                            } else {
                                default_colors[s_idx % default_colors.len()]
                            };
                            let lum = (color.r() as u32 * 299 + color.g() as u32 * 587 + color.b() as u32 * 114) / 1000;
                            if (color.r() <= 30 && color.g() <= 30 && color.b() >= 150) || lum < 70 {
                                color = default_colors[s_idx % default_colors.len()];
                            }

                            // Horizontal guideline to Y axis using HLine
                            plot_ui.hline(
                                HLine::new(y)
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

                    // Vertical crosshair guideline across plot Y bounds using VLine
                    plot_ui.vline(
                        VLine::new(active_x)
                            .color(Color32::from_rgba_unmultiplied(220, 220, 240, 110))
                            .style(LineStyle::Dashed { length: 5.0_f32 })
                            .width(1.2_f32),
                    );

                    active_coords = Some((active_x, y_values));
                }
            });

            // Consume scroll delta if pointer is over the plot canvas so parent tape doesn't scroll
            if plot_resp.response.contains_pointer() || plot_resp.response.hovered() {
                ui.ctx().input_mut(|i| {
                    i.smooth_scroll_delta = egui::Vec2::ZERO;
                    i.raw_scroll_delta = egui::Vec2::ZERO;
                });
            }

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
                        ExtremumKind::ZeroCrossing => {
                            format!("⨁ Zero / Root: x = {:.4}, y = 0", ext.x)
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
