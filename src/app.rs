use eframe::egui;
use std::time::{Duration, Instant};
use crate::state::{AppState, View, FocusZone};
use crate::focus::{handle_key, handle_arrow, FocusAction};
use crate::subsonic::SubsonicClient;
use crate::mpv::MpvController;
use crate::ui::common::{ContextMenuState, ContextMenuAction};

pub struct NavidromeApp {
    pub state: AppState,
    /// Subsonic client thread handle. `None` when no server is configured
    /// (the wizard hasn't run yet) — in that case the UI operates in an
    /// offline / placeholder mode.
    pub subsonic: Option<SubsonicClient>,
    /// mpv subprocess controller. `None` when no server is configured or
    /// the mpv binary failed to spawn.
    pub mpv: Option<MpvController>,
    pub wizard: crate::ui::wizard::WizardState,
    /// When the last search query was sent to the Subsonic client.
    last_search_time: Option<Instant>,
    /// The last query string that was sent, to avoid re-sending on every
    /// frame when the query hasn't changed.
    last_search_query: String,
    /// UI-only state for the Play/Shuffle/Add-to-Queue context menu flyout.
    pub context_menu: ContextMenuState,
    /// A context-menu action waiting on album-detail tracks to arrive from
    /// the Subsonic client thread. Populated when the user opens the menu on
    /// an album card whose tracks aren't loaded yet; consumed in the
    /// `SubsonicCommand::poll` path once `album_detail` arrives.
    pending_menu_album: Option<(String, ContextMenuAction)>,
    /// Disk + memory cover-art cache. Fetches synchronously (blocking HTTP
    /// on cold cache, disk hit on warm cache). Populated lazily as albums
    /// are displayed.
    cover_art_cache: crate::subsonic::cover_art::CoverArtCache,
}

impl NavidromeApp {
    pub fn new(
        state: AppState,
        subsonic: Option<SubsonicClient>,
        mpv: Option<MpvController>,
    ) -> Self {
        // Initialize the cover-art cache using the configured cache directory.
        let cache_dir = std::path::PathBuf::from(&state.config.cache.dir)
            .join("covers");
        Self {
            state,
            subsonic,
            mpv,
            wizard: Default::default(),
            last_search_time: None,
            last_search_query: String::new(),
            context_menu: ContextMenuState::default(),
            pending_menu_album: None,
            cover_art_cache: crate::subsonic::cover_art::CoverArtCache::new(cache_dir),
        }
    }
}

