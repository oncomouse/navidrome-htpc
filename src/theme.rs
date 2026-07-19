use egui::Color32;

pub const ACCENT: Color32 = Color32::from_rgb(167, 139, 250);
pub const BG_PANEL: Color32 = Color32::from_rgb(10, 10, 12);
pub const BG_WIDGET: Color32 = Color32::from_rgb(20, 20, 23);
pub const BG_HOVER: Color32 = Color32::from_rgb(30, 30, 34);
pub const BG_FOCUS: Color32 = Color32::from_rgb(45, 45, 50);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(240, 240, 240);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 160, 168);

pub fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = BG_PANEL;
    style.visuals.widgets.noninteractive.bg_fill = BG_WIDGET;
    style.visuals.widgets.hovered.bg_fill = BG_HOVER;
    style.visuals.selection.bg_fill = ACCENT;
    ctx.set_style(style);
}
