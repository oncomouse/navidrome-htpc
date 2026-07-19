//! Subsonic client thread.
//!
//! [`SubsonicClient::start`] spawns a dedicated OS thread running a tokio
//! runtime. The UI thread sends [`SubsonicCommand`]s over an unbounded
//! crossbeam channel; the client thread executes them against the
//! `opensubsonic` crate and writes the results into a shared
//! `Arc<RwLock<FetchResults>>` that the UI polls each frame.
//!
//! The split keeps all async network I/O off the UI thread (egui is
//! single-threaded and synchronous) while giving the UI cheap non-blocking
//! reads of the latest fetched data.

pub mod commands;
pub mod cover_art;
pub mod models;

use std::sync::{Arc, RwLock};

use crossbeam::channel::{self, Receiver, Sender};
use tokio::runtime::Runtime;

use opensubsonic::{Auth, Client};
use opensubsonic::data::{
    AlbumId3, AlbumWithSongsId3, ArtistId3, ArtistWithAlbumsId3, Child, Playlist as OsPlaylist,
    PlaylistWithSongs,
};

use crate::config::{AuthMethod, Config};
use crate::subsonic::commands::{FetchResults, SearchResults, SortType, SubsonicCommand};

/// Handle to the running Subsonic client thread.
///
/// Clone-safe to keep in the egui app struct — `Sender` is `Send + Sync` and
/// the `Arc<RwLock<…>>` is the shared result store.
pub struct SubsonicClient {
    pub command_tx: Sender<SubsonicCommand>,
    pub results: Arc<RwLock<FetchResults>>,
}

impl SubsonicClient {
    /// Spawn the client thread and return a handle.
    ///
    /// If the server URL is empty or the `opensubsonic::Client` cannot be
    /// constructed (bad URL, missing credentials), the thread sets
    /// `FetchResults::error` and then exits — `send` will still appear to
    /// work (channel buffer accepts) but no commands will be processed.
    pub fn start(config: Config) -> Self {
        let (command_tx, command_rx) = channel::unbounded::<SubsonicCommand>();
        let results = Arc::new(RwLock::new(FetchResults::default()));
        let results_clone = results.clone();

        std::thread::spawn(move || {
            let rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("Failed to create tokio runtime: {e}");
                    if let Ok(mut r) = results_clone.write() {
                        r.error = Some(format!("Runtime failed: {e}"));
                    }
                    return;
                }
            };
            rt.block_on(async move {
                run_client(config, command_rx, results_clone).await;
            });
        });

        Self {
            command_tx,
            results,
        }
    }

    /// Send a command to the client thread. Non-blocking; drops silently if
    /// the receiver has been dropped (thread exited).
    pub fn send(&self, cmd: SubsonicCommand) {
        let _ = self.command_tx.send(cmd);
    }

    /// Clone the current fetch results. Cheap (single RwLock read + clone).
    pub fn poll(&self) -> FetchResults {
        match self.results.read() {
            Ok(r) => r.clone(),
            Err(_) => FetchResults::default(),
        }
    }
}

/// Client-thread entry point: build the `opensubsonic::Client` then loop on
/// the command channel until the sender side is dropped.
async fn run_client(
    config: Config,
    command_rx: Receiver<SubsonicCommand>,
    results: Arc<RwLock<FetchResults>>,
) {
    // Map our AuthMethod → opensubsonic::Auth. The opensubsonic crate only
    // exposes token & plain; ApiKey isn't a standard Subsonic auth method so
    // we fall back to token auth using the configured api_key as the secret
    // (servers that don't understand it will simply reject it, which surfaces
    // as a normal API error in FetchResults::error).
    let auth = match config.server.auth_method {
        AuthMethod::Token => Auth::token(&config.server.password),
        AuthMethod::Plain => Auth::plain(&config.server.password),
        AuthMethod::ApiKey => Auth::token(&config.server.api_key),
    };

    let client = match Client::new(&config.server.url, &config.server.username, auth) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to create Subsonic client: {e}");
            if let Ok(mut r) = results.write() {
                r.error = Some(format!("Connection failed: {e}"));
            }
            return;
        }
    };

    while let Ok(cmd) = command_rx.recv() {
        if let Err(e) = handle_command(&client, cmd, &results).await {
            tracing::error!("Subsonic command failed: {e}");
            if let Ok(mut r) = results.write() {
                r.error = Some(format!("{e}"));
            }
        }
    }
}

