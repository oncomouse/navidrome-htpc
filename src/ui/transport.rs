use eframe::egui;
use crate::state::{AppState, FocusZone};
use crate::theme::*;

pub fn render(ctx: &egui::Context, state: &mut AppState) {
    egui::TopBottomPanel::bottom("transport").show(ctx, |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            // Transport buttons (painter-based)
            let buttons = [
                ("\u{23EE}", 0), // Prev
                (if state.is_playing { "\u{23F8}" } else { "\u{25B6}" }, 1), // Play/Pause
                ("\u{23F9}", 2), // Stop
                ("\u{23ED}", 3), // Next
            ];
            for (label, idx) in buttons {
                let focused = state.focus.zone == FocusZone::Transport && state.focus.transport_index == idx;
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(48.0, 48.0), egui::Sense::click());
                let bg = if focused {
                    BG_FOCUS
                } else if resp.hovered() {
                    BG_HOVER
                } else {
                    egui::Color32::TRANSPARENT
                };
                if bg != egui::Color32::TRANSPARENT {
                    ui.painter().rect_filled(rect, 8.0, bg);
                }
                if focused {
                    ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(2.0, ACCENT));
                }
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::TextStyle::Heading.resolve(ui.style()),
                    TEXT_PRIMARY,
                );
                if resp.clicked_by(egui::PointerButton::Primary) {
                    handle_transport_click(idx, state);
                }
            }

            ui.add_space(24.0);

            // Progress slider
            let progress = if state.total_duration > 0.0 {
                state.current_time / state.total_duration
            } else {
                0.0
            };
            let mut seek = progress;
            ui.add_sized([200.0, 20.0], egui::Slider::new(&mut seek, 0.0..=1.0).show_value(false));
            if (seek - progress).abs() > 0.001 {
                // TODO: send seek to mpv
                state.current_time = seek * state.total_duration;
            }

            ui.add_space(24.0);

            // Volume slider
            let mut vol = state.volume;
            ui.add_sized([120.0, 20.0], egui::Slider::new(&mut vol, 0.0..=1.0).text("\u{1F50A}").show_value(false));
            state.volume = vol;
        });
        ui.add_space(8.0);
    });
}

fn handle_transport_click(idx: usize, state: &mut AppState) {
    match idx {
        0 => {
            // Previous — advance to previous track in queue
            if let Some(idx) = state.current_track_index {
                if idx > 0 {
                    state.current_track_index = Some(idx - 1);
                    state.current_time = 0.0;
                    state.is_playing = true;
                }
            }
        }
        1 => {
            // Play/Pause toggle
            state.is_playing = !state.is_playing;
        }
        2 => {
            // Stop
            state.is_playing = false;
            state.current_track_index = None;
            state.current_time = 0.0;
        }
        3 => {
            // Next — advance to next track in queue
            if let Some(idx) = state.current_track_index {
                let next = idx + 1;
                if next < state.play_queue.len() {
                    state.current_track_index = Some(next);
                    state.current_time = 0.0;
                    state.is_playing = true;
                }
            }
        }
        _ => {}
    }
}