impl eframe::App for NavidromeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::theme::apply_theme(ctx);
        ctx.set_pixels_per_point(self.state.config.display.scale);

        if !self.state.server_configured {
            crate::ui::wizard::render(ctx, &mut self.state, &mut self.wizard);
            return;
        }

        // ── Post-wizard backend initialization ─────────────────────────────────
        // If the wizard has completed (server_configured was just set to true)
        // but the Subsonic / mpv backends haven't been created yet (they were
        // None at startup because the wizard hadn't run), spin them up now.
        if self.subsonic.is_none() {
            self.subsonic = Some(crate::subsonic::SubsonicClient::start(
                self.state.config.clone(),
            ));
        }
        if self.mpv.is_none() {
            self.mpv =
                crate::mpv::MpvController::start(self.state.config.audio.clone());
        }

        // ── Fetch data on view entry ──────────────────────────────────────────
        match self.state.current_view() {
            crate::state::View::Home if self.state.recent_albums.is_empty() => {
                if let Some(ref subsonic) = self.subsonic {
                    subsonic.send(crate::subsonic::commands::SubsonicCommand::GetRecentlyAdded { limit: 20 });
                    subsonic.send(crate::subsonic::commands::SubsonicCommand::GetRecentlyPlayed { limit: 20 });
                }
            }
            crate::state::View::AlbumList if self.state.albums.is_empty() => {
                if let Some(ref subsonic) = self.subsonic {
                    subsonic.send(crate::subsonic::commands::SubsonicCommand::GetAlbumList {
                        sort: crate::subsonic::commands::SortType::Newest,
                        offset: 0,
                        limit: 100,
                    });
                }
            }
            crate::state::View::ArtistList if self.state.artists.is_empty() => {
                if let Some(ref subsonic) = self.subsonic {
                    subsonic.send(crate::subsonic::commands::SubsonicCommand::GetArtists);
                }
            }
            crate::state::View::PlaylistList if self.state.playlists.is_empty() => {
                if let Some(ref subsonic) = self.subsonic {
                    subsonic.send(crate::subsonic::commands::SubsonicCommand::GetPlaylists);
                }
            }
            crate::state::View::AlbumDetail if self.state.current_album_tracks.is_empty() => {
                if let Some(ref album) = self.state.current_album {
                    if let Some(ref subsonic) = self.subsonic {
                        subsonic.send(crate::subsonic::commands::SubsonicCommand::GetAlbumDetail { id: album.id.clone() });
                    }
                }
            }
            crate::state::View::ArtistDetail if self.state.current_artist_albums.is_empty() => {
                if let Some(ref artist) = self.state.current_artist {
                    if let Some(ref subsonic) = self.subsonic {
                        subsonic.send(crate::subsonic::commands::SubsonicCommand::GetArtistDetail { id: artist.id.clone() });
                    }
                }
            }
            crate::state::View::PlaylistDetail if self.state.current_playlist_tracks.is_empty() => {
                if let Some(ref playlist) = self.state.current_playlist {
                    if let Some(ref subsonic) = self.subsonic {
                        subsonic.send(crate::subsonic::commands::SubsonicCommand::GetPlaylistDetail { id: playlist.id.clone() });
                    }
                }
            }
            _ => {}
        }

        // ── Debounced search ────────────────────────────────────────────────
        if let Some(ref subsonic) = self.subsonic {
            let query = self.state.search_query.clone();
            if query.len() >= 2
                && query != self.last_search_query
                && self
                    .last_search_time
                    .map(|t| t.elapsed() >= Duration::from_millis(300))
                    .unwrap_or(true)
            {
                self.last_search_time = Some(Instant::now());
                self.last_search_query = query.clone();
                subsonic.send(crate::subsonic::commands::SubsonicCommand::Search {
                    query,
                    artist_count: 20,
                    album_count: 20,
                    song_count: 50,
                });
                // Reset search results so a new query clears stale data
                self.state.search_results_artists.clear();
                self.state.search_results_albums.clear();
                self.state.search_results_tracks.clear();
            }
        }

        // ── Keyboard dispatch ─────────────────────────────────────────────────
        if let Some(ref subsonic) = self.subsonic {
            let results = subsonic.poll();
            if let Some(albums) = results.recent_albums {
                self.state.recent_albums = albums;
            }
            if let Some(albums) = results.recent_played {
                self.state.recent_played = albums;
            }
            if let Some(albums) = results.album_list {
                self.state.albums = albums;
            }
            if let Some(artists) = results.artists {
                self.state.artists = artists;
            }
            if let Some(playlists) = results.playlists {
                self.state.playlists = playlists;
            }
            if let Some((album, tracks)) = results.album_detail {
                // If a context-menu action is waiting for this album's tracks,
                // apply it now (the user opened the flyout on an album card
                // whose tracks weren't loaded yet; we sent GetAlbumDetail and
                // stashed the pending action). The action handler will set
                // current_album_tracks itself, so we skip the normal
                // assignment only when the pending action consumes it.
                let album_id = album.id.clone();
                let pending = self
                    .pending_menu_album
                    .as_ref()
                    .map(|(id, _)| id == &album_id)
                    .unwrap_or(false);
                if pending {
                    self.state.current_album = Some(album);
                    self.state.current_album_tracks = tracks.clone();
                    let action = self
                        .pending_menu_album
                        .take()
                        .map(|(_, a)| a)
                        .unwrap();
                    self.apply_action_to_tracks(action, tracks);
                } else {
                    self.state.current_album = Some(album);
                    self.state.current_album_tracks = tracks;
                }
            }
            if let Some((artist, albums)) = results.artist_detail {
                self.state.current_artist = Some(artist);
                self.state.current_artist_albums = albums;
            }
            if let Some((playlist, tracks)) = results.playlist_detail {
                self.state.current_playlist = Some(playlist);
                self.state.current_playlist_tracks = tracks;
            }
            if let Some(sr) = results.search_results {
                self.state.search_results_artists = sr.artists;
                self.state.search_results_albums = sr.albums;
                self.state.search_results_tracks = sr.tracks;
            }
            if let Some(ref err) = results.error {
                self.state.toasts.push(crate::state::Toast {
                    message: err.clone(),
                    ttl: 3.0,
                });
            }
        }

        // ── Cover-art fetch ────────────────────────────────────────────────────
        // Collect all album cover IDs visible in the current view, then fetch
        // any that aren't cached yet. On the first render after fetching, the
        // texture will be available. This runs every frame but `fetch_blocking`
        // short-circuits on cache hits (in-memory check).
        self.fetch_cover_arts_for_current_view(ctx);

        // ── Keyboard dispatch ─────────────────────────────────────────────────
        let keys = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Space),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::ArrowLeft),
                i.key_pressed(egui::Key::ArrowRight),
            )
        });

        if keys.0 {
            // Clone focus to a local so we can pass both `&mut focus` and `&self.state`
            // to handle_key without a double-borrow of `self.state`. (handle_key's
            // app_state param is currently unused — reserved for later tasks — so
            // writing the clone back is a no-op for now.)
            let mut focus = self.state.focus.clone();
            let action = handle_key(&mut focus, egui::Key::Escape, &self.state);
            self.state.focus = focus;
            if action == FocusAction::Escape {
                if self.state.focus.menu_expanded {
                    self.state.focus.menu_expanded = false;
                } else {
                    self.state.pop_view();
                }
            }
        }
        // ── Arrow-key focus navigation ──────────────────────────────────────────
        //
        // Dispatch arrow keys to `handle_arrow` for focus movement. We skip
        // dispatch when the context-menu flyout is open (it handles its own
        // keyboard input). ArrowRight in the Content zone opens the context
        // menu (below); for other zones it navigates normally.
        let arrow_key = if keys.3 {
            Some(egui::Key::ArrowUp)
        } else if keys.4 {
            Some(egui::Key::ArrowDown)
        } else if keys.5 {
            Some(egui::Key::ArrowLeft)
        } else if keys.6 {
            Some(egui::Key::ArrowRight)
        } else {
            None
        };

        if let Some(key) = arrow_key {
            // Compute the number of selectable rows in the current view so
            // ArrowDown on the last row jumps to the transport controls.
            let num_content_rows = match self.state.current_view() {
                View::Home => 3,            // section cards, Recently Added, Recently Played
                View::AlbumDetail => self.state.current_album_tracks.len().max(1),
                View::PlaylistDetail => self.state.current_playlist_tracks.len().max(1),
                View::NowPlaying => self.state.play_queue.len().max(1),
                _ => 1,
            };
            let mut focus = self.state.focus.clone();
            handle_arrow(&mut focus, key, num_content_rows, 6);
            self.state.focus = focus;
        }

        // ── Context menu: open on Space when focused on a card/track ──────────
        // (only when the flyout isn't already open — it handles its own keys)
        if keys.2 && self.state.focus.zone == FocusZone::Content && !self.context_menu.open {
            self.maybe_open_context_menu_for_focus();
        }

        // ── Enter key: activate the focused item ──────────────────────────────
        //
        // Pressing Enter on a focused content item triggers the same action as
        // clicking it (navigate to detail view, play from track, etc.).
        if keys.1 && !self.context_menu.open && self.state.focus.zone == FocusZone::Content {
            let row = self.state.focus.content_row;
            let col = self.state.focus.content_col;
            match self.state.current_view() {
                View::Home => match row {
                    // Section cards: Artists (0), Albums (1), Playlists (2)
                    0 => match col {
                        0 => self.state.push_view(View::ArtistList),
                        1 => self.state.push_view(View::AlbumList),
                        2 => self.state.push_view(View::PlaylistList),
                        _ => {}
                    },
                    // Recently Added (row 1) or Recently Played (row 2)
                    1 | 2 => {
                        let albums = if row == 1 {
                            &self.state.recent_albums
                        } else {
                            &self.state.recent_played
                        };
                        if let Some(album) = albums.get(col) {
                            self.state.current_album = Some(album.clone());
                            self.state.push_view(View::AlbumDetail);
                        }
                    }
                    _ => {}
                },
                View::AlbumList => {
                    if let Some(album) = self.state.albums.get(col) {
                        self.state.current_album = Some(album.clone());
                        self.state.push_view(View::AlbumDetail);
                    }
                }
                View::AlbumDetail => {
                    if row < self.state.current_album_tracks.len() {
                        self.state.play_queue = self.state.current_album_tracks.clone();
                        self.state.current_track_index = Some(row);
                        self.state.is_playing = true;
                        self.state.push_view(View::NowPlaying);
                    }
                }
                View::ArtistList => {
                    if let Some(artist) = self.state.artists.get(col) {
                        self.state.current_artist = Some(artist.clone());
                        self.state.push_view(View::ArtistDetail);
                    }
                }
                View::PlaylistList => {
                    if let Some(playlist) = self.state.playlists.get(col) {
                        self.state.current_playlist = Some(playlist.clone());
                        self.state.push_view(View::PlaylistDetail);
                    }
                }
                View::NowPlaying => {
                    if row < self.state.play_queue.len() {
                        self.state.play_queue = self.state.play_queue.clone();
                        self.state.current_track_index = Some(row);
                        self.state.is_playing = true;
                    }
                }
                _ => {}
            }
        }

        // ── Render transport + menu before CentralPanel ────────────────────────
        crate::ui::menu::render(ctx, &mut self.state);
        crate::ui::transport::render(ctx, &mut self.state);

        // ── CentralPanel ───────────────────────────────────────────────────────
        ctx.memory_mut(|mem| {
            if let Some(id) = mem.focused() {
                mem.surrender_focus(id);
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.state.current_view() {
                View::Home => crate::ui::home::render(ui, &mut self.state),
                View::ArtistList => crate::ui::artist_list::render(ui, &mut self.state),
                View::ArtistDetail => crate::ui::artist_detail::render(ui, &mut self.state),
                View::AlbumList => crate::ui::album_list::render(ui, &mut self.state),
                View::AlbumDetail => crate::ui::album_detail::render(ui, &mut self.state),
                View::PlaylistList => crate::ui::playlist_list::render(ui, &mut self.state),
                View::PlaylistDetail => crate::ui::playlist_detail::render(ui, &mut self.state),
                View::Search => crate::ui::search::render(ui, &mut self.state),
                View::NowPlaying => crate::ui::now_playing::render(ui, &mut self.state),
                View::Settings => crate::ui::settings::render(ui, &mut self.state),
            }

            // Render toast notifications on top of everything
            crate::ui::common::render_toasts(ui, &mut self.state);
        });

        // ── Context menu flyout ───────────────────────────────────────────────
        // Rendered as an `egui::Area` after CentralPanel so it floats above the
        // view content. `render_context_menu` handles its own Up/Down/Enter/
        // Escape/Left keyboard input; we only resolve the returned action
        // here. Position: anchored near the top-left of the screen plus a
        // small margin, so it's visible on every view regardless of where the
        // pointer is. (A future polish could anchor it next to the focused
        // widget, but egui doesn't expose focus widget rects cheaply.)
        if self.context_menu.open {
            let pos = ctx
                .input(|i| i.pointer.latest_pos())
                .unwrap_or_else(|| egui::pos2(120.0, 120.0));
            // Clamp so the 200x120 menu stays on-screen.
            let screen = ctx.screen_rect();
            let clamped = egui::pos2(
                pos.x.min(screen.max.x - 210.0).max(screen.min.x + 8.0),
                pos.y.min(screen.max.y - 130.0).max(screen.min.y + 8.0),
            );
            if let Some(action) =
                crate::ui::common::render_context_menu(ctx, &mut self.context_menu, clamped)
            {
                self.handle_context_menu_action(action);
            }
        }

        // ── Poll mpv state + advance queue on track end ───────────────────────
        // After rendering: we read the mpv subprocess's current state, sync the
        // UI's playback fields, and detect the "track ended" transition so we
        // can advance the play queue. We also auto-switch to the NowPlaying
        // view the first time `is_playing` flips to true after a click on
        // Play/Shuffle (album_detail / playlist_detail already push the
        // NowPlaying view themselves; this is a safety net for any other
        // trigger path).
        if let Some(ref mpv) = self.mpv {
            let mpv_state = mpv.poll();

            // Snapshot the "was playing" flag *before* we overwrite it, so we
            // can detect the playing → stopped transition that mpv emits as
            // `end-file`.
            let was_playing = self.state.is_playing;

            self.state.is_playing = mpv_state.is_playing && !mpv_state.is_paused;
            self.state.current_time = mpv_state.current_time;
            self.state.total_duration = mpv_state.total_duration;

            // If mpv crashed, surface a toast so the user knows playback died.
            if mpv_state.crashed && was_playing {
                self.state.toasts.push(crate::state::Toast {
                    message: "mpv subprocess crashed — playback stopped".to_string(),
                    ttl: 4.0,
                });
            }

            // Initial play detection: the UI set is_playing=true (e.g. from
            // album_detail Play button) but mpv hasn't started yet (no
            // current_time, not playing). Send the first track's URL to mpv.
            //
            // CRITICAL: we check `was_playing` (the pre-poll value, true when
            // the click handler just set it) instead of `self.state.is_playing`
            // (which was just overwritten by mpv's idle state in the poll
            // above). Without this guard the initial play fires but then the
            // track-end check below also fires on the same frame because
            // `was_playing` is true and mpv hasn't started yet — advancing
            // past track 0 and skipping the first track entirely.
            if was_playing
                && !mpv_state.is_playing
                && self.state.current_time == 0.0
                && self.state.total_duration == 0.0
                && self.state.current_track_index.is_some()
            {
                if let Some(idx) = self.state.current_track_index {
                    if idx < self.state.play_queue.len() {
                        if let Some(ref subsonic) = self.subsonic {
                            let track = self.state.play_queue[idx].clone();
                            let max_bitrate = self.state.config.audio.max_bitrate;
                            match subsonic.stream_url(&track.id, max_bitrate) {
                                Some(url) => {
                                    mpv.send(crate::mpv::MpvCommand::Play { url });
                                }
                                None => {
                                    self.state.toasts.push(crate::state::Toast {
                                        message: "Could not build stream URL".to_string(),
                                        ttl: 3.0,
                                    });
                                    self.state.is_playing = false;
                                }
                            }
                        }
                    }
                }
            }

            // Track-end transition: mpv reports is_playing=false but we were
            // playing on the previous frame. Advance the queue (when auto-
            // advance is enabled) and tell mpv to load the next URL.
            //
            // GUARD: we require `total_duration > 0.0` to avoid a false
            // positive on the very first frame after a Play click. On that
            // frame `was_playing` is true (set by the click handler) and
            // `mpv_state.is_playing` is false (mpv hasn't started loading
            // yet), but mpv hasn't actually played anything — `total_duration`
            // is still 0.0 (mpv only sets duration when it loads a file).
            // Without this guard the track-end fires immediately after a
            // Play click, advancing past track 0 and skipping it entirely.
            if !mpv_state.is_playing
                && was_playing
                && !mpv_state.crashed
                && self.state.total_duration > 0.0
                && self.state.config.playback.auto_advance
            {
                match self.state.current_track_index {
                    Some(idx) => {
                        let next = idx + 1;
                        if next < self.state.play_queue.len() {
                            self.state.current_track_index = Some(next);
                            self.state.current_time = 0.0;
                            self.state.total_duration = 0.0;
                            // Send the next track's stream URL to mpv. We
                            // build the URL synchronously via the Subsonic
                            // client handle (no I/O — just query-string
                            // construction). If the URL can't be built we
                            // leave mpv idle and surface a toast.
                            if let Some(ref subsonic) = self.subsonic {
                                let track = self.state.play_queue[next].clone();
                                let max_bitrate = self.state.config.audio.max_bitrate;
                                match subsonic.stream_url(&track.id, max_bitrate) {
                                    Some(url) => {
                                        mpv.send(crate::mpv::MpvCommand::Play { url });
                                        self.state.is_playing = true;
                                    }
                                    None => {
                                        self.state.toasts.push(crate::state::Toast {
                                            message:
                                                "Could not build stream URL for next track"
                                                    .to_string(),
                                            ttl: 3.0,
                                        });
                                        self.state.is_playing = false;
                                    }
                                }
                            }
                        } else {
                            // Queue exhausted
                            self.state.is_playing = false;
                            self.state.current_track_index = None;
                            self.state.current_time = 0.0;
                            self.state.total_duration = 0.0;
                        }
                    }
                    None => {
                        self.state.is_playing = false;
                    }
                }
            }

            // Auto-switch to NowPlaying when playback starts and we're not
            // already showing it. Album/playlist detail views push NowPlaying
            // themselves on Play/Shuffle clicks; this catches the mpv-driven
            // path (e.g. queue advancement while the user is browsing another
            // view) so the UI follows the music.
            if self.state.is_playing
                && !was_playing
                && self.state.current_view() != View::NowPlaying
                && self.state.current_track_index.is_some()
            {
                self.state.push_view(View::NowPlaying);
            }

            // ── Pending transport actions ──────────────────────────────────────
            //
            // The transport bar click handler sets `pending_transport_action`
            // when the user clicks play/pause/stop/next/previous. We consume it
            // here, inside the mpv block where we have access to the mpv
            // command channel. This is the only place mpv commands can be sent.
            if let Some(action) = self.state.pending_transport_action.take() {
                match action {
                    crate::state::TransportAction::Play => {
                        mpv.send(crate::mpv::MpvCommand::Resume);
                    }
                    crate::state::TransportAction::Pause => {
                        mpv.send(crate::mpv::MpvCommand::Pause);
                    }
                    crate::state::TransportAction::Stop => {
                        mpv.send(crate::mpv::MpvCommand::Stop);
                    }
                    crate::state::TransportAction::Next
                    | crate::state::TransportAction::Previous => {
                        // Stop the current track; the initial-play detection
                        // on the next frame will pick up the new track index
                        // (already set by the transport click handler) and
                        // send its URL to mpv.
                        mpv.send(crate::mpv::MpvCommand::Stop);
                    }
                }
            }
        }  // end if let Some(ref mpv)
    }
}

