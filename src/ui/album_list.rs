use crate::state::{AppState, View, FocusZone, AlbumSort};
use crate::theme::*;
use eframe::egui;
use crate::ui::common;

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.add_space(40.0);
        ui.label(egui::RichText::new("Albums").size(28.0).color(TEXT_PRIMARY));
        ui.add_space(24.0);
        let sort_label = match state.album_sort {
            AlbumSort::Newest => "Sort: Newest",
            AlbumSort::AlphabeticalByName => "Sort: Name A-Z",
            AlbumSort::AlphabeticalByArtist => "Sort: Artist A-Z",
            AlbumSort::Random => "Sort: Random",
        };
        if ui.add(egui::Button::new(sort_label).min_size(egui::vec2(180.0, 28.0))).clicked() {
            state.album_sort = match state.album_sort {
                AlbumSort::Newest => AlbumSort::AlphabeticalByName,
                AlbumSort::AlphabeticalByName => AlbumSort::AlphabeticalByArtist,
                AlbumSort::AlphabeticalByArtist => AlbumSort::Random,
                AlbumSort::Random => AlbumSort::Newest,
            };
            state.albums.clear();
        }
    });

    ui.add_space(16.0);

    let mut albums = state.albums.clone();
    // Apply client-side sort to match the selected sort mode
    match state.album_sort {
        AlbumSort::Newest => { /* server already returns newest first */ }
        AlbumSort::AlphabeticalByName => {
            albums.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        }
        AlbumSort::AlphabeticalByArtist => {
            albums.sort_by(|a, b| a.artist_name.to_lowercase().cmp(&b.artist_name.to_lowercase())
                .then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        }
        AlbumSort::Random => {
            use rand::seq::SliceRandom;
            let mut rng = rand::rng();
            albums.shuffle(&mut rng);
        }
    }
    let width = ui.available_width();
    let cols = (width / 180.0).floor().max(1.0) as usize;

    egui::ScrollArea::vertical().id_salt("album_list").show(ui, |ui| {
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
