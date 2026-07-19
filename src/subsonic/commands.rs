//! Commands sent from the UI thread to the Subsonic client thread, plus the
//! shared `FetchResults` state the client writes back.
//!
//! The UI sends `SubsonicCommand` values over a crossbeam channel and reads
//! results by cloning the `Arc<RwLock<FetchResults>>` shared with the client
//! thread. Each command overwrites the corresponding field of `FetchResults`
//! and clears `error` on success (or sets it on failure).

use uuid::Uuid;

use crate::subsonic::models::{Album, Artist, Playlist, Track};

/// A command for the Subsonic client thread.
///
/// Variants mirror the Subsonic REST endpoints the UI needs. The client
/// thread matches on this and dispatches to the appropriate `opensubsonic`
/// method, writing the result into the shared `FetchResults`.
#[derive(Debug)]
pub enum SubsonicCommand {
    /// `getAlbumList2?type=newest` — recently *added* albums.
    GetRecentlyAdded { limit: u32 },
    /// `getAlbumList2?type=recent` — recently *played* albums.
    GetRecentlyPlayed { limit: u32 },
    /// `getArtists` — full ID3 artist index.
    GetArtists,
    /// `getArtist` — artist detail + their albums.
    GetArtistDetail { id: String },
    /// `getAlbumList2` with an arbitrary sort.
    GetAlbumList {
        sort: SortType,
        offset: u32,
        limit: u32,
    },
    /// `getAlbum` — album detail + its songs.
    GetAlbumDetail { id: String },
    /// `getPlaylists` — all playlists visible to the user.
    GetPlaylists,
    /// `getPlaylist` — playlist detail + its entries.
    GetPlaylistDetail { id: String },
    /// `search3` — ID3-based search across artists, albums, and songs.
    Search {
        query: String,
        artist_count: u32,
        album_count: u32,
        song_count: u32,
    },
    /// `scrobble` — report a play (submission=true) or now-playing (false).
    Scrobble { id: String, submission: bool },
}

/// How to sort `getAlbumList2` results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortType {
    Newest,
    Recent,
    AlphabeticalByName,
    AlphabeticalByArtist,
    ByYear,
    Random,
}

/// Snapshot of everything the client thread has fetched so far.
///
/// Each field is `Option<Vec<…>>` (or tuple for detail views) so the UI can
/// tell "fetched and empty" (`Some(vec![])`) from "not fetched yet" (`None`).
/// `error` holds the most recent error message, if any.
#[derive(Debug, Clone, Default)]
pub struct FetchResults {
    pub recent_albums: Option<Vec<Album>>,
    pub recent_played: Option<Vec<Album>>,
    pub artists: Option<Vec<Artist>>,
    pub artist_detail: Option<(Artist, Vec<Album>)>,
    pub album_list: Option<Vec<Album>>,
    pub album_detail: Option<(Album, Vec<Track>)>,
    pub playlists: Option<Vec<Playlist>>,
    pub playlist_detail: Option<(Playlist, Vec<Track>)>,
    pub search_results: Option<SearchResults>,
    pub error: Option<String>,
}

/// Results from a `search3` call, split by category.
#[derive(Debug, Clone, Default)]
pub struct SearchResults {
    pub artists: Vec<Artist>,
    pub albums: Vec<Album>,
    pub tracks: Vec<Track>,
}

/// Generate a random request-id string (currently unused but reserved for
/// future logging / tracing correlation).
pub fn new_request_id() -> String {
    Uuid::new_v4().to_string()
}