// ── Context-menu helpers ─────────────────────────────────────────────────────
//
// These methods live on `NavidromeApp` rather than inline in `update()` so the
// keyboard-dispatch and action-resolution logic stays readable. They assume
// `self.state` and `self.context_menu` are the only state they touch (plus
// `self.subsonic` / `self.pending_menu_album` for the async album-fetch path).

impl NavidromeApp {
    /// Open the context-menu flyout for the currently focused item, if it's a
    /// card or track row in one of the views that supports the flyout. No-op
    /// for views with explicit header buttons (which rely on those buttons
    /// instead) or when the focused index is out of range.
    fn maybe_open_context_menu_for_focus(&mut self) {
        let view = self.state.current_view();
        let row = self.state.focus.content_row;
        let col = self.state.focus.content_col;

        match view {
            View::AlbumList => {
                if let Some(album) = self.state.albums.get(row) {
                    self.context_menu.open_for_album(album.id.clone());
                }
            }
            View::Home => match row {
                // Row 0 = navigation cards (Artists/Albums/Playlists): no menu.
                1 => {
                    if let Some(album) = self.state.recent_albums.get(col) {
                        self.context_menu.open_for_album(album.id.clone());
                    }
                }
                2 => {
                    if let Some(album) = self.state.recent_played.get(col) {
                        self.context_menu.open_for_album(album.id.clone());
                    }
                }
                _ => {}
            },
            View::AlbumDetail => {
                if row < self.state.current_album_tracks.len() {
                    self.context_menu.open_for_track(row);
                }
            }
            View::PlaylistDetail => {
                if row < self.state.current_playlist_tracks.len() {
                    self.context_menu.open_for_track(row);
                }
            }
            // Other views (ArtistList, ArtistDetail, PlaylistList, Search,
            // NowPlaying, Settings) don't surface the context-menu flyout in
            // this task; their cards are navigation-only or have dedicated
            // controls.
            _ => {}
        }
    }

