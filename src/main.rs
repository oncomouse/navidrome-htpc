mod app;
mod config;
mod theme;
mod state;
mod focus;
mod subsonic;
mod mpv;
mod ui;

use eframe::egui;
use crate::config::Config;
use crate::state::{AppState, View};

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::load().unwrap_or_default();
    let server_configured = config.wizard.completed;

    let state = AppState {
        config: config.clone(),
        view_stack: vec![View::Home],
        focus: Default::default(),
        server_configured,
        recent_albums: Vec::new(),
        recent_played: Vec::new(),
        artists: Vec::new(),
        albums: Vec::new(),
        playlists: Vec::new(),
        current_album: None,
        current_album_tracks: Vec::new(),
        current_artist: None,
        current_artist_albums: Vec::new(),
        current_playlist: None,
        current_playlist_tracks: Vec::new(),
        search_query: String::new(),
        search_results_artists: Vec::new(),
        search_results_albums: Vec::new(),
        search_results_tracks: Vec::new(),
        play_queue: Vec::new(),
        current_track_index: None,
        is_playing: false,
        current_time: 0.0,
        total_duration: 0.0,
        volume: 0.75,
        toasts: Vec::new(),
        cover_textures: std::collections::HashMap::new(),
    };

    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Navidrome HTPC")
            .with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Navidrome HTPC",
        opts,
        Box::new(move |_cc| Ok(Box::new(crate::app::NavidromeApp::new(state)))),
    )
}
