# Interactive 2D Plotting & Extrema Snapping

## Overview
This document specifies the interactive 2D Cartesian plotting subsystem for Casette (backed by the SageMath kernel over JSONL IPC and rendered with `egui_plot`).

Unlike static image plots (PNG/SVG), interactive plots provide:
1. **Dynamic Navigation:** Smooth canvas panning, dragging, mouse wheel zooming, boxed zoom, and quick "↺ Reset" to initial bounds.
2. **Continuous Cursor Inspection:** Hovering over the graph tracks the mouse horizontal coordinate $x$, projecting dotted horizontal guidelines to the Y axis and drawing intersection circles at each curve's $y(x)$.
3. **Magnetic Extrema Snapping:** Automatically snaps the inspected $x$ coordinate to nearby local minima and maxima within a screen pixel threshold (18px), displaying clear extremum badges (`▲ Local Max`, `▼ Local Min`) with exact coordinates.
4. **macOS-Styled Card Layout:** Embedded natively into the session tape with dark styling (`#141418` background, `#26262C` controls, subtle borders, and monospace coordinate pills).
5. **Static Artifact Parity:** Still outputs SVG and PNG artifacts to disk alongside `plot_data` so that session serialization and Markdown exports retain their plot figures.

---

## Architecture

```text
User Input: "plot sin(x), x=-pi..pi" or "plot #3, x=-10..10"
     │
     ▼
friendly-compiler:
  - Expands #ROW tokens to __casette_tape_refs[ROW]
  - Supports comma-free syntax via split_trailing_range ("plot sin(x) x=-pi..pi")
  - Lowers to Sage: "plot(sin(x), (x, -pi, pi))"
     │
     ▼
casette-kernel (SageKernel / worker.py IPC over JSONL):
  - Evaluates plot expression in SageMath -> Graphics object
  - worker.py extracts sampled curve coordinates (p.xdata, p.ydata) via _extract_plot_data
  - Still produces static SVG/PNG artifacts for notebook export and persistence
  - Packages structured plot_data into WorkerResponse envelope
  - casette-kernel runs compute_local_extrema() via 3-point sign-change analysis
     │
     ▼
casette-gui (render::interactive_plot):
  - Builds egui_plot::Plot with custom styling & controls
  - Slices curves into contiguous finite segments (skipping NaNs outside real domain)
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
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

In `crates/casette-kernel/src/protocol.rs`, `compute_local_extrema(points: &[[f64; 2]])`:
- Iterates over sampled interior points $p_i = (x_i, y_i)$ with neighbors $p_{i-1}$ and $p_{i+1}$.
- Checks finite differences:
  - If $y_i > y_{i-1}$ and $y_i > y_{i+1}$, classifies as `ExtremumKind::Maximum`.
  - If $y_i < y_{i-1}$ and $y_i < y_{i+1}$, classifies as `ExtremumKind::Minimum`.
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
   `interpolate_y(points, target_x)` uses binary search to locate the surrounding interval $[x_j, x_{j+1}]$ and performs linear interpolation for precise intersection coordinates, filtering out any non-finite sample values.
4. **Intuitive Navigation & Zoom Controls:**
   - `plot.allow_scroll(true)` enables natural mouse-wheel and trackpad pinch zooming directly over the plot.
   - `plot.allow_double_click_reset(true)` resets bounds back to initial domain and range on double-click.
   - On-screen `[ ➕ ]` (zoom in 1.3x) and `[ ➖ ]` (zoom out 0.77x) buttons provide discoverable touch/click controls.
   - `[ ↺ Reset ]` button restores default view.
   - Header hints guide the user: `"Pan: drag · Zoom: scroll/[+][−] · Reset: double-click"`.
5. **High-Contrast Dark-Mode Palette:**
   - Curve line widths are rendered at `2.5px` for crisp HiDPI rendering.
   - Automatically detects and replaces Sage's default white-background dark blue (`[0, 0, 204]`) with vivid `#38BDF8` sky blue, emerald `#34D399`, amber `#FBBF24`, and lilac `#C084FC`.

---

## Tape Reference Plotting & Domain Singularity Slicing

When referencing prior row results (e.g. `integrate 1/(1-x^2)` in row #1, then `plot #1, x=-10..10`):
1. **Tape Lowering:**
   - Tape references `#ROW` are expanded into `__casette_tape_refs[ROW]` via `friendly_compiler::compile_with_refs`.
   - The evaluated result of row #1 is registered in Sage's `__casette_tape_refs` dictionary: `__casette_tape_refs[1] = (1/2*log(x + 1) - 1/2*log(x - 1))`.
   - Sage natively generates the correct antiderivative $1/2\log(x+1) - 1/2\log(x-1)$, which evaluates on both $x < -1$ and $x > 1$.
2. **Contiguous Finite Segment Slicing:**
   - Functions like $\int \frac{1}{1-x^2} dx = \frac{1}{2}\log(x+1) - \frac{1}{2}\log(x-1)$ have discontinuities or non-real branches (e.g. $|x| < 1$).
   - Passing vertices with `NaN` coordinates directly into `egui_plot::Line` causes `epaint`'s `coarse_tessellation_culling` to cull the entire shape due to `NaN` bounding boxes.
   - `render_interactive_plot_card` partitions sampled points into contiguous finite segments (`x.is_finite() && y.is_finite()`). Only valid real segments are drawn, faithfully rendering the curve within its real domain while cleanly omitting out-of-domain regions.