    /// Resolve a `ContextMenuAction` returned by the flyout against the play
    /// queue. For album-card triggers the tracks may not be loaded yet, in
    /// which case we kick off a `GetAlbumDetail` fetch and defer the action
    /// until the results arrive (see `pending_menu_album` handling in the
    /// poll path). For track-row triggers the tracks are already in
    /// `current_album_tracks` / `current_playlist_tracks`.
    fn handle_context_menu_action(&mut self, action: ContextMenuAction) {
        // Track-row trigger: act on the row's containing track list.
        if let Some(idx) = self.context_menu.track_index {
            let view = self.state.current_view();
            match view {
                View::AlbumDetail => {
                    let tracks = self.state.current_album_tracks.clone();
                    self.apply_action_to_track_in_list(action, &tracks, idx);
                }
                View::PlaylistDetail => {
                    let tracks = self.state.current_playlist_tracks.clone();
                    self.apply_action_to_track_in_list(action, &tracks, idx);
                }
                _ => {}
            }
            return;
        }

        // Album-card trigger: we need the album's tracks. If we already have
        // them (current_album matches), apply immediately; otherwise fetch.
        if let Some(album_id) = self.context_menu.album_id.clone() {
            let have_tracks = self
                .state
                .current_album
                .as_ref()
                .map(|a| a.id == album_id && !self.state.current_album_tracks.is_empty())
                .unwrap_or(false);
            if have_tracks {
                let tracks = self.state.current_album_tracks.clone();
                self.apply_action_to_tracks(action, tracks);
            } else {
                // Defer: fetch album detail, stash the pending action.
                if let Some(ref subsonic) = self.subsonic {
                    subsonic.send(crate::subsonic::commands::SubsonicCommand::GetAlbumDetail {
                        id: album_id.clone(),
                    });
                    self.pending_menu_album = Some((album_id, action));
                    self.state.toasts.push(crate::state::Toast {
                        message: "Loading album…".to_string(),
                        ttl: 1.5,
                    });
                } else {
                    self.state.toasts.push(crate::state::Toast {
                        message: "No server — can't load album".to_string(),
                        ttl: 3.0,
                    });
                }
            }
        }
    }

