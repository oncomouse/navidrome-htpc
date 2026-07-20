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
                if ui.add_sized([100.0, 36.0], egui::Button::new("\u{25B6} Play")).clicked() {
                    state.play_queue = state.current_playlist_tracks.clone();
                    state.current_track_index = Some(0);
                    state.is_playing = true;
                    state.push_view(View::NowPlaying);
                }
                if ui.add_sized([120.0, 36.0], egui::Button::new("\u{1f500} Shuffle")).clicked() {
                    let mut tracks = state.current_playlist_tracks.clone();
                    use rand::seq::SliceRandom;
                    let mut rng = rand::rng();
                    tracks.shuffle(&mut rng);
                    state.play_queue = tracks;
                    state.current_track_index = Some(0);
                    state.is_playing = true;
                    state.push_view(View::NowPlaying);
                }
                if ui.add_sized([140.0, 36.0], egui::Button::new("+ Add to Queue")).clicked() {
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
