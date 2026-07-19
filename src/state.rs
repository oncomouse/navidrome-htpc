use std::collections::HashMap;
use crate::config::Config;
use crate::subsonic::models::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum View {
    Home,
    ArtistList,
    ArtistDetail,
    AlbumList,
    AlbumDetail,
    PlaylistList,
    PlaylistDetail,
    Search,
    NowPlaying,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusZone {
    Content,
    Menu,
    Transport,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ArtistSort {
    #[default]
    Alphabetical,
    ByAlbumCount,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AlbumSort {
    #[default]
    Newest,
    AlphabeticalByName,
    AlphabeticalByArtist,
    Random,
}

#[derive(Debug, Clone)]
pub struct FocusState {
    pub zone: FocusZone,
    pub content_row: usize,       // which row in home / which item in list
    pub content_col: usize,       // which item within row
    pub menu_expanded: bool,
    pub menu_index: usize,        // 0=Search, 1=Settings, 2=NowPlaying
    pub transport_index: usize,   // 0=Prev, 1=Play, 2=Stop, 3=Next, 4=Progress, 5=Volume
}

impl Default for FocusState {
    fn default() -> Self {
        Self {
            zone: FocusZone::Content,
            content_row: 0,
            content_col: 0,
            menu_expanded: false,
            menu_index: 0,
            transport_index: 1, // start on Play/Pause
        }
    }
}

pub struct AppState {
    pub config: Config,
    pub view_stack: Vec<View>,
    pub focus: FocusState,
    pub server_configured: bool,

    // Sort state
    pub artist_sort: ArtistSort,
    pub album_sort: AlbumSort,

    // Data (populated by Subsonic client thread)
    pub recent_albums: Vec<Album>,
    pub recent_played: Vec<Album>,
    pub artists: Vec<Artist>,
    pub albums: Vec<Album>,
    pub playlists: Vec<Playlist>,
    pub current_album: Option<Album>,
    pub current_album_tracks: Vec<Track>,
    pub current_artist: Option<Artist>,
    pub current_artist_albums: Vec<Album>,
    pub current_playlist: Option<Playlist>,
    pub current_playlist_tracks: Vec<Track>,
    pub search_query: String,
    pub search_results_artists: Vec<Artist>,
    pub search_results_albums: Vec<Album>,
    pub search_results_tracks: Vec<Track>,

    // Playback state (updated by mpv thread)
    pub play_queue: Vec<Track>,
    pub current_track_index: Option<usize>,
    pub is_playing: bool,
    pub current_time: f32,
    pub total_duration: f32,
    pub volume: f32,

    // UI state
    pub toasts: Vec<Toast>,
    pub cover_textures: HashMap<String, egui::TextureHandle>,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub ttl: f32,  // seconds remaining
}

impl AppState {
    pub fn current_view(&self) -> View {
        *self.view_stack.last().unwrap_or(&View::Home)
    }

    pub fn push_view(&mut self, view: View) {
        self.view_stack.push(view);
    }

    pub fn pop_view(&mut self) {
        if self.view_stack.len() > 1 {
            self.view_stack.pop();
        }
    }
}