    /// Apply a context-menu action to a full track list (album-card trigger).
    /// `PlayNow` replaces the queue starting from track 0; `Shuffle` replaces
    /// the queue with a shuffled copy; `AddToQueue` appends to the current
    /// queue and surfaces a toast.
    fn apply_action_to_tracks(
        &mut self,
        action: ContextMenuAction,
        tracks: Vec<crate::subsonic::models::Track>,
    ) {
        if tracks.is_empty() {
            self.state.toasts.push(crate::state::Toast {
                message: "No tracks to play".to_string(),
                ttl: 2.0,
            });
            return;
        }
        match action {
            ContextMenuAction::PlayNow => {
                self.state.play_queue = tracks;
                self.state.current_track_index = Some(0);
                self.state.is_playing = true;
                self.state.current_time = 0.0;
                self.state.total_duration = 0.0;
                self.state.push_view(View::NowPlaying);
            }
            ContextMenuAction::Shuffle => {
                let mut tracks = tracks;
                use rand::seq::SliceRandom;
                let mut rng = rand::rng();
                tracks.shuffle(&mut rng);
                self.state.play_queue = tracks;
                self.state.current_track_index = Some(0);
                self.state.is_playing = true;
                self.state.current_time = 0.0;
                self.state.total_duration = 0.0;
                self.state.push_view(View::NowPlaying);
            }
            ContextMenuAction::AddToQueue => {
                let n = tracks.len();
                self.state.play_queue.extend(tracks);
                self.state.toasts.push(crate::state::Toast {
                    message: format!("Added {n} tracks to queue"),
                    ttl: 3.0,
                });
            }
        }
    }

