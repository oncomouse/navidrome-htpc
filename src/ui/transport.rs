use crate::state::{AppState, FocusZone, TransportAction};
use crate::theme::*;
use eframe::egui;

/// How many frames the play/pause intent latch survives before being
/// force-cleared if mpv never reports a matching state. At ~60fps this is a
/// ~0.5s ceiling — long enough to cover mpv's IPC lag, short enough that a
/// command mpv silently drops won't wedge the icon indefinitely.
const INTENT_LATCH_FRAMES: u16 = 30;

pub fn render(ctx: &egui::Context, state: &mut AppState) {
    egui::TopBottomPanel::bottom("transport").show(ctx, |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            // Left spacer to avoid overlapping the hamburger menu
            // positioned at (16, screen_bottom - 60) in menu.rs.
            ui.add_space(64.0);
            // Transport buttons + progress slider are only shown when a track
            // is loaded (`current_track_index.is_some()`). When nothing is
            // loaded we hide them so the bar shows only Volume. We key off
            // `current_track_index` rather than `is_playing` so the controls
            // stay visible while paused mid-track (a loaded track is still
            // "something playing" in the user's mental model).
            if state.current_track_index.is_some() {
                let buttons = [
                    ("\u{23EE}", 0), // Prev
                    // Render the play/pause icon from the user's latched intent
                    // (`intended_playing`) when present, falling back to the
                    // mpv-derived `is_playing`. mpv's IPC lags a click by 1-2
                    // frames, so reading `is_playing` directly makes the icon
                    // flicker between ▶ and ⏸ right after a press; the latch
                    // shows the definitive requested state until mpv catches up.
                    (
                        if state.intended_playing.unwrap_or(state.is_playing) {
                            "\u{23F8}"
                        } else {
                            "\u{25B6}"
                        },
                        1,
                    ), // Play/Pause
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
            } else if state.focus.zone == FocusZone::Transport {
                // No transport controls are rendered when nothing is loaded,
                // so the Transport focus zone would trap keyboard focus
                // with nothing to land on. Bounce it back to Content.
                state.focus.zone = FocusZone::Content;
            }

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
            if let Some(track_idx) = state.current_track_index {
                if track_idx > 0 {
                    state.current_track_index = Some(track_idx - 1);
                    state.current_time = 0.0;
                    state.total_duration = 0.0;
                    state.is_playing = true;
                    state.pending_transport_action = Some(TransportAction::Previous);
                }
            }
        }
        1 => {
            // Play/Pause toggle
            if state.is_playing {
                state.is_playing = false;
                state.pending_transport_action = Some(TransportAction::Pause);
            } else {
                state.is_playing = true;
                state.pending_transport_action = Some(TransportAction::Play);
            }
            // Latch the requested play/pause state so the button icon shows
            // the definitive intent while mpv's IPC catches up (1-2 frames),
            // instead of flickering off the raw poll. app.rs clears the latch
            // once mpv's reported state converges or the frame budget expires.
            state.intended_playing = Some(state.is_playing);
            state.intent_frames_remaining = INTENT_LATCH_FRAMES;
        }
        2 => {
            // Stop
            // Remember which track we were on so Play can restart it after
            // Stop (mpv unloads the file on stop, so a plain Resume won't
            // produce audio).
            state.last_played_track_index = state.current_track_index;
            state.is_playing = false;
            state.current_track_index = None;
            state.current_time = 0.0;
            state.total_duration = 0.0;
            state.pending_transport_action = Some(TransportAction::Stop);
            // Stop clears current_track_index, so the transport buttons stop
            // rendering entirely — drop any stale play/pause intent latch.
            state.intended_playing = None;
            state.intent_frames_remaining = 0;
        }
        3 => {
            // Next — advance to next track in queue
            if let Some(track_idx) = state.current_track_index {
                let next = track_idx + 1;
                if next < state.play_queue.len() {
                    state.current_track_index = Some(next);
                    state.current_time = 0.0;
                    state.total_duration = 0.0;
                    state.is_playing = true;
                    state.pending_transport_action = Some(TransportAction::Next);
                }
            }
        }
        _ => {}
    }
}
