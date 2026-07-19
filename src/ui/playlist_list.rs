use crate::state::{AppState, View, FocusZone};
use crate::theme::*;
use eframe::egui;

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.add_space(40.0);
        ui.label(egui::RichText::new("Playlists").size(28.0).color(TEXT_PRIMARY));
    });

    ui.add_space(16.0);

    let playlists = state.playlists.clone();
    let width = ui.available_width();

    egui::ScrollArea::vertical().id_salt("playlist_list").show(ui, |ui| {
        if playlists.is_empty() {
            ui.label(egui::RichText::new("No playlists found.").color(TEXT_SECONDARY));
        }
        for (i, playlist) in playlists.iter().enumerate() {
            let focused = state.focus.zone == FocusZone::Content && state.focus.content_row == i;
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 56.0), egui::Sense::click());
            let bg = if focused { BG_FOCUS } else if resp.hovered() { BG_HOVER } else { BG_WIDGET };
            ui.painter().rect_filled(rect, 6.0, bg);
            if focused { ui.painter().rect_stroke(rect, 6.0, egui::Stroke::new(2.0, ACCENT)); }
            ui.painter().text(egui::pos2(rect.min.x + 20.0, rect.center().y), egui::Align2::LEFT_CENTER, &playlist.name, egui::TextStyle::Body.resolve(ui.style()), TEXT_PRIMARY);
            let info = format!("{} songs", playlist.song_count);
            ui.painter().text(egui::pos2(rect.max.x - 20.0, rect.center().y), egui::Align2::RIGHT_CENTER, &info, egui::TextStyle::Small.resolve(ui.style()), TEXT_SECONDARY);
            if resp.clicked_by(egui::PointerButton::Primary) {
                state.current_playlist = Some(playlist.clone());
                state.current_playlist_tracks.clear();
                state.push_view(View::PlaylistDetail);
            }
        }
    });
}
