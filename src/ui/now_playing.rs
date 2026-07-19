use eframe::egui;
use crate::state::{AppState, FocusZone};
use crate::ui::common;
use crate::theme::*;

/// Now Playing view: large album art + track info + progress bar on top,
/// scrollable play queue below with the current track highlighted and
/// auto-scrolled to center.
pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    let current_idx = state.current_track_index.unwrap_or(0);
    // Clone the current track + queue up-front so we can iterate the queue
    // inside the ScrollArea closure without holding a borrow of `state`
    // (home.rs borrow pattern: we need `&mut state` for the click handler).
    let current_track = state.play_queue.get(current_idx).cloned();
    let queue: Vec<crate::subsonic::models::Track> = state.play_queue.clone();

    // ── Top: large album art + track info + progress bar ──────────────────
    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.add_space(60.0);
        // Large album art (200x200)
        let (art_rect, _) =
            ui.allocate_exact_size(egui::vec2(200.0, 200.0), egui::Sense::hover());
        ui.painter().rect_filled(art_rect, 12.0, BG_WIDGET);
        if let Some(ref track) = current_track {
            if let Some(tex) = state.cover_textures.get(&track.album_id) {
                ui.painter().image(
                    tex.id(),
                    art_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }

        ui.add_space(32.0);
        ui.vertical(|ui| {
            if let Some(ref track) = current_track {
                ui.label(
                    egui::RichText::new(&track.title)
                        .size(28.0)
                        .color(TEXT_PRIMARY),
                );
                ui.label(
                    egui::RichText::new(&track.artist_name)
                        .size(18.0)
                        .color(TEXT_SECONDARY),
                );
                ui.label(
                    egui::RichText::new(&track.album_name).color(TEXT_SECONDARY),
                );
            } else {
                ui.label("Nothing playing");
            }
            ui.add_space(16.0);
            // Progress bar (current_time / total_duration)
            let progress = if state.total_duration > 0.0 {
                (state.current_time / state.total_duration).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let bar_width = 400.0;
            let (bar_rect, _) =
                ui.allocate_exact_size(egui::vec2(bar_width, 8.0), egui::Sense::hover());
            ui.painter().rect_filled(bar_rect, 4.0, BG_WIDGET);
            let filled = egui::Rect::from_min_size(
                bar_rect.min,
                egui::vec2(bar_rect.width() * progress, bar_rect.height()),
            );
            ui.painter().rect_filled(filled, 4.0, ACCENT);
            // Time labels (current / total)
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{}:{:02}",
                    state.current_time as u32 / 60,
                    state.current_time as u32 % 60
                ));
                ui.add_space(bar_width - 80.0);
                ui.label(format!(
                    "{}:{:02}",
                    state.total_duration as u32 / 60,
                    state.total_duration as u32 % 60
                ));
            });
        });
    });

    // ── Below: play queue with auto-scroll to current track ───────────────
    ui.add_space(20.0);
    ui.label(
        egui::RichText::new("Play Queue")
            .size(18.0)
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);
    let width = ui.available_width();
    let mut scroll_to: Option<egui::Rect> = None;

    egui::ScrollArea::vertical()
        .id_salt("play_queue")
        .show(ui, |ui| {
            for (i, track) in queue.iter().enumerate() {
                let focused = state.focus.zone == FocusZone::Content
                    && state.focus.content_row == i;
                let is_current = i == current_idx;
                if common::render_track_row(ui, track, i, focused, is_current, width) {
                    // Click / Enter → jump to this track
                    state.current_track_index = Some(i);
                    state.is_playing = true;
                }
                if is_current {
                    scroll_to = Some(ui.max_rect());
                }
            }
        });

    // Auto-scroll to keep current track centered
    if let Some(rect) = scroll_to {
        ui.scroll_to_rect(rect, Some(egui::Align::Center));
    }
}
