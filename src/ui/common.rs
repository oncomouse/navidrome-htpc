use eframe::egui;
use crate::state::AppState;
use crate::theme::*;

/// Render a large section card (Artists, Albums, Playlists) — painter-based, click + focus
pub fn render_card(
    ui: &mut egui::Ui,
    label: &str,
    focused: bool,
) -> bool {
    let size = egui::vec2(200.0, 120.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());

    let bg = if focused {
        BG_FOCUS
    } else if resp.hovered() {
        BG_HOVER
    } else {
        BG_WIDGET
    };
    ui.painter().rect_filled(rect, 12.0, bg);
    if focused {
        ui.painter().rect_stroke(rect, 12.0, egui::Stroke::new(3.0, ACCENT));
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::TextStyle::Heading.resolve(ui.style()),
        TEXT_PRIMARY,
    );

    resp.clicked_by(egui::PointerButton::Primary)
}

/// Render an album thumbnail (cover art + name) — painter-based
pub fn render_album_thumbnail(
    ui: &mut egui::Ui,
    album: &crate::subsonic::models::Album,
    focused: bool,
    cover_texture: Option<&egui::TextureHandle>,
) -> bool {
    let size = egui::vec2(160.0, 200.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());

    let bg = if focused {
        BG_FOCUS
    } else if resp.hovered() {
        BG_HOVER
    } else {
        BG_WIDGET
    };
    ui.painter().rect_filled(rect, 8.0, bg);
    if focused {
        ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(3.0, ACCENT));
    }

    // Cover art area (top 160x160)
    let cover_rect = egui::Rect::from_min_size(rect.min, egui::vec2(160.0, 160.0));
    if let Some(tex) = cover_texture {
        ui.painter().image(
            tex.id(),
            cover_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    } else {
        ui.painter().rect_filled(cover_rect, 8.0, egui::Color32::from_rgb(40, 40, 45));
        ui.painter().text(
            cover_rect.center(),
            egui::Align2::CENTER_CENTER,
            "\u{266A}",
            egui::TextStyle::Heading.resolve(ui.style()),
            TEXT_SECONDARY,
        );
    }

    // Album name (bottom 40px, artist name below)
    let name_rect =
        egui::Rect::from_min_size(egui::pos2(rect.min.x, rect.min.y + 164.0), egui::vec2(160.0, 18.0));
    ui.painter().text(
        name_rect.min,
        egui::Align2::LEFT_TOP,
        &album.name,
        egui::TextStyle::Small.resolve(ui.style()),
        TEXT_PRIMARY,
    );
    // Artist name below album title
    let artist_rect =
        egui::Rect::from_min_size(egui::pos2(rect.min.x, rect.min.y + 182.0), egui::vec2(160.0, 16.0));
    ui.painter().text(
        artist_rect.min,
        egui::Align2::LEFT_TOP,
        &album.artist_name,
        egui::TextStyle::Body.resolve(ui.style()),
        TEXT_SECONDARY,
    );

    resp.clicked_by(egui::PointerButton::Primary)
}

/// Render a detail-view header action button (Play / Shuffle / Add to Queue).
///
/// These buttons live outside egui's native widget focus (which `app.rs`
/// surrenders every frame), so we paint them manually and drive the keyboard
/// highlight from the custom focus system's `Header` zone. Returns the
/// mouse-clicked bool; keyboard activation is handled centrally in `app.rs`.
pub fn render_header_button(
    ui: &mut egui::Ui,
    label: &str,
    width: f32,
    focused: bool,
) -> bool {
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(width, 36.0), egui::Sense::click());
    let bg = if focused {
        BG_FOCUS
    } else if resp.hovered() {
        BG_HOVER
    } else {
        BG_WIDGET
    };
    ui.painter().rect_filled(rect, 8.0, bg);
    if focused {
        ui.painter()
            .rect_stroke(rect, 8.0, egui::Stroke::new(2.0, ACCENT));
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::TextStyle::Button.resolve(ui.style()),
        TEXT_PRIMARY,
    );
    resp.clicked_by(egui::PointerButton::Primary)
}

