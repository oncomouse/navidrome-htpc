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

    // Spawn the Subsonic client thread when a server is configured. The
    // client owns its own OS thread + tokio runtime; the UI talks to it
    // through the crossbeam channel returned in the handle.
    let subsonic = if server_configured {
        Some(crate::subsonic::SubsonicClient::start(config.clone()))
    } else {
        None
    };

    // Spawn the mpv audio subprocess when a server is configured. Returns
    // None if the `mpv` binary is missing or the IPC socket can't be opened;
    // in that case playback is simply unavailable (UI still works).
    let mpv = if server_configured {
        crate::mpv::MpvController::start(config.audio.clone())
    } else {
        None
    };

    let state = AppState {
        config: config.clone(),
        view_stack: vec![View::Home],
        focus: Default::default(),
        server_configured,
        artist_sort: Default::default(),
        album_sort: Default::default(),
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
        last_played_track_index: None,
        last_scrolled_track: None,
        is_playing: false,
        current_time: 0.0,
        total_duration: 0.0,
        volume: 0.75,
        pending_transport_action: None,
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
        Box::new(move |_cc| Ok(Box::new(crate::app::NavidromeApp::new(state, subsonic, mpv)))),
    )
}
