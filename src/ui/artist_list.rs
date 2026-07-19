use eframe::egui;
use crate::state::{AppState, View, FocusZone};
use crate::theme::*;

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.add_space(40.0);
        ui.label(
            egui::RichText::new("Artists")
                .size(28.0)
                .color(TEXT_PRIMARY),
        );
        ui.add_space(24.0);
        // Sort dropdown (placeholder: shows current sort; clicking cycles)
        let sort_label = match state.artist_sort {
            crate::state::ArtistSort::Alphabetical => "Sort: A→Z",
            crate::state::ArtistSort::ByAlbumCount => "Sort: Album count",
        };
        if ui
            .add(egui::Button::new(sort_label).min_size(egui::vec2(180.0, 28.0)))
            .clicked()
        {
            state.artist_sort = match state.artist_sort {
                crate::state::ArtistSort::Alphabetical => {
                    crate::state::ArtistSort::ByAlbumCount
                }
                crate::state::ArtistSort::ByAlbumCount => {
                    crate::state::ArtistSort::Alphabetical
                }
            };
        }
    });

    ui.add_space(16.0);

    // Clone artists so we can iterate while mutating state on click.
    let mut artists = state.artists.clone();
    match state.artist_sort {
        crate::state::ArtistSort::Alphabetical => {
            artists.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        }
        crate::state::ArtistSort::ByAlbumCount => {
            artists.sort_by(|a, b| b.album_count.cmp(&a.album_count));
        }
    }

    let width = ui.available_width();
    egui::ScrollArea::vertical()
        .id_salt("artist_list")
        .show(ui, |ui| {
            if artists.is_empty() {
                ui.label(
                    egui::RichText::new("No artists loaded.")
                        .color(TEXT_SECONDARY),
                );
            }
            for (i, artist) in artists.iter().enumerate() {
                let focused = state.focus.zone == FocusZone::Content
                    && state.focus.content_row == i;
                let (rect, resp) = ui.allocate_exact_size(
                    egui::vec2(width, 56.0),
                    egui::Sense::click(),
                );
                let bg = if focused {
                    BG_FOCUS
                } else if resp.hovered() {
                    BG_HOVER
                } else {
                    BG_WIDGET
                };
                ui.painter().rect_filled(rect, 6.0, bg);
                if focused {
                    ui.painter()
                        .rect_stroke(rect, 6.0, egui::Stroke::new(2.0, ACCENT));
                }
                // Artist name
                ui.painter().text(
                    egui::pos2(rect.min.x + 20.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &artist.name,
                    egui::TextStyle::Body.resolve(ui.style()),
                    TEXT_PRIMARY,
                );
                // Album count (right)
                let count_str = format!("{} albums", artist.album_count);
                ui.painter().text(
                    egui::pos2(rect.max.x - 20.0, rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    &count_str,
                    egui::TextStyle::Small.resolve(ui.style()),
                    TEXT_SECONDARY,
                );

                if resp.clicked_by(egui::PointerButton::Primary) {
                    state.current_artist = Some(artist.clone());
                    state.push_view(View::ArtistDetail);
                }
            }
        });
}
