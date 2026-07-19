use eframe::egui;
use crate::state::{AppState, View, FocusZone};
use crate::focus::{handle_key, handle_arrow, FocusAction};
use crate::subsonic::SubsonicClient;

pub struct NavidromeApp {
    pub state: AppState,
    /// Subsonic client thread handle. `None` when no server is configured
    /// (the wizard hasn't run yet) — in that case the UI operates in an
    /// offline / placeholder mode.
    pub subsonic: Option<SubsonicClient>,
}

impl NavidromeApp {
    pub fn new(state: AppState, subsonic: Option<SubsonicClient>) -> Self {
        Self { state, subsonic }
    }
}

impl eframe::App for NavidromeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::theme::apply_theme(ctx);
        ctx.set_pixels_per_point(self.state.config.display.scale);

        ctx.memory_mut(|mem| {
            if let Some(id) = mem.focused() {
                mem.surrender_focus(id);
            }
        });

        // Keyboard dispatch
        let keys = ctx.input(|i| (
            i.key_pressed(egui::Key::Escape),
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::Space),
            i.key_pressed(egui::Key::ArrowUp),
            i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::ArrowLeft),
            i.key_pressed(egui::Key::ArrowRight),
        ));

        if keys.0 {
            // Clone focus to a local so we can pass both `&mut focus` and `&self.state`
            // to handle_key without a double-borrow of `self.state`. (handle_key's
            // app_state param is currently unused — reserved for later tasks — so
            // writing the clone back is a no-op for now.)
            let mut focus = self.state.focus.clone();
            let action = handle_key(&mut focus, egui::Key::Escape, &self.state);
            self.state.focus = focus;
            if action == FocusAction::Escape {
                if self.state.focus.menu_expanded {
                    self.state.focus.menu_expanded = false;
                } else {
                    self.state.pop_view();
                }
            }
        }
        // (Full keyboard dispatch expanded in later tasks)

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Navidrome HTPC");
            ui.label(format!("View: {:?} | Focus: {:?}", self.state.current_view(), self.state.focus.zone));
        });
    }
}
