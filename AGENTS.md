# AGENTS.md

## What this repo is

A Rust desktop music client for Navidrome (Subsonic-compatible) servers, designed for HTPC use with a Pepper Jobs W10 remote. Built with egui/eframe for the UI, opensubsonic-rs for the Subsonic API, and mpv as a subprocess for audiophile-grade audio playback.

## Runtime dependencies

- **Rust** 1.85+ (edition 2021)
- **mpv** — audio playback engine (subprocess, controlled via JSON IPC over Unix socket)
  - Arch: `pacman -S mpv`
  - Debian/Ubuntu: `apt install mpv`
- **A Navidrome server** (or any Subsonic-compatible server) for actual data

## Build & run

```bash
# Build
cargo build --release

# Run
cargo run --release

# Debug with verbose logging
RUST_LOG=debug cargo run

# Mock features (for UI testing without a server or mpv)
cargo run --features mock-server
cargo run --features mock-mpv
```

No automated tests, linters, or CI. Verify changes with `cargo check` and manual testing.

## Key conventions

- **egui/eframe:** `default-features = false`, features = `["default_fonts", "wayland", "glow"]` on eframe and `["default_fonts"]` on egui. This disables accesskit, which crashes on Linux without AT-SPI running.
- **Interactive UI elements:** Use `allocate_exact_size(size, Sense::click())` + painter rendering. Do NOT use `egui::Button` for elements in the D-pad focus zones — it conflicts with the custom FocusZone system.
- **Mouse clicks:** Use `clicked_by(PointerButton::Primary)`, NOT `.clicked()`. This prevents keyboard Enter from double-firing through egui's native widget focus.
- **Focus management:** `ctx.memory_mut(|mem| { if let Some(id) = mem.focused() { mem.surrender_focus(id); } })` at the start of each frame, before any panels render. Except: text input widgets (search bar, wizard fields) call `resp.request_focus()` to re-acquire native focus.
- **Borrow patterns:** When iterating over `state` data inside an egui `ScrollArea` or `horizontal` closure and then mutating `state` (e.g., on click), clone the data first: `let albums = state.albums.clone();` then iterate `albums`. This avoids the classic egui double-borrow.
- **`stable_dt`** in egui 0.30 is already `f32`, not `Duration`. Do NOT call `.as_secs_f32()` on it.

## Architecture

Three-thread model:
1. **UI thread (egui/eframe)** — immediate-mode rendering at 60fps, never blocks on I/O. Polls shared state from other threads each frame.
2. **Subsonic client thread (tokio)** — async HTTP via opensubsonic-rs. Receives commands via crossbeam channel, returns results via `Arc<RwLock<FetchResults>>`.
3. **mpv subprocess thread** — spawns `mpv --idle --input-ipc-server=<socket>`. Sends JSON commands, reads events, monitors process health.

Shared state: `Arc<RwLock<T>>` for results from worker threads. The UI thread polls each frame.

## Key behaviours to keep correct

- **ExFAT character sanitization:** N/A (this is a network client, not a file sync tool)
- **Focus zones:** Three zones (Content, Menu, Transport), navigated purely by direction (no Tab). Down from Content → Transport. Left from leftmost Content → Menu. Up from Transport → Content.
- **Space/Play-Pause behavior:** If nothing playing → start queue + push NowPlaying. If playing → toggle play/pause (no view change). If paused → resume + push NowPlaying.
- **Auto-switch:** When a track starts playing (from any view), auto-switch to NowPlaying. Escape returns to previous view.
- **Play queue management:** The app maintains the queue, not mpv. Per-track `loadfile` with `replace`. On `end-file` event, app increments `current_track_index` and loads the next track.
- **Initial Play command:** When the UI sets `is_playing=true` (from Play button click), app.rs detects that mpv isn't actually playing yet (current_time == 0.0, total_duration == 0.0) and sends the initial `MpvCommand::Play` with the stream URL.
- **Config:** TOML at `~/.config/navidrome-htpc/config.toml`. Settings save on change. Wizard sets `wizard.completed = true` on finish.
- **Cover art cache:** Disk cache at `~/.cache/navidrome-htpc/covers/`, in-memory `HashMap<String, TextureHandle>`. `CoverArtCache` struct is implemented but not yet wired into NavidromeApp.

