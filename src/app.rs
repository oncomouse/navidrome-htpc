use eframe::egui;
use crate::config::Config;

pub struct NavidromeApp {
    pub config: Config,
    pub server_configured: bool,
}

impl NavidromeApp {
    pub fn new(config: Config) -> Self {
        let server_configured = config.wizard.completed;
        Self { config, server_configured }
    }
}

impl eframe::App for NavidromeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::theme::apply_theme(ctx);
        ctx.set_pixels_per_point(self.config.display.scale);

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
