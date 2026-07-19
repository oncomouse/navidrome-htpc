use crate::state::{AppState, View, FocusZone};
use crate::theme::*;
use eframe::egui;
use crate::ui::common;

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    let artist = match &state.current_artist {
        Some(a) => a.clone(),
        None => { ui.label("No artist selected"); return; }
    };

    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.add_space(40.0);
        ui.label(egui::RichText::new(&artist.name).size(28.0).color(TEXT_PRIMARY));
    });

    ui.add_space(16.0);
    ui.label(egui::RichText::new("Albums").size(18.0).color(TEXT_SECONDARY));
    ui.add_space(8.0);

    let albums = state.current_artist_albums.clone();
    let width = ui.available_width();
    let cols = (width / 180.0).floor().max(1.0) as usize;

    egui::ScrollArea::vertical().id_salt("artist_albums").show(ui, |ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::splat(16.0);
        let rows = (albums.len() + cols - 1) / cols;
        for row in 0..rows {
            ui.horizontal(|ui| {
                for col in 0..cols {
                    let idx = row * cols + col;
                    if idx >= albums.len() { break; }
                    let album = &albums[idx];
                    let focused = state.focus.zone == FocusZone::Content && state.focus.content_row == idx;
                    let tex = state.cover_textures.get(&album.id);
                    if common::render_album_thumbnail(ui, album, focused, tex) {
                        state.current_album = Some(album.clone());
                        state.current_album_tracks.clear();
                        state.push_view(View::AlbumDetail);
                    }
                }
            });
        }
    });
}
