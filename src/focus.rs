use eframe::egui::Key;
use crate::state::{FocusState, FocusZone, AppState};

#[derive(Debug, Clone, PartialEq)]
pub enum FocusAction {
    None,
    Activate,           // Enter pressed on focused item
    Escape,             // Escape pressed
    PlayPauseToggle,    // Space / Play-Pause media key
    Stop,
    Next,
    Previous,
    VolumeUp,
    VolumeDown,
    Mute,
    SeekForward,
    SeekBackward,
}

pub fn handle_key(focus: &mut FocusState, key: Key, app_state: &AppState) -> FocusAction {
    match key {
        Key::Escape => FocusAction::Escape,
        Key::Enter => FocusAction::Activate,
        // NOTE: Space is NOT handled here. The app dispatches Space
        // directly to the context-menu opener in `app.rs` (keys.2 ->
        // maybe_open_context_menu_for_focus). This function is only ever
        // called with `Key::Escape`, so any Space arm would be dead and
        // misleading. Do not re-add `Key::Space => ...` here.
        _ => FocusAction::None,
    }
}

pub fn handle_arrow(
    focus: &mut FocusState,
    key: Key,
    num_content_rows: usize,
    num_transport_controls: usize,
) -> FocusAction {
    match focus.zone {
        FocusZone::Content => handle_content_arrow(focus, key, num_content_rows),
        FocusZone::Menu => handle_menu_arrow(focus, key),
        FocusZone::Transport => handle_transport_arrow(focus, key, num_transport_controls),
    }
}

fn handle_content_arrow(focus: &mut FocusState, key: Key, num_rows: usize) -> FocusAction {
    match key {
        Key::ArrowUp => {
            if focus.content_row > 0 {
                focus.content_row -= 1;
            }
            FocusAction::None
        }
        Key::ArrowDown => {
            if focus.content_row + 1 < num_rows {
                focus.content_row += 1;
            } else {
                focus.zone = FocusZone::Transport;
                focus.transport_index = 0;
            }
            FocusAction::None
        }
        Key::ArrowLeft => {
            if focus.content_col > 0 {
                focus.content_col -= 1;
            } else {
                focus.zone = FocusZone::Menu;
            }
            FocusAction::None
        }
        Key::ArrowRight => {
            focus.content_col += 1;
            FocusAction::None
        }
        _ => FocusAction::None,
    }
}

fn handle_menu_arrow(focus: &mut FocusState, key: Key) -> FocusAction {
    match key {
        Key::ArrowUp => {
            if focus.menu_expanded && focus.menu_index > 0 {
                focus.menu_index -= 1;
            }
            FocusAction::None
        }
        Key::ArrowDown => {
            if focus.menu_expanded && focus.menu_index < 2 {
                focus.menu_index += 1;
            } else {
                focus.zone = FocusZone::Transport;
                focus.transport_index = 0;
            }
            FocusAction::None
        }
        Key::ArrowRight => {
            if focus.menu_expanded {
                FocusAction::Activate
            } else {
                focus.zone = FocusZone::Content;
                focus.content_col = 0;
                FocusAction::None
            }
        }
        _ => FocusAction::None,
    }
}

fn handle_transport_arrow(focus: &mut FocusState, key: Key, num_controls: usize) -> FocusAction {
    match key {
        Key::ArrowUp => {
            focus.zone = FocusZone::Content;
            FocusAction::None
        }
        Key::ArrowLeft => {
            if focus.transport_index > 0 {
                focus.transport_index -= 1;
            }
            FocusAction::None
        }
        Key::ArrowRight => {
            if focus.transport_index + 1 < num_controls {
                focus.transport_index += 1;
            }
            FocusAction::None
        }
        _ => FocusAction::None,
    }
}
