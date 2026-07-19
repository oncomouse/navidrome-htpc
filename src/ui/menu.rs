use eframe::egui;
use crate::state::{AppState, View, FocusZone};
use crate::theme::*;

pub fn render(ctx: &egui::Context, state: &mut AppState) {
    let menu_items = ["Search", "Settings", "Now Playing"];

    let screen_bottom = ctx.screen_rect().max.y;

    egui::Area::new(egui::Id::new("menu_area"))
        .fixed_pos(egui::pos2(16.0, screen_bottom - 60.0))
        .show(ctx, |ui| {
            if !state.focus.menu_expanded {
                // Collapsed: just the ☰ icon
                let focused = state.focus.zone == FocusZone::Menu;
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(48.0, 48.0), egui::Sense::click());
                let bg = if focused {
                    BG_FOCUS
                } else if resp.hovered() {
                    BG_HOVER
                } else {
                    egui::Color32::TRANSPARENT
                };
                if bg != egui::Color32::TRANSPARENT {
                    ui.painter().rect_filled(rect, 8.0, bg);
                }
                if focused {
                    ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(2.0, ACCENT));
                }
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "\u{2630}",
                    egui::TextStyle::Heading.resolve(ui.style()),
                    TEXT_PRIMARY,
                );
                if resp.clicked_by(egui::PointerButton::Primary)
                    || (state.focus.zone == FocusZone::Menu
                        && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                {
                    state.focus.menu_expanded = true;
                }
            } else {
                // Expanded: vertical flyout
                ui.vertical(|ui| {
                    for (i, label) in menu_items.iter().enumerate() {
                        let focused =
                            state.focus.zone == FocusZone::Menu && state.focus.menu_index == i;
                        let (rect, resp) =
                            ui.allocate_exact_size(egui::vec2(160.0, 40.0), egui::Sense::click());
                        let bg = if focused {
                            BG_FOCUS
                        } else if resp.hovered() {
                            BG_HOVER
                        } else {
                            BG_WIDGET
                        };
                        ui.painter().rect_filled(rect, 8.0, bg);
                        if focused {
                            ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(2.0, ACCENT));
                        }
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            label,
                            egui::TextStyle::Body.resolve(ui.style()),
                            TEXT_PRIMARY,
                        );
                        if resp.clicked_by(egui::PointerButton::Primary) {
                            match i {
                                0 => state.push_view(View::Search),
                                1 => state.push_view(View::Settings),
                                2 => state.push_view(View::NowPlaying),
                                _ => {}
                            }
                            state.focus.menu_expanded = false;
                        }
                    }
                });
            }
        });
}