    /// Apply a context-menu action to a single track row within a track list
    /// (track-row trigger). `PlayNow` replaces the queue with tracks from the
    /// selected index onward (matching the album_detail click behaviour);
    /// `Shuffle` replaces the queue with a shuffled copy of the full list;
    /// `AddToQueue` appends just the one track.
    fn apply_action_to_track_in_list(
        &mut self,
        action: ContextMenuAction,
        tracks: &[crate::subsonic::models::Track],
        idx: usize,
    ) {
        if tracks.is_empty() || idx >= tracks.len() {
            return;
        }
        match action {
            ContextMenuAction::PlayNow => {
                self.state.play_queue = tracks[idx..].to_vec();
                self.state.current_track_index = Some(0);
                self.state.is_playing = true;
                self.state.current_time = 0.0;
                self.state.total_duration = 0.0;
                self.state.push_view(View::NowPlaying);
            }
            ContextMenuAction::Shuffle => {
                let mut tracks = tracks.to_vec();
                use rand::seq::SliceRandom;
                let mut rng = rand::rng();
                tracks.shuffle(&mut rng);
                self.state.play_queue = tracks;
                self.state.current_track_index = Some(0);
                self.state.is_playing = true;
                self.state.current_time = 0.0;
                self.state.total_duration = 0.0;
                self.state.push_view(View::NowPlaying);
            }
            ContextMenuAction::AddToQueue => {
                let track = tracks[idx].clone();
                self.state.play_queue.push(track.clone());
                self.state.toasts.push(crate::state::Toast {
                    message: format!("Added \"{}\" to queue", track.title),
                    ttl: 3.0,
                });
            }
        }
    }

