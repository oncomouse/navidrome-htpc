mod app;
mod config;
mod theme;

use eframe::egui;
use crate::config::Config;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::load().unwrap_or_default();
    let scale = config.display.scale;
    let server_configured = config.wizard.completed;

    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Navidrome HTPC")
            .with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Navidrome HTPC",
        opts,
        Box::new(move |_cc| {
            Ok(Box::new(crate::app::NavidromeApp::new(config.clone())))
        }),
    )
}
