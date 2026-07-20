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

/// A user action from the transport bar that must be forwarded to the mpv
/// thread. The transport click handler sets this as a pending action; the
/// mpv poll block in `app.rs` reads it, sends the corresponding mpv
/// command, and clears it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransportAction {
    /// Resume playback (un-pause)
    Play,
    /// Pause playback
    Pause,
    /// Stop playback and unload the current file
    Stop,
    /// Advance to the next track in the queue
    Next,
    /// Go back to the previous track
    Previous,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusZone {
    Content,
    Menu,
    Transport,
    /// The action buttons at the top of a detail view (album/playlist):
    /// Play (0), Shuffle (1), Add to Queue (2). These are painted manually
    /// and have no egui-native focus, so they get their own zone.
    Header,
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
    pub header_index: usize,      // 0=Play, 1=Shuffle, 2=Add to Queue (detail view header)
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
            header_index: 0,    // start on Play
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
    /// Index of the last track that was actually playing, retained across a
    /// Stop so the Play button can resume/restart it. `None` only before the
    /// first track has ever played.
    pub last_played_track_index: Option<usize>,
    /// Index of the track we last auto-scrolled the NowPlaying queue to,
    /// so we only re-scroll when the current track actually changes
    /// (not every frame — that would fight manual queue scrolling).
    pub last_scrolled_track: Option<usize>,
    pub is_playing: bool,
    /// Play/pause state the user last requested via the transport bar. While
    /// `Some`, the transport button renders from this intended value instead
    /// of the raw per-frame mpv poll, eliminating the 1-2 frame icon flicker
    /// while mpv's IPC catches up to the command. Cleared once mpv's reported
    /// state matches the intent, once a stopped/empty state settles against a
    /// play-intent, or after `intent_frames_remaining` hits zero.
    pub intended_playing: Option<bool>,
    /// Frames left before the `intended_playing` latch is force-cleared, so a
    /// command mpv never confirms (e.g. a dead stream) can't wedge the icon.
    pub intent_frames_remaining: u16,
    pub current_time: f32,
    pub total_duration: f32,
    pub volume: f32,
    /// Pending action from the transport bar that the mpv poll block should
    /// execute on the next frame. Set by `transport::render` when the user
    /// clicks a transport button; consumed by the mpv block in `app.rs`.
    pub pending_transport_action: Option<TransportAction>,

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