## Subsonic API

Uses the [opensubsonic-rs](https://github.com/M0Rf30/opensubsonic-rs) crate (v0.1). Key notes:
- `Client::new(base_url, username, auth)` — takes 3 args (not 2)
- `Auth` has `token(password)` and `plain(username, password)` — no `api_key` variant (map ApiKey to token)
- `get_album_list2(AlbumListType, ...)` — takes an enum, not a string; returns `Vec<AlbumId3>` directly
- `get_artist(&id)` / `get_album(&id)` — single-arg, no Option second param
- `stream_url(song_id, max_bitrate)` — builds URL without HTTP request (used to pass to mpv)

## mpv IPC

JSON over Unix socket at `/tmp/navidrome-htpc-mpv-<pid>.sock`. Commands: `loadfile`, `set_property` (pause, time-pos, volume), `get_property`, `observe_property` (time-pos, duration, pause). Events: `start-file`, `end-file`, `property-change`.

## File map

| File | Responsibility |
|---|---|
| `src/main.rs` | Entry point, config load, thread spawning |
| `src/app.rs` | NavidromeApp, eframe::App impl, keyboard dispatch, mpv polling, context menu |
| `src/config.rs` | Config struct, TOML load/save, AuthMethod/ReplayGainMode enums |
| `src/state.rs` | AppState, View enum, FocusZone enum, FocusState, ArtistSort, AlbumSort, Toast |
| `src/focus.rs` | Directional focus navigation logic (handle_key, handle_arrow) |
| `src/theme.rs` | Color constants, apply_theme() |
| `src/subsonic/mod.rs` | SubsonicClient, command channel, API mapping, stream_url() helper |
| `src/subsonic/commands.rs` | SubsonicCommand enum, SortType, FetchResults, SearchResults |
| `src/subsonic/models.rs` | Domain types (Artist, Album, Track, Playlist, QueueSource) |
| `src/subsonic/cover_art.rs` | CoverArtCache (disk + memory LRU) |
| `src/mpv/mod.rs` | MpvController, subprocess lifecycle, event loop |
| `src/mpv/ipc.rs` | MpvIpc, JSON protocol over Unix socket |
| `src/mpv/events.rs` | MpvEvent enum, MpvState struct |
| `src/ui/mod.rs` | Module declarations |
| `src/ui/home.rs` | Home view (cards, recently added/played) |
| `src/ui/artist_list.rs` | Sortable artist list |
| `src/ui/artist_detail.rs` | Artist's album grid |
| `src/ui/album_list.rs` | Sortable album grid |
| `src/ui/album_detail.rs` | Album track list + Play/Shuffle/Add to Queue |
| `src/ui/playlist_list.rs` | Playlist list |
| `src/ui/playlist_detail.rs` | Playlist track list |
| `src/ui/search.rs` | Search view (debounced auto-search) |
| `src/ui/now_playing.rs` | Now Playing + play queue + auto-scroll |
| `src/ui/settings.rs` | Settings view (5 categories) |
| `src/ui/wizard.rs` | 4-step connection wizard |
| `src/ui/transport.rs` | Transport bar (painter-based) |
| `src/ui/menu.rs` | Bottom-left ☰ menu flyout |
| `src/ui/common.rs` | Shared widgets (Card, Thumbnail, TrackRow, Toast, ContextMenu) |

## No build, test, lint, or CI setup

No automated tests, linters, or CI workflows. Verify changes manually with `cargo check` and `cargo run`. Debug features (`--features mock-server`, `--features mock-mpv`) allow testing the UI without a server or mpv.