/// Dispatch one command, writing the result into `results` on success.
async fn handle_command(
    client: &Client,
    cmd: SubsonicCommand,
    results: &Arc<RwLock<FetchResults>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match cmd {
        SubsonicCommand::GetRecentlyAdded { limit } => {
            let albums = client
                .get_album_list2(
                    opensubsonic::AlbumListType::Newest,
                    Some(limit as i32),
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await?;
            let mapped: Vec<crate::subsonic::models::Album> =
                albums.into_iter().map(map_album).collect();
            if let Ok(mut r) = results.write() {
                r.recent_albums = Some(mapped);
                r.error = None;
            }
        }
        SubsonicCommand::GetRecentlyPlayed { limit } => {
            let albums = client
                .get_album_list2(
                    opensubsonic::AlbumListType::Recent,
                    Some(limit as i32),
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await?;
            let mapped: Vec<crate::subsonic::models::Album> =
                albums.into_iter().map(map_album).collect();
            if let Ok(mut r) = results.write() {
                r.recent_played = Some(mapped);
                r.error = None;
            }
        }
        SubsonicCommand::GetArtists => {
            let resp = client.get_artists(None).await?;
            let mapped: Vec<crate::subsonic::models::Artist> = resp
                .index
                .into_iter()
                .flat_map(|i| i.artist.into_iter().map(map_artist))
                .collect();
            if let Ok(mut r) = results.write() {
                r.artists = Some(mapped);
                r.error = None;
            }
        }
        SubsonicCommand::GetArtistDetail { id } => {
            let resp = client.get_artist(&id).await?;
            let artist = map_artist_id3_full(&resp);
            let album_vec: Vec<crate::subsonic::models::Album> =
                resp.album.into_iter().map(map_album).collect();
            if let Ok(mut r) = results.write() {
                r.artist_detail = Some((artist, album_vec));
                r.error = None;
            }
        }
        SubsonicCommand::GetAlbumDetail { id } => {
            let resp = client.get_album(&id).await?;
            let album = map_album_with_songs(&resp);
            let tracks: Vec<crate::subsonic::models::Track> =
                resp.song.into_iter().map(map_track).collect();
            if let Ok(mut r) = results.write() {
                r.album_detail = Some((album, tracks));
                r.error = None;
            }
        }
        SubsonicCommand::GetPlaylists => {
            let resp = client.get_playlists(None).await?;
            let mapped: Vec<crate::subsonic::models::Playlist> =
                resp.into_iter().map(map_playlist).collect();
            if let Ok(mut r) = results.write() {
                r.playlists = Some(mapped);
                r.error = None;
            }
        }
        SubsonicCommand::GetPlaylistDetail { id } => {
            let resp = client.get_playlist(&id).await?;
            let playlist = map_playlist_with_songs(&resp);
            let tracks: Vec<crate::subsonic::models::Track> =
                resp.entry.into_iter().map(map_track).collect();
            if let Ok(mut r) = results.write() {
                r.playlist_detail = Some((playlist, tracks));
                r.error = None;
            }
        }
        SubsonicCommand::Search {
            query,
            artist_count,
            album_count,
            song_count,
        } => {
            let resp = client
                .search3(
                    &query,
                    Some(artist_count as i32),
                    None,
                    Some(album_count as i32),
                    None,
                    Some(song_count as i32),
                    None,
                    None,
                )
                .await?;
            let sr = SearchResults {
                artists: resp.artist.into_iter().map(map_artist).collect(),
                albums: resp.album.into_iter().map(map_album).collect(),
                tracks: resp.song.into_iter().map(map_track).collect(),
            };
            if let Ok(mut r) = results.write() {
                r.search_results = Some(sr);
                r.error = None;
            }
        }
        SubsonicCommand::GetAlbumList { sort, offset, limit } => {
            let list_type = match sort {
                SortType::Newest => opensubsonic::AlbumListType::Newest,
                SortType::Recent => opensubsonic::AlbumListType::Recent,
                SortType::AlphabeticalByName => opensubsonic::AlbumListType::AlphabeticalByName,
                SortType::AlphabeticalByArtist => {
                    opensubsonic::AlbumListType::AlphabeticalByArtist
                }
                SortType::ByYear => opensubsonic::AlbumListType::ByYear,
                SortType::Random => opensubsonic::AlbumListType::Random,
            };
            let resp = client
                .get_album_list2(
                    list_type,
                    Some(limit as i32),
                    Some(offset as i32),
                    None,
                    None,
                    None,
                    None,
                )
                .await?;
            let mapped: Vec<crate::subsonic::models::Album> =
                resp.into_iter().map(map_album).collect();
            if let Ok(mut r) = results.write() {
                r.album_list = Some(mapped);
                r.error = None;
            }
        }
        SubsonicCommand::Scrobble { id, submission } => {
            client.scrobble(&id, None, Some(submission)).await?;
            if let Ok(mut r) = results.write() {
                r.error = None;
            }
        }
    }
    Ok(())
}

// ── Mapping helpers: opensubsonic data types → our domain types ────────────

/// Map an `ArtistId3` (the index-only variant, no albums) to our `Artist`.
fn map_artist(a: ArtistId3) -> crate::subsonic::models::Artist {
    crate::subsonic::models::Artist {
        id: a.id,
        name: a.name,
        album_count: a.album_count.unwrap_or(0) as u32,
        cover_art_id: a.cover_art,
    }
}

/// Map an `ArtistWithAlbumsId3` (the detail variant) to our `Artist`.
///
/// We pull `album_count` from the real field; if absent, fall back to the
/// number of albums in the embedded `album` vec — but that's consumed by the
/// caller, so we use the field value here (the caller's `resp.album` is
/// separately mapped).
fn map_artist_id3_full(a: &ArtistWithAlbumsId3) -> crate::subsonic::models::Artist {
    crate::subsonic::models::Artist {
        id: a.id.clone(),
        name: a.name.clone(),
        album_count: a.album_count.unwrap_or(0) as u32,
        cover_art_id: a.cover_art.clone(),
    }
}

/// Map an `AlbumId3` (list variant, no songs) to our `Album`.
fn map_album(a: AlbumId3) -> crate::subsonic::models::Album {
    crate::subsonic::models::Album {
        id: a.id,
        name: a.name,
        artist_id: a.artist_id.unwrap_or_default(),
        artist_name: a.artist.unwrap_or_default(),
        year: a.year.map(|y| y as u16),
        genre: a.genre,
        cover_art_id: a.cover_art,
        song_count: a.song_count.unwrap_or(0) as u32,
        duration_secs: a.duration.unwrap_or(0) as u32,
        created: a.created.unwrap_or_default(),
    }
}

/// Map an `AlbumWithSongsId3` (detail variant) to our `Album`. The songs are
/// handled separately by the caller via `map_track`.
fn map_album_with_songs(a: &AlbumWithSongsId3) -> crate::subsonic::models::Album {
    crate::subsonic::models::Album {
        id: a.id.clone(),
        name: a.name.clone(),
        artist_id: a.artist_id.clone().unwrap_or_default(),
        artist_name: a.artist.clone().unwrap_or_default(),
        year: a.year.map(|y| y as u16),
        genre: a.genre.clone(),
        cover_art_id: a.cover_art.clone(),
        song_count: a.song_count.unwrap_or(0) as u32,
        duration_secs: a.duration.unwrap_or(0) as u32,
        created: a.created.clone().unwrap_or_default(),
    }
}

/// Map a `Child` (the universal Subsonic media item) to our `Track`.
fn map_track(s: Child) -> crate::subsonic::models::Track {
    crate::subsonic::models::Track {
        id: s.id,
        title: s.title,
        artist_id: s.artist_id.unwrap_or_default(),
        artist_name: s.artist.unwrap_or_default(),
        album_id: s.album_id.unwrap_or_default(),
        album_name: s.album.unwrap_or_default(),
        track_number: s.track.map(|t| t as u32),
        disc_number: s.disc_number.map(|d| d as u32),
        duration_secs: s.duration.unwrap_or(0) as u32,
        cover_art_id: s.cover_art,
        bitrate: s.bit_rate.map(|b| b as u32),
        suffix: s.suffix,
        size_bytes: s.size.map(|sz| sz as u64),
    }
}

/// Map a `Playlist` (list variant, no entries) to our `Playlist`.
fn map_playlist(p: OsPlaylist) -> crate::subsonic::models::Playlist {
    crate::subsonic::models::Playlist {
        id: p.id,
        name: p.name,
        song_count: p.song_count.unwrap_or(0) as u32,
        duration_secs: p.duration.unwrap_or(0) as u32,
        public: p.public,
        owner: p.owner,
        created: p.created,
    }
}

/// Map a `PlaylistWithSongs` (detail variant) to our `Playlist`. Entries are
/// handled separately by the caller via `map_track`.
fn map_playlist_with_songs(p: &PlaylistWithSongs) -> crate::subsonic::models::Playlist {
    crate::subsonic::models::Playlist {
        id: p.id.clone(),
        name: p.name.clone(),
        song_count: p.song_count.unwrap_or(0) as u32,
        duration_secs: p.duration.unwrap_or(0) as u32,
        public: p.public,
        owner: p.owner.clone(),
        created: p.created.clone(),
    }
}

// ── URL builders (no HTTP; safe to call from any thread) ───────────────────

/// Build a streaming URL for a song. `max_bitrate=0` means unlimited.
pub fn build_stream_url(
    client: &Client,
    song_id: &str,
    max_bitrate: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = client.stream_url(
        song_id,
        if max_bitrate > 0 {
            Some(max_bitrate as i32)
        } else {
            None
        },
        None,
    )?;
    Ok(url.to_string())
}

/// Build a cover-art URL for an album/artist. `size=0` means server default.
pub fn build_cover_art_url(
    client: &Client,
    cover_id: &str,
    size: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = client.cover_art_url(cover_id, if size > 0 { Some(size as i32) } else { None })?;
    Ok(url.to_string())
}
