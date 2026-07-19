use eframe::egui;
use crate::state::{AppState, View, FocusZone};
use crate::focus::{handle_key, handle_arrow, FocusAction};
use crate::subsonic::SubsonicClient;
use crate::mpv::MpvController;

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
}

impl NavidromeApp {
    pub fn new(
        state: AppState,
        subsonic: Option<SubsonicClient>,
        mpv: Option<MpvController>,
    ) -> Self {
        Self { state, subsonic, mpv, wizard: Default::default() }
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

        // ── Poll SubsonicClient results ────────────────────────────────────────
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
                self.state.current_album = Some(album);
                self.state.current_album_tracks = tracks;
            }
            if let Some((artist, albums)) = results.artist_detail {
                self.state.current_artist = Some(artist);
                self.state.current_artist_albums = albums;
            }
            if let Some((playlist, tracks)) = results.playlist_detail {
                self.state.current_playlist = Some(playlist);
                self.state.current_playlist_tracks = tracks;
            }
            if let Some(ref err) = results.error {
                self.state.toasts.push(crate::state::Toast { message: err.clone(), ttl: 3.0 });
            }
        }

        // ── Keyboard dispatch ─────────────────────────────────────────────────
        let keys = ctx.input(|i| (
            i.key_pressed(egui::Key::Escape),
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::Space),
            i.key_pressed(egui::Key::ArrowUp),
            i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::ArrowLeft),
            i.key_pressed(egui::Key::ArrowRight),
        ));

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
        // (Full keyboard dispatch expanded in later tasks)

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
            if self.state.is_playing
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
            if !mpv_state.is_playing
                && was_playing
                && !mpv_state.crashed
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
        }
    }
}
