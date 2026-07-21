use eframe::egui;
use crate::state::{AppState, View, FocusZone};
use crate::ui::common;

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    // Row 1: Section cards (Artists, Albums, Playlists)
    ui.add_space(40.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
        ui.add_space(60.0);
        let cards = ["Artists", "Albums", "Playlists"];
        for (i, label) in cards.iter().enumerate() {
            if i > 0 {
                ui.add_space(24.0);
            }
            let focused = state.focus.zone == FocusZone::Content
                && state.focus.content_row == 0
                && state.focus.content_col == i;
            if common::render_card(ui, label, focused) {
                match i {
                    0 => state.push_view(View::ArtistList),
                    1 => state.push_view(View::AlbumList),
                    2 => state.push_view(View::PlaylistList),
                    _ => {}
                }
            }
        }
    });

    // Row 2: Recently Added (horizontal scroll)
    ui.add_space(40.0);
    ui.label(
        egui::RichText::new("Recently Added")
            .size(20.0)
            .color(crate::theme::TEXT_PRIMARY),
    );
    ui.add_space(8.0);
    let mut scroll_to_added: Option<egui::Rect> = None;
    egui::ScrollArea::horizontal()
        .id_salt("recent_added")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let albums: Vec<_> = state.recent_albums.clone();
                let cover_ids: Vec<String> = albums.iter().map(|a| a.id.clone()).collect();
                for (i, album) in albums.iter().enumerate() {
                    if i > 0 {
                        ui.add_space(16.0);
                    }
                    let focused = state.focus.zone == FocusZone::Content
                        && state.focus.content_row == 1
                        && state.focus.content_col == i;
                    let tex = state.cover_textures.get(&cover_ids[i]);
                    let (clicked, rect) =
                        common::render_album_thumbnail(ui, album, focused, tex);
                    if clicked {
                        state.current_album = Some(album.clone());
                        state.push_view(View::AlbumDetail);
                    }
                    if focused {
                        scroll_to_added = Some(rect);
                    }
                }
            });
        });
    // Auto-scroll the row so the keyboard-focused thumbnail stays in view —
    // but only when focus actually moves to a different album (gated on the
    // last-scrolled position), so we don't fight the user's manual scrolling.
    if let Some(rect) = scroll_to_added {
        let key = (1usize, state.focus.content_col);
        if state.last_scrolled_home != Some(key) {
            ui.scroll_to_rect(rect, Some(egui::Align::Center));
            state.last_scrolled_home = Some(key);
        }
    }

    // Row 3: Recently Played (horizontal scroll)
    ui.add_space(32.0);
    ui.label(
        egui::RichText::new("Recently Played")
            .size(20.0)
            .color(crate::theme::TEXT_PRIMARY),
    );
    ui.add_space(8.0);
    let mut scroll_to_played: Option<egui::Rect> = None;
    egui::ScrollArea::horizontal()
        .id_salt("recent_played")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let albums: Vec<_> = state.recent_played.clone();
                let cover_ids: Vec<String> = albums.iter().map(|a| a.id.clone()).collect();
                for (i, album) in albums.iter().enumerate() {
                    if i > 0 {
                        ui.add_space(16.0);
                    }
                    let focused = state.focus.zone == FocusZone::Content
                        && state.focus.content_row == 2
                        && state.focus.content_col == i;
                    let tex = state.cover_textures.get(&cover_ids[i]);
                    let (clicked, rect) =
                        common::render_album_thumbnail(ui, album, focused, tex);
                    if clicked {
                        state.current_album = Some(album.clone());
                        state.push_view(View::AlbumDetail);
                    }
                    if focused {
                        scroll_to_played = Some(rect);
                    }
                }
            });
        });
    // Same gated auto-scroll for the Recently Played row.
    if let Some(rect) = scroll_to_played {
        let key = (2usize, state.focus.content_col);
        if state.last_scrolled_home != Some(key) {
            ui.scroll_to_rect(rect, Some(egui::Align::Center));
            state.last_scrolled_home = Some(key);
        }
    }
}
