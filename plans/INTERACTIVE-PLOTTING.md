# Interactive 2D Plotting & Extrema Snapping

## Overview
This document specifies the interactive 2D Cartesian plotting subsystem for Casette (implemented with pure Rust Woxi kernel evaluation and `egui_plot`).

Unlike static image plots (PNG/SVG), interactive plots provide:
1. **Dynamic Navigation:** Smooth canvas panning, dragging, mouse wheel zooming, boxed zoom, and quick "↺ Reset" to initial bounds.
2. **Continuous Cursor Inspection:** Hovering over the graph tracks the mouse horizontal coordinate $x$, projecting dotted horizontal guidelines to the Y axis and drawing intersection circles at each curve's $y(x)$.
3. **Magnetic Extrema Snapping:** Automatically snaps the inspected $x$ coordinate to nearby local minima and maxima within a screen pixel threshold (18px), displaying clear extremum badges (`▲ Local Max`, `▼ Local Min`) with exact coordinates.
4. **macOS-Styled Card Layout:** Embedded natively into the session tape with dark styling (`#141418` background, `#26262C` controls, subtle borders, and monospace coordinate pills).

---

## Architecture

```text
User Input: "plot sin(x), x=-pi..pi"
     │
     ▼
friendly-compiler desugars / normalizes to Woxi Plot[...]
     │
     ▼
casette-kernel (WoxiKernel):
  - Evaluates Plot[...] -> Expr::Graphics { source: PlotSource, .. }
  - Extracts sampled curve points (Vec<[f64; 2]>)
  - Runs compute_local_extrema() via 3-point sign-change analysis
  - Packages into PlotDataPayload (ranges, series, extrema)
     │
     ▼
casette-gui (render::interactive_plot):
  - Builds egui_plot::Plot with custom styling & controls
  - Draws curves (PlotPoints, Line, Points) and subtle extrema markers
  - On pointer hover: computes screen distance to all extrema
  - Snaps to closest extremum if within SNAP_THRESHOLD_PX (18px)
  - Interpolates y(active_x) on all curves with interpolate_y()
  - Renders vertical crosshair + horizontal axis guidelines + intersection dots
  - Updates persistent coordinate readout pill
```

---

## Protocol & Data Structures

Defined in `crates/casette-kernel/src/protocol.rs`:

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ExtremumKind {
    Maximum,
    Minimum,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LocalExtremum {
    pub x: f64,
    pub y: f64,
    pub kind: ExtremumKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PlotSeriesPayload {
    pub label: Option<String>,
    pub points: Vec<[f64; 2]>,
    pub color: [u8; 3],
    pub is_scatter: bool,
    pub extrema: Vec<LocalExtremum>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PlotDataPayload {
    pub title: Option<String>,
    pub x_label: Option<String>,
    pub y_label: Option<String>,
    pub x_range: [f64; 2],
    pub y_range: [f64; 2],
    pub series: Vec<PlotSeriesPayload>,
}
```

The payload is carried in `WorkerResponse.plot_data`.

---

## Kernel Extrema Detection Algorithm

In `crates/casette-kernel/src/woxi_kernel.rs`, `compute_local_extrema(points: &[[f64; 2]])`:
- Iterates over sampled interior points $p_i = (x_i, y_i)$ with neighbors $p_{i-1}$ and $p_{i+1}$.
- Checks finite differences:
  - If $y_i > y_{i-1} + 10^{-10}$ and $y_i > y_{i+1} + 10^{-10}$, classifies as `ExtremumKind::Maximum`.
  - If $y_i < y_{i-1} - 10^{-10}$ and $y_i < y_{i+1} - 10^{-10}$, classifies as `ExtremumKind::Minimum`.
- Boundary edge cases: ignores endpoints to avoid false edge extrema.

---

## GUI Interaction & Snapping Pipeline

In `crates/casette-gui/src/render/interactive_plot.rs`:
1. **Screen Proximity Metric:**
   Extremum coordinate $x_{ext}$ is projected to screen coordinates via `plot_ui.screen_from_plot(PlotPoint::new(x, 0.0)).x`.
   Screen distance $|x_{ext,screen} - x_{mouse,screen}|$ is measured against `SNAP_THRESHOLD_PX = 18.0`.
2. **Snapping State & Feedback:**
   - When snapped, the vertical crosshair aligns directly to the extremum.
   - The intersection point radius expands from `4.5px` to `6.0px`.
   - The coordinate pill displays `▲ Local Max: x = ..., y = ...` or `▼ Local Min: x = ..., y = ...`.
   - An on-screen toggle button allows turning snapping ON or OFF.
3. **Continuous Interpolation:**
   `interpolate_y(points, target_x)` uses binary search to locate the surrounding interval $[x_j, x_{j+1}]$ and performs linear interpolation for precise intersection coordinates.
4. **Tape Scroll Harmony:**
   `plot.allow_scroll(false)` ensures the session tape scrolls smoothly without the cursor getting caught in the plot widget during normal page scrolling. Zooming can still be done via boxed zoom or drag.
