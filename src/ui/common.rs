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

    // Album name (bottom 40px)
    let text_rect =
        egui::Rect::from_min_size(egui::pos2(rect.min.x, rect.min.y + 164.0), egui::vec2(160.0, 36.0));
    ui.painter().text(
        text_rect.min,
        egui::Align2::LEFT_TOP,
        &album.name,
        egui::TextStyle::Small.resolve(ui.style()),
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