    /// Global Space / Play-Pause handler (per spec §5).
    ///
    /// - If nothing is currently playing → start the queue from track 0:
    ///   set `is_playing=true`, `current_track_index=Some(0)`, and push the
    ///   NowPlaying view so the user lands on the playback screen. The mpv
    ///   poll loop in `update()` will detect the `is_playing && !mpv_playing`
    ///   transition and send the stream URL to mpv.
    /// - If something is playing → toggle mpv's pause state with no view
    ///   change. We always route through `MpvCommand::TogglePause` so mpv
    ///   remains the source of truth for the paused flag; the next
    ///   `mpv.poll()` will refresh `state.is_playing` accordingly.
    ///
    /// If the queue is empty and nothing is playing, we surface a toast
    /// instead of silently no-op'ing — the user pressed Play and nothing
    /// happened, which is confusing without feedback.
    fn handle_play_pause_global(&mut self) {
        // Already playing: toggle pause via mpv, no view change.
        if self.state.is_playing {
            if let Some(ref mpv) = self.mpv {
                mpv.send(crate::mpv::MpvCommand::TogglePause);
            }
            return;
        }

        // Nothing playing. Need a non-empty queue to start.
        if self.state.play_queue.is_empty() {
            self.state.toasts.push(crate::state::Toast {
                message: "Queue is empty — nothing to play".to_string(),
                ttl: 2.5,
            });
            return;
        }

        // If we have a current track index but are paused (is_playing=false
        // but current_track_index=Some), resume instead of restarting.
        if let Some(idx) = self.state.current_track_index {
            if idx < self.state.play_queue.len() {
                self.state.is_playing = true;
                if let Some(ref mpv) = self.mpv {
                    mpv.send(crate::mpv::MpvCommand::Resume);
                }
                if self.state.current_view() != View::NowPlaying {
                    self.state.push_view(View::NowPlaying);
                }
                return;
            }
        }

        // Fresh start: kick off track 0. The mpv poll loop will see
        // `is_playing && !mpv_playing && current_time == 0` and send the
        // stream URL on the next frame.
        self.state.current_track_index = Some(0);
        self.state.is_playing = true;
        self.state.current_time = 0.0;
        self.state.total_duration = 0.0;
        self.state.push_view(View::NowPlaying);
    }
}

