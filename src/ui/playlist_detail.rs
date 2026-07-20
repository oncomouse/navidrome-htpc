use crate::state::{AppState, FocusZone, View};
use crate::theme::*;
use crate::ui::common;
use eframe::egui;

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    let playlist = match &state.current_playlist {
        Some(p) => p.clone(),
        None => { ui.label("No playlist selected"); return; }
    };

    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.add_space(40.0);
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(&playlist.name).size(24.0).color(TEXT_PRIMARY));
            ui.label(egui::RichText::new(format!("{} songs", playlist.song_count)).color(TEXT_SECONDARY));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                let play_focused = state.focus.zone == FocusZone::Header
                    && state.focus.header_index == 0;
                if common::render_header_button(ui, "\u{25B6} Play", 100.0, play_focused) {
                    state.play_queue = state.current_playlist_tracks.clone();
                    state.current_track_index = Some(0);
                    state.is_playing = true;
                    state.push_view(View::NowPlaying);
                }
                let shuffle_focused = state.focus.zone == FocusZone::Header
                    && state.focus.header_index == 1;
                if common::render_header_button(ui, "\u{1f500} Shuffle", 120.0, shuffle_focused) {
                    let mut tracks = state.current_playlist_tracks.clone();
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
                    state.play_queue.extend(state.current_playlist_tracks.clone());
                    state.toasts.push(crate::state::Toast {
                        message: format!("Added {} tracks to queue", state.current_playlist_tracks.len()),
                        ttl: 3.0,
                    });
                }
            });
        });
    });

    ui.add_space(20.0);
    let width = ui.available_width();
    let tracks = state.current_playlist_tracks.clone();
    egui::ScrollArea::vertical().id_salt("playlist_tracks").show(ui, |ui| {
        for (i, track) in tracks.iter().enumerate() {
            let focused = state.focus.zone == FocusZone::Content && state.focus.content_row == i;
            let is_current = state.current_track_index == Some(i) && state.current_view() == View::NowPlaying;
            if common::render_track_row(ui, track, i, focused, is_current, width) {
                state.play_queue = state.current_playlist_tracks[i..].to_vec();
                state.current_track_index = Some(0);
                state.is_playing = true;
                state.push_view(View::NowPlaying);
            }
        }
    });
}