/// Render a track row in a list — painter-based
pub fn render_track_row(
    ui: &mut egui::Ui,
    track: &crate::subsonic::models::Track,
    index: usize,
    focused: bool,
    is_current: bool,
    width: f32,
) -> bool {
    let height = 48.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());

    let bg = if is_current {
        egui::Color32::from_rgba_premultiplied(
            ACCENT.r(),
            ACCENT.g(),
            ACCENT.b(),
            25,
        )
    } else if focused {
        BG_FOCUS
    } else if resp.hovered() {
        BG_HOVER
    } else {
        egui::Color32::TRANSPARENT
    };
    if bg != egui::Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, 4.0, bg);
    }
    if focused {
        ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(2.0, ACCENT));
    }

    // Track number or \u{25B6} for current
    let prefix = if is_current {
        "\u{25B6}"
    } else {
        &format!(
            "{}",
            track.track_number.unwrap_or(index as u32 + 1)
        )
    };
    ui.painter().text(
        egui::pos2(rect.min.x + 16.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        prefix,
        egui::TextStyle::Body.resolve(ui.style()),
        if is_current { ACCENT } else { TEXT_SECONDARY },
    );

    // Title
    ui.painter().text(
        egui::pos2(rect.min.x + 60.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &track.title,
        egui::TextStyle::Body.resolve(ui.style()),
        TEXT_PRIMARY,
    );

    // Duration (right-aligned)
    let dur = format!(
        "{}:{:02}",
        track.duration_secs / 60,
        track.duration_secs % 60
    );
    ui.painter().text(
        egui::pos2(rect.max.x - 16.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        &dur,
        egui::TextStyle::Small.resolve(ui.style()),
        TEXT_SECONDARY,
    );

    resp.clicked_by(egui::PointerButton::Primary)
}

/// Render a toast notification
pub fn render_toasts(ui: &mut egui::Ui, state: &mut AppState) {
    let delta = ui.ctx().input(|i| i.stable_dt);
    let mut to_remove = Vec::new();
    for (i, toast) in state.toasts.iter_mut().enumerate() {
        toast.ttl -= delta;
        if toast.ttl <= 0.0 {
            to_remove.push(i);
            continue;
        }
        let alpha = (toast.ttl / 3.0).min(1.0);
        let color = egui::Color32::from_rgba_premultiplied(
            30,
            30,
            34,
            (alpha * 255.0) as u8,
        );
        let rect = egui::Rect::from_min_size(
            egui::pos2(
                ui.min_rect().max.x - 320.0,
                ui.min_rect().min.y + 80.0 + i as f32 * 50.0,
            ),
            egui::vec2(300.0, 40.0),
        );
        ui.painter().rect_filled(rect, 8.0, color);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            &toast.message,
            egui::TextStyle::Body.resolve(ui.style()),
            TEXT_PRIMARY,
        );
    }
    for i in to_remove.iter().rev() {
        state.toasts.swap_remove(*i);
    }
}

// ── Context menu (Play Now / Shuffle / Add to Queue) ──────────────────────────
//
// Triggered by Right arrow on a card (album thumbnail) or a track row. Not used
// in detail views' headers where explicit Play/Shuffle/Add-to-Queue buttons
// already exist. `album_id` is set when the menu was opened on an album card;
// `track_index` is set when opened on a track row within the current album's
// track list. Both may be None when the menu is closed.

pub struct ContextMenuState {
    pub open: bool,
    /// Album the menu was opened on (None for track rows).
    pub album_id: Option<String>,
    /// Track index within `state.current_album_tracks` when opened on a row.
    pub track_index: Option<usize>,
    /// Currently highlighted item (0..3). Keyboard Up/Down adjusts this.
    pub selected: usize,
}

impl Default for ContextMenuState {
    fn default() -> Self {
        Self {
            open: false,
            album_id: None,
            track_index: None,
            selected: 0,
        }
    }
}

impl ContextMenuState {
    /// Open the menu anchored on an album card (e.g. from Home / AlbumList).
    pub fn open_for_album(&mut self, album_id: String) {
        self.open = true;
        self.album_id = Some(album_id);
        self.track_index = None;
        self.selected = 0;
    }

    /// Open the menu anchored on a track row within the current album's tracks.
    pub fn open_for_track(&mut self, track_index: usize) {
        self.open = true;
        self.album_id = None;
        self.track_index = Some(track_index);
        self.selected = 0;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.album_id = None;
        self.track_index = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContextMenuAction {
    PlayNow,
    Shuffle,
    AddToQueue,
}

/// Render the context-menu flyout as an `egui::Area`. Returns the selected
/// action (if any), in which case the caller is responsible for closing the
/// menu — `render_context_menu` already sets `open = false` on selection.
///
/// Keyboard navigation: Up/Down moves `selected`, Enter activates, Escape/Left
/// closes. Mouse clicks also activate.
pub fn render_context_menu(
    ctx: &egui::Context,
    menu: &mut ContextMenuState,
    pos: egui::Pos2,
) -> Option<ContextMenuAction> {
    if !menu.open {
        return None;
    }

    let mut action = None;
    let mut close = false;
    let num_items = 3usize;

    // Keyboard navigation (consumed on this frame).
    let (pressed_up, pressed_down, pressed_enter, pressed_escape, pressed_left) = ctx.input(|i| {
        (
            i.key_pressed(egui::Key::ArrowUp),
            i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::Escape),
            i.key_pressed(egui::Key::ArrowLeft),
        )
    });

    if pressed_up {
        if menu.selected > 0 {
            menu.selected -= 1;
        }
    }
    if pressed_down {
        if menu.selected + 1 < num_items {
            menu.selected += 1;
        }
    }
    if pressed_enter {
        action = Some(match menu.selected {
            0 => ContextMenuAction::PlayNow,
            1 => ContextMenuAction::Shuffle,
            _ => ContextMenuAction::AddToQueue,
        });
        close = true;
    }
    if pressed_escape || pressed_left {
        close = true;
    }

    egui::Area::new(egui::Id::new("context_menu"))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let items = ["\u{25b6} Play Now", "\u{1f500} Shuffle Play", "+ Add to Queue"];
            for (i, label) in items.iter().enumerate() {
                let (rect, resp) = ui.allocate_exact_size(
                    egui::vec2(200.0, 40.0),
                    egui::Sense::click(),
                );
                let is_selected = i == menu.selected;
                let bg = if is_selected || resp.hovered() {
                    BG_HOVER
                } else {
                    BG_WIDGET
                };
                ui.painter().rect_filled(rect, 8.0, bg);
                if is_selected {
                    ui.painter()
                        .rect_stroke(rect, 8.0, egui::Stroke::new(2.0, ACCENT));
                }
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::TextStyle::Body.resolve(ui.style()),
                    TEXT_PRIMARY,
                );
                if resp.clicked_by(egui::PointerButton::Primary) {
                    action = Some(match i {
                        0 => ContextMenuAction::PlayNow,
                        1 => ContextMenuAction::Shuffle,
                        _ => ContextMenuAction::AddToQueue,
                    });
                    close = true;
                }
            }
        });

    if close {
        menu.close();
    }

    action
}