// ── Cover-art fetch helpers ─────────────────────────────────────────────────
//
// These live on a separate `impl` block so they don't clutter the main update
// method. The primary driver is `fetch_cover_arts_for_current_view` which
// collects album cover IDs visible in the current view and triggers blocking
// fetches for any not yet cached.

impl NavidromeApp {
    /// Collect all album cover IDs from the current view's visible albums and
    /// fetch any that aren't in the cache yet. Also syncs cached textures into
    /// `state.cover_textures` so the render functions can find them via
    /// `state.cover_textures.get(&album_id)` (or `&track.album_id`).
    fn fetch_cover_arts_for_current_view(&mut self, ctx: &egui::Context) {
        let pairs = self.collect_visible_cover_ids();

        let subsonic = match self.subsonic {
            Some(ref s) => s,
            None => return,
        };
        let client = match subsonic.client {
            Some(ref c) => c,
            None => return,
        };
        let size = self.state.config.cache.cover_art_size;

        for (album_id, cover_id_opt) in &pairs {
            let cover_id = match cover_id_opt {
                Some(cid) => cid,
                None => continue,
            };
            // Fetch by cover_art_id (the URL key), key the cache by
            // cover_art_id for deduplication (multiple albums may share
            // the same cover art).
            if self.cover_art_cache.get(cover_id).is_none() {
                match crate::subsonic::build_cover_art_url(client, cover_id, size) {
                    Ok(url) => {
                        if let Err(e) = self
                            .cover_art_cache
                            .fetch_blocking(ctx, cover_id, &url, size)
                        {
                            tracing::warn!("Cover art fetch failed for {cover_id}: {e}");
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Could not build cover art URL for {cover_id}: {e}");
                    }
                }
            }
        }

        // Sync cached textures into state.cover_textures keyed by album_id
        // (matching what home.rs, album_detail.rs, and now_playing.rs use
        // for their lookups). Multiple albums sharing the same cover_id
        // produce multiple cover_textures entries, each pointing to the
        // same TextureHandle (cheap Arc clone).
        for (album_id, cover_id_opt) in &pairs {
            if let Some(cover_id) = cover_id_opt {
                if let Some(tex) = self.cover_art_cache.get(cover_id) {
                    self.state
                        .cover_textures
                        .entry(album_id.clone())
                        .or_insert_with(|| tex.clone());
                }
            }
        }
    }

    /// Collect `(album_id, cover_art_id)` pairs for albums visible in the
    /// current view. The `album_id` is what views use as a lookup key in
    /// `cover_textures`; `cover_art_id` is passed to the Subsonic URL
    /// builder. Returns `None` when the album/track has no cover art.
    fn collect_visible_cover_ids(&self) -> Vec<(String, Option<String>)> {
        let mut pairs = Vec::new();
        match self.state.current_view() {
            View::Home => {
                for album in &self.state.recent_albums {
                    pairs.push((album.id.clone(), album.cover_art_id.clone()));
                }
                for album in &self.state.recent_played {
                    pairs.push((album.id.clone(), album.cover_art_id.clone()));
                }
            }
            View::AlbumList => {
                for album in &self.state.albums {
                    pairs.push((album.id.clone(), album.cover_art_id.clone()));
                }
            }
            View::AlbumDetail => {
                if let Some(ref album) = self.state.current_album {
                    pairs.push((album.id.clone(), album.cover_art_id.clone()));
                }
            }
            View::ArtistDetail => {
                for album in &self.state.current_artist_albums {
                    pairs.push((album.id.clone(), album.cover_art_id.clone()));
                }
            }
            View::NowPlaying => {
                if let Some(idx) = self.state.current_track_index {
                    if let Some(track) = self.state.play_queue.get(idx) {
                        pairs.push((track.album_id.clone(), track.cover_art_id.clone()));
                    }
                }
            }
            // ArtistList, PlaylistList, PlaylistDetail, Search, Settings
            // don't display cover art.
            _ => {}
        }
        pairs
    }
}
