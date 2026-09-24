//! Design theme, typography scale, and color system for Casette.
//!
//! Implements a refined dark interface with clear visual hierarchy,
//! consistent typography sizing, rounded cards, and cassette-styled accents.

use egui::{
    Color32, Context, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Visuals,
};

/// Color tokens for Casette UI.
pub struct Colors;

impl Colors {
    // Backgrounds
    pub const WINDOW_BG: Color32 = Color32::from_rgb(24, 24, 27); // #18181b
    pub const CARD_BG: Color32 = Color32::from_rgb(32, 32, 36); // #202024
    pub const CARD_BG_HOVER: Color32 = Color32::from_rgb(38, 38, 44); // #26262c
    pub const CARD_BG_SELECTED: Color32 = Color32::from_rgb(42, 42, 50); // #2a2a32
    pub const SIDEBAR_BG: Color32 = Color32::from_rgb(20, 20, 23); // #141417
    pub const INPUT_BG: Color32 = Color32::from_rgb(28, 28, 32); // #1c1c20

    // Borders & Dividers
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(48, 48, 54); // #303036
    pub const BORDER_FOCUS: Color32 = Color32::from_rgb(56, 189, 248); // Sky blue
    pub const DIVIDER: Color32 = Color32::from_rgb(40, 40, 46);

    // Text hierarchy
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(244, 244, 245); // #f4f4f5
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(161, 161, 170); // #a1a1aa
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(113, 113, 122); // #71717a

    // Accents & Semantics
    pub const ACCENT_PRIMARY: Color32 = Color32::from_rgb(56, 189, 248); // Sky-400
    pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(167, 139, 250); // Purple-400
    pub const EXACT_GREEN: Color32 = Color32::from_rgb(52, 211, 153); // Emerald-400
    pub const EXACT_GREEN_BG: Color32 = Color32::from_rgb(6, 78, 59); // Emerald-900
    pub const NUMERIC_AMBER: Color32 = Color32::from_rgb(251, 191, 36); // Amber-400
    pub const NUMERIC_AMBER_BG: Color32 = Color32::from_rgb(120, 53, 15); // Amber-900
    pub const ERROR_RED: Color32 = Color32::from_rgb(248, 113, 113); // Red-400
    pub const ERROR_RED_BG: Color32 = Color32::from_rgb(69, 10, 10);
    pub const BADGE_BG: Color32 = Color32::from_rgb(39, 39, 42);
}

/// Applies Casette typography scales and visual styling to egui context.
pub fn apply_theme(ctx: &Context) {
    let mut style = (*ctx.style()).clone();

    // 1. Configure Typography Scale
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(17.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(13.5, FontFamily::Proportional)),
        (
            TextStyle::Monospace,
            FontId::new(12.5, FontFamily::Monospace),
        ),
        (
            TextStyle::Button,
            FontId::new(12.5, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(11.0, FontFamily::Proportional),
        ),
    ]
    .into();

    // 2. Configure Dark Visuals
    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(Colors::TEXT_PRIMARY);
    visuals.panel_fill = Colors::WINDOW_BG;
    visuals.window_fill = Colors::CARD_BG;
    visuals.extreme_bg_color = Colors::INPUT_BG;

    // Widgets (normal, hovered, active)
    visuals.widgets.noninteractive.bg_fill = Colors::CARD_BG;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Colors::BORDER_SUBTLE);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

    visuals.widgets.inactive.bg_fill = Colors::CARD_BG;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Colors::BORDER_SUBTLE);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

    visuals.widgets.hovered.bg_fill = Colors::CARD_BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Colors::ACCENT_PRIMARY);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);

    visuals.widgets.active.bg_fill = Colors::CARD_BG_SELECTED;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, Colors::ACCENT_PRIMARY);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);

    visuals.widgets.open.bg_fill = Colors::CARD_BG_HOVER;
    visuals.widgets.open.corner_radius = CornerRadius::same(6);

    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(56, 189, 248, 64);
    visuals.selection.stroke = Stroke::new(1.0_f32, Colors::ACCENT_PRIMARY);

    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.window_margin = Margin::same(12);

    ctx.set_style(style);
}
