mod app;
mod theme;

use eframe::egui;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Navidrome HTPC")
            .with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Navidrome HTPC",
        opts,
        Box::new(|_cc| Ok(Box::new(crate::app::NavidromeApp::default()))),
    )
}
