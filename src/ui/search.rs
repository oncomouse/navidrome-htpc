use eframe::egui;
use crate::state::{AppState, View, FocusZone};
use crate::ui::common;
use crate::theme::*;

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new("Search").size(28.0).color(TEXT_PRIMARY));
        ui.add_space(16.0);

        let resp = ui.add_sized(
            [ui.available_width() - 120.0, 48.0],
            egui::TextEdit::singleline(&mut state.search_query)
                .hint_text("Type to search...")
                .font(egui::TextStyle::Heading),
        );
        if state.focus.zone == FocusZone::Content && state.focus.content_row == 0 {
            resp.request_focus();
        }
    });

    ui.add_space(24.0);

    // Results
    if !state.search_results_artists.is_empty() {
        ui.label(egui::RichText::new("Artists").size(18.0).color(TEXT_PRIMARY));
        ui.add_space(4.0);
        for artist in &state.search_results_artists {
            ui.label(&artist.name);
        }
        ui.add_space(16.0);
    }
    if !state.search_results_albums.is_empty() {
        ui.label(egui::RichText::new("Albums").size(18.0).color(TEXT_PRIMARY));
        ui.add_space(4.0);
        for album in &state.search_results_albums {
            ui.label(format!("{} - {}", album.artist_name, album.name));
        }
        ui.add_space(16.0);
    }
    if !state.search_results_tracks.is_empty() {
        ui.label(egui::RichText::new("Songs").size(18.0).color(TEXT_PRIMARY));
        ui.add_space(4.0);
        let width = ui.available_width();
        // Clone to avoid borrow-checker issues when iterating tracks and
        // mutating state on click.
        let tracks: Vec<_> = state.search_results_tracks.iter().cloned().collect();
        for (i, track) in tracks.iter().enumerate() {
            let focused = state.focus.zone == FocusZone::Content && state.focus.content_row == i + 1;
            if common::render_track_row(ui, track, i, focused, false, width) {
                state.play_queue = vec![track.clone()];
                state.current_track_index = Some(0);
                state.is_playing = true;
                state.push_view(View::NowPlaying);
            }
        }
    }
    if state.search_query.len() >= 2
        && state.search_results_artists.is_empty()
        && state.search_results_albums.is_empty()
        && state.search_results_tracks.is_empty()
    {
        ui.label("No results");
    }
}
