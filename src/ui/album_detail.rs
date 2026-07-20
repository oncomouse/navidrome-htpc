use crate::state::{AppState, FocusZone, View};
use crate::ui::common;
use eframe::egui;

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    let album = match &state.current_album {
        Some(a) => a.clone(),
        None => {
            ui.label("No album selected");
            return;
        }
    };

    // Header: album art + title + [Play] [Shuffle] [Add to Queue]
    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.add_space(40.0);
        // Album art (left)
        let (art_rect, _) =
            ui.allocate_exact_size(egui::vec2(120.0, 120.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(art_rect, 8.0, crate::theme::BG_WIDGET);
        if let Some(tex) = state.cover_textures.get(&album.id) {
            ui.painter().image(
                tex.id(),
                art_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }

        ui.add_space(24.0);
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(&album.name)
                    .size(24.0)
                    .color(crate::theme::TEXT_PRIMARY),
            );
            ui.label(
                egui::RichText::new(&album.artist_name)
                    .color(crate::theme::TEXT_SECONDARY),
            );
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                let play_focused = state.focus.zone == FocusZone::Header
                    && state.focus.header_index == 0;
                if common::render_header_button(ui, "\u{25B6} Play", 100.0, play_focused) {
                    // Replace queue with album tracks, start playing
                    state.play_queue = state.current_album_tracks.clone();
                    state.current_track_index = Some(0);
                    state.is_playing = true;
                    state.push_view(View::NowPlaying);
                    // (mpv play command sent in app.rs playback logic)
                }
                let shuffle_focused = state.focus.zone == FocusZone::Header
                    && state.focus.header_index == 1;
                if common::render_header_button(ui, "\u{1f500} Shuffle", 120.0, shuffle_focused) {
                    let mut tracks = state.current_album_tracks.clone();
                    use rand::seq::SliceRandom;
                    let mut rng = rand::rng();
                    tracks.shuffle(&mut rng);
                    state.play_queue = tracks;
                    state.current_track_index = Some(0);
                    state.is_playing = true;
                    state.push_view(View::NowPlaying);
                }
                let add_focused = state.focus.zone == FocusZone::Header
                    && state.focus.header_index == 2;
                if common::render_header_button(ui, "+ Add to Queue", 140.0, add_focused) {
                    state
                        .play_queue
                        .extend(state.current_album_tracks.clone());
                    state.toasts.push(crate::state::Toast {
                        message: format!(
                            "Added {} tracks to queue",
                            state.current_album_tracks.len()
                        ),
                        ttl: 3.0,
                    });
                }
            });
        });
    });

    // Track list
    ui.add_space(20.0);
    let width = ui.available_width();
    // Clone the tracks up-front so we can iterate without borrowing state
    // while mutating it inside the click handler (home.rs borrow pattern).
    let tracks = state.current_album_tracks.clone();
    egui::ScrollArea::vertical()
        .id_salt("album_tracks")
        .show(ui, |ui| {
            for (i, track) in tracks.iter().enumerate() {
                let focused = state.focus.zone == FocusZone::Content
                    && state.focus.content_row == i;
                let is_current = state.current_track_index == Some(i)
                    && state.current_view() == View::NowPlaying;
                if common::render_track_row(ui, track, i, focused, is_current, width) {
                    // Click → play from this track
                    state.play_queue = state.current_album_tracks[i..].to_vec();
                    state.current_track_index = Some(0);
                    state.is_playing = true;
                    state.push_view(View::NowPlaying);
                }
            }
        });
}
