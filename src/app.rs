use eframe::egui;

pub struct NavidromeApp {
    pub server_configured: bool,
}

impl Default for NavidromeApp {
    fn default() -> Self {
        Self {
            server_configured: false,
        }
    }
}

impl eframe::App for NavidromeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::theme::apply_theme(ctx);

        // Surrender native focus each frame (custom FocusZone system)
        ctx.memory_mut(|mem| {
            if let Some(id) = mem.focused() {
                mem.surrender_focus(id);
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Navidrome HTPC");
            if !self.server_configured {
                ui.label("Wizard goes here");
            } else {
                ui.label("Home goes here");
            }
        });
    }
}
