# Navidrome HTPC Client — Design Spec

**Date:** 2026-07-19
**Status:** Draft — pending user review
**Author:** Andrew Pilsch (oncomouse)

## 1. Overview

A desktop music client for [Navidrome](https://www.navidrome.org/) servers, designed for HTPC (Home Theater PC) use with a Pepper Jobs W10 gyro remote. The client is controllable entirely via D-pad (arrow keys), Enter, Space, and media keys, with optional mouse support. It targets a 10-foot UI paradigm with large, navigable elements.

**Key differentiators from existing clients:**
- Remote-first navigation: no Tab cycling, pure directional (D-pad) focus management
- Full-screen Now Playing view with play queue and auto-scroll to current track
- Drill-in navigation model (Home → section → detail) with Escape to go back
- Audiophile-grade audio via mpv subprocess (bit-perfect, exclusive mode, gapless)

**Reference UIs:** Jellyfin Web and Jellyfin Media Player — similar visual language and remote behavior, but with a superior music/now-playing experience.

## 2. Technology Stack

| Component | Technology | Rationale |
|---|---|---|
| Language | Rust | Performance, safety, developer's existing expertise |
| GUI | egui / eframe 0.30+ | Immediate-mode, excellent keyboard input handling, HTPC focus patterns already documented, cross-platform (X11/Wayland/Windows/macOS) |
| Subsonic API | `opensubsonic-rs` crate | Async, complete (~80 endpoints), Subsonic v1.16.1 + OpenSubsonic extensions, explicitly Navidrome-compatible |
| Audio | mpv subprocess via JSON IPC | Bit-perfect output, gapless, replaygain, all formats, low integration effort. Controlled via Unix socket / named pipe. |
| Async runtime | tokio | Required by opensubsonic-rs and for mpv IPC |
| Config | TOML (`~/.config/navidrome-htpc/config.toml`) | Human-readable, simple, standard in Rust ecosystem |
| HTTP | reqwest (rustls) | For cover art fetching (opensubsonic-rs uses reqwest internally) |
| Image loading | `image` crate + `egui_extras` | Decode cover art JPEG/PNG to egui textures |

### 2.1 Why not alternatives

- **Tauri (web frontend):** Web focus management fights the remote's D-pad. Two tech stacks. No clear win for a simple HTPC UI.
- **C++/Qt:** Mature but steep learning curve for the developer. Slower iteration vs. Rust/egui.
- **cpal + symphonia (pure Rust audio):** Writing a media player core from scratch (gapless, replaygain, format support, streaming pipeline) is weeks of work before reaching the UI.
- **GStreamer:** Heavy dependency, complex pipeline API, harder to debug than mpv IPC.

## 3. Architecture

### 3.1 Three-thread model

```
┌─────────────────────────────────────────────────┐
│                  Main Binary                     │
│                                                  │
│  ┌──────────┐  channels  ┌──────────────────┐   │
│  │  egui    │◄──────────►│  Subsonic Client  │   │
│  │  UI      │            │  (opensubsonic-rs)│   │
│  │ (eframe) │            │  async / tokio    │   │
│  │          │            └──────────────────┘   │
│  │          │  channels  ┌──────────────────┐   │
│  │          │◄──────────►│  mpv subprocess   │   │
│  │          │            │  (JSON IPC socket)│   │
│  │          │            └──────────────────┘   │
│  └──────────┘                                    │
│       │                                          │
│       ▼                                          │
│  Config (TOML, persisted to disk)                │
└─────────────────────────────────────────────────┘
```

1. **UI thread (egui/eframe)** — immediate-mode rendering at 60fps. Never blocks on I/O. Polls shared state from other threads each frame.
2. **Subsonic client thread (tokio)** — async HTTP. Receives commands via crossbeam channel, returns results via `Arc<RwLock<>>` shared state with request IDs to prevent stale overwrites.
3. **mpv subprocess thread** — spawns `mpv --idle --input-ipc-server=<socket>`. Sends JSON commands, reads events, monitors process health.

### 3.2 Data flow: "Play an album"

1. User selects album, presses Enter on a track in AlbumDetail
2. UI thread builds play queue: `Vec<stream_url>` via `client.stream_url(track_id)`
3. UI thread sends `PlayQueue { urls, start_index }` to mpv thread
4. mpv thread sends `{"command": ["loadfile", url, "replace"]}` to mpv via socket
5. mpv fires `start-file` event → mpv thread updates `current_track_index` in shared state
6. UI thread polls `current_track_index` each frame, updates NowPlaying view, auto-scrolls to current track

### 3.3 Config

TOML at `~/.config/navidrome-htpc/config.toml` (platform-equivalent path via `dirs` crate).

```toml
[server]
url = "https://music.local:4533"
username = "admin"
auth_method = "token"  # token | api_key | plain
# password / api_key stored here (local config, not network-transmitted in plain)

[audio]
device = "auto"         # "auto" or specific alsa/hw device string
exclusive = true        # --audio-exclusive
gapless = true          # --gapless-audio
replaygain = "album"    # off | track | album
max_bitrate = 0         # 0 = unlimited

[display]
scale = 1.5             # pixels_per_point
theme = "dark"

[playback]
scrobble = true
auto_advance = true
resume_on_start = true  # restore last queue + position

[cache]
dir = "~/.cache/navidrome-htpc"
cover_art_size = 300    # px

[wizard]
completed = false       # set true after wizard finishes
```

## 4. Views and Navigation

### 4.1 View stack (drill-in model)

Each view pushes onto a stack. Escape pops it. Home is the root.

```
Home
├── ArtistList → ArtistDetail (album grid) → AlbumDetail (track list)
├── AlbumList → AlbumDetail (track list)
├── PlaylistList → PlaylistDetail (track list)
├── Search (results)
├── Settings
└── NowPlaying
```

### 4.2 Home view

```
┌────────────────────────────────────────────────────┐
│                                                    │
│   [ Artists ]    [ Albums ]    [ Playlists ]       │  Row 1: section cards
│                                                    │
│   Recently Added                              →    │  Row 2: horizontal scroll
│   [■] [■] [■] [■] [■] [■] [■] [■]              →    │
│                                                    │
│   Recently Played                             →    │  Row 3: horizontal scroll
│   [■] [■] [■] [■] [■] [■] [■] [■]              →    │
│                                                    │
│ [☰]                    [⏮] [▶] [⏹] [⏭]    🔊──    │  Bottom: menu + transport
└────────────────────────────────────────────────────┘
```

- **Row 1:** Three large cards (Artists, Albums, Playlists). Enter drills into the respective list view.
- **Row 2:** "Recently Added" — horizontal-scrolling album thumbnails from `getAlbumList2(type=new, size=20)`. Enter on a thumbnail → AlbumDetail.
- **Row 3:** "Recently Played" — horizontal-scrolling album thumbnails from `getAlbumList2(type=recent, size=20)`. Enter on a thumbnail → AlbumDetail.
- **Bottom-left ☰:** collapsed menu icon. Enter/Right expands → [Search, Settings, Now Playing] flyout. See §4.9.
- **Bottom-center/right:** Transport bar (always visible when playing). See §6.

### 4.3 ArtistList

- Top bar: sort dropdown (Name A-Z, Name Z-A) + optional genre filter chips
- Main: virtualized scrolling list or grid of artists
- Enter → ArtistDetail
- Escape → Home
- Data: `getArtists()` — returns artists indexed by letter, lazy-load per section on scroll

### 4.4 ArtistDetail

- Header: artist name
- Main: album grid (their albums)
- Enter on an album → AlbumDetail
- Escape → ArtistList
- Data: `getArtist(id)`

### 4.5 AlbumList

- Top bar: sort dropdown (Name, Artist, Year, Date Added — A-Z and Z-A for each) + filter chips (genre, artist)
- Main: virtualized album grid
- Enter → AlbumDetail
- Escape → Home
- Data: `getAlbumList2(type=<sort>, size=100, offset=<page>)` with pagination

### 4.6 AlbumDetail

- Header: large album art, album title, artist name, [Play] [Shuffle] [Add to Queue] buttons
- Main: track list (track number, title, duration)
- Enter on a track → starts playing from that track (replaces queue with album tracks from that point)
- Escape → back
- Data: `getAlbum(id)` returns album + child songs
- Play = replace queue and start. Shuffle = replace queue (shuffled) and start. Add to Queue = append to current queue, show toast "Added N tracks to queue".

### 4.7 PlaylistList / PlaylistDetail

- PlaylistList: list of playlists (name, song count, duration). Enter → PlaylistDetail. Escape → Home.
- PlaylistDetail: same layout as AlbumDetail (header with [Play] [Shuffle] [Add to Queue], track list). Escape → PlaylistList.
- Data: `getPlaylists()` / `getPlaylist(id)`

### 4.8 Search

- Full-screen view. Search bar at top, auto-focused on entry.
- As-you-type search (debounced 300ms) via `search3(query, artistCount=20, albumCount=20, songCount=50)`
- Results: sections for Artists, Albums, Songs (or unified list — TBD during implementation)
- Enter on artist → ArtistDetail. Enter on album → AlbumDetail. Enter on song → play it (replaces queue with just that song, or adds to queue — see §5.3 Context Menu).
- Escape → Home (clears search query)
- Focus: keyboard input goes to the search bar. Down moves focus to results. Up from results returns to search bar.

### 4.9 Bottom-left ☰ Menu

- **Collapsed state:** ☰ icon in bottom-left corner. Focusable via Left arrow from leftmost content item.
- **Expanded state:** small vertical flyout with [Search, Settings, Now Playing]. Up/Down navigates. Enter selects. Escape or Left collapses.
- Selecting Search → push Search view. Settings → push Settings view. Now Playing → push NowPlaying view (only if something is playing or a queue exists).

### 4.10 NowPlaying

- Full-screen view. Toggle: auto-switch on play start, or via ☰ menu → Now Playing.
- Top section: large album art, track title, artist name, album name, progress bar (seekable via Left/Right when transport is focused, or via slider)
- Below: play queue — scrollable list of all tracks in the queue
- Current track: highlighted with accent color + "now playing" indicator (▶ icon)
- Focused track: focus ring (distinct from current-track highlight)
- Auto-scroll: when track advances, the queue list scrolls to center the new current track. Uses `scroll_to_rect` with `Align::Center`.
- Enter on a track in the queue → jump to that track (plays it immediately)
- Escape → previous view (wherever you were before NowPlaying)
- Transport bar at bottom (same as all views)

### 4.11 Settings

Full-screen scrolling list with category headers:

- **Connection:** Server URL, Username, Auth Method (dropdown), [Reconnect] button
- **Audio:** Device (dropdown from `mpv --audio-device=help`), Exclusive Mode (toggle), Gapless (toggle), ReplayGain (dropdown), Max Bitrate (dropdown)
- **Display:** UI Scale (dropdown: 1.0x, 1.25x, 1.5x, 2.0x), Theme (dropdown)
- **Playback:** Scrobble (toggle), Auto-advance (toggle), Resume on Start (toggle)
- **Cache:** Location (text), Cover Art Size (dropdown), [Clear Cache] button

Navigation: Up/Down scroll, Left/Right adjust toggles/sliders/dropdowns, Enter activates buttons. Escape → Home.

Changes apply immediately for display/playback/cache. Connection changes require [Reconnect] button (re-initializes Subsonic client). Audio device/exclusive changes restart mpv.

### 4.12 Connection Wizard (first launch)

4-step wizard, full-screen overlay. Shown when `config.wizard_completed == false` or no config file exists.

**Step 1 — Server URL:**
- Single text field, auto-focused
- URL validation on Next (must parse as URL with scheme + host)
- Enter proceeds

**Step 2 — Credentials:**
- Username field, password field, auth method dropdown (Token / API Key / Plain)
- Tab/Down to move between fields (Tab works within the wizard form)
- Enter on "Next" proceeds

**Step 3 — Test Connection:**
- Shows "Connecting..." spinner
- Calls `client.ping()` + `client.getLicense()`
- Success: green checkmark, "Connected to \<server\>", auto-advance after 1s or Enter
- Failure: red X, error message, [Back] to fix credentials, [Retry] button

**Step 4 — Audio Output:**
- Audio device dropdown (populated from `mpv --audio-device=help`)
- Exclusive mode toggle
- [Test] button — plays a short silent file to verify device
- [Finish] → saves config, sets `wizard_completed = true`, launches mpv, shows Home

Keyboard: arrow keys to navigate fields, Enter to proceed/activate, Escape to quit app (before wizard completes).

## 5. Focus Management and Keyboard Input

### 5.1 Focus zones

Three zones, navigated purely by direction (no Tab cycling):

```
┌────────────────────────────────────────────┐
│            CONTENT ZONE                     │
│        (rows, grids, lists, etc.)           │
├────┬───────────────────────────────────────┤
│ ☰  │         TRANSPORT ZONE                 │
│MENU │  ⏮  ▶  ⏹  ⏭    ────◯    🔊           │
└────┴───────────────────────────────────────┘
```

### 5.2 Zone transitions

- **Down from bottom of Content** → Transport zone (first control: Prev)
- **Up from Transport** → Content zone (last focused position)
- **Left from leftmost Content item** → Menu zone (☰ icon)
- **Right from Menu** → Content zone (leftmost item of current row)
- **Down from Menu** → Transport zone (first control: Prev)
- **Up from Transport** → Content zone (not Menu — Menu reached only via Left from content)

### 5.3 Within-zone navigation

**Content zone:**
- Up/Down: move between rows (Home: Row1 → Row 2 → Row 3. Lists: scroll. Grids: move by column count.)
- Left/Right: move within row (cards, thumbnails, list items)
- Enter: activate (drill in, play, select)
- Escape: pop view stack

**Menu zone:**
- Collapsed: ☰ is the only item. Enter/Right expands → [Search, Settings, Now Playing]
- Expanded: Up/Down navigates flyout. Enter selects. Escape/Left collapses.

**Transport zone:**
- Left/Right: cycle through [Prev, Play/Pause, Stop, Next, Volume slider]
- Enter: activate focused control
- Volume: Left/Right on volume slider adjusts. Media keys (Vol Up/Down/Mute) work globally.

### 5.4 Global keys (work regardless of focus zone)

| Key | Behavior |
|---|---|
| Space / Play-Pause | If nothing playing → start queue + jump to NowPlaying. If playing → toggle play/pause (no view change). |
| Stop | Stop playback, clear current track. Stay in current view. |
| Next | Skip to next track in queue. No view change. |
| Previous | Skip to previous track in queue. No view change. |
| Vol Up/Down | Adjust volume. No view change. |
| Mute | Toggle mute. No view change. |
| Escape | If Menu expanded → collapse. Else → pop view stack. |

### 5.5 Context menu (Play / Add to Queue)

On any playable item (album card, track row, playlist card, artist card), a context menu flyout is available:

- **Trigger:** Right arrow on a card/row in a grid or list (not in detail views which have explicit buttons)
- **Flyout contents:**
  ```
  ┌─────────────────┐
  │ ▶ Play Now      │
  │ ▶▶ Shuffle Play │
  │ + Add to Queue  │
  └─────────────────┘
  ```
- Up/Down navigates, Enter selects, Escape/Left closes
- **Play Now:** replaces queue with item's tracks, starts playing, auto-switch to NowPlaying
- **Shuffle Play:** replaces queue (shuffled), starts playing, auto-switch to NowPlaying
- **Add to Queue:** appends to current queue, shows toast "Added N tracks to queue"

In detail views (AlbumDetail, PlaylistDetail, ArtistDetail), explicit [Play] [Shuffle] [Add to Queue] buttons at the top serve the same functions — no context menu needed there.

### 5.6 NowPlaying focus

- Content zone: play queue list. Up/Down scroll. Enter on a track → jump to it.
- Current track highlighted (accent color + ▶ icon). Focused track gets focus ring.
- Transport zone: transport controls at bottom (same as all views).

### 5.7 Mouse coexistence

- Clicking any item activates it (same as Enter) and moves keyboard focus to that item/zone.
- Hover shows visual highlight (distinct from keyboard focus ring).
- Mouse scroll scrolls content without moving keyboard focus.

### 5.8 egui implementation

Per the egui HTPC patterns:
- `allocate_exact_size(_, Sense::click())` + painter rendering for all interactive items (avoids native focus conflicts)
- `clicked_by(PointerButton::Primary)` for mouse-only clicks (keyboard handled by custom dispatch)
- Custom keyboard dispatch in `eframe::App::update()` before any panels render
- `ctx.input()` for key state polling
- `scroll_to_rect` for auto-scrolling to focused/current-playing items
- `ctx.memory_mut(|mem| mem.surrender_focus(id))` each frame to clear native focus (except for text input widgets like search bar)
- `pixels_per_point` set from config for UI scale

## 6. Transport Bar

Always visible at the bottom of the screen when music is playing (or a queue exists). Positioned bottom-center/right, to the right of the ☰ menu.

```
[⏮] [▶/⏸] [⏹] [⏭]    [───────●───────]    [🔊 ────●──]
```

Controls (left to right):
0. Previous (⏮)
1. Play/Pause (▶/⏸) — icon changes based on state
2. Stop (⏹) — stops playback, clears current track
3. Next (⏭)
4. Progress/seek slider — shows current position / total duration. Left/Right seeks. (egui::Slider, keyboard-navigable)
5. Volume slider — Left/Right adjusts. (egui::Slider)

Painter-based rendering for buttons (0-3) per egui skill. Sliders (4-5) use egui::Slider widgets (natural keyboard support, no focus conflict).

## 7. Data Model

### 7.1 Domain types

```rust
struct Artist {
    id: String,
    name: String,
    album_count: u32,
    cover_art_id: Option<String>,
}

struct Album {
    id: String,
    name: String,
    artist_id: String,
    artist_name: String,
    year: Option<u16>,
    genre: Option<String>,
    cover_art_id: Option<String>,
    song_count: u32,
    duration_secs: u32,
    created: DateTime,  // date added — for sorting
}

struct Track {
    id: String,
    title: String,
    artist_id: String,
    artist_name: String,
    album_id: String,
    album_name: String,
    track_number: Option<u32>,
    disc_number: Option<u32>,
    duration_secs: u32,
    cover_art_id: Option<String>,
    bitrate: Option<u32>,
    suffix: Option<String>,  // format: flac, mp3, etc.
    size_bytes: Option<u64>,
}

struct Playlist {
    id: String,
    name: String,
    song_count: u32,
    duration_secs: u32,
    public: Option<bool>,
    owner: Option<String>,
    created: Option<DateTime>,
}

enum QueueSource {
    Album(String),      // album id
    Artist(String),     // artist id
    Playlist(String),   // playlist id
    SearchResult,
    Manual,
}

struct PlayQueue {
    tracks: Vec<Track>,
    current_index: usize,
    source: QueueSource,
    is_shuffled: bool,
}
```

### 7.2 Subsonic API mapping

| Feature | Endpoint | Notes |
|---|---|---|
| Recently Added | `getAlbumList2(type=new, size=20)` | Home Row 2 |
| Recently Played | `getAlbumList2(type=recent, size=20)` | Home Row 3 |
| All artists | `getArtists()` | Indexed by letter, lazy-load on scroll |
| Artist detail | `getArtist(id)` | Artist + album list |
| All albums | `getAlbumList2(type=<sort>, size=100, offset=N)` | Paginated |
| Album detail | `getAlbum(id)` | Album + child songs |
| All playlists | `getPlaylists()` | |
| Playlist detail | `getPlaylist(id)` | Playlist + entries |
| Search | `search3(query, artistCount, albumCount, songCount)` | Debounced 300ms |
| Cover art | `cover_art_url(id, size)` → HTTP GET | Cached to disk |
| Stream | `stream_url(id, maxBitRate)` | URL passed to mpv |
| Scrobble | `scrobble(id, submission=true/false)` | On track start (false) and completion (true) |
| Play queue sync | `savePlayQueue` / `getPlayQueue` | Optional server-side queue persistence |

### 7.3 Sorting

Subsonic `getAlbumList2` types mapped to sort options:

| Sort | Subsonic type |
|---|---|
| Name A-Z | `alphabeticalByName` |
| Name Z-A | `alphabeticalByName` (reverse client-side) |
| Artist A-Z | `alphabeticalByArtist` |
| Artist Z-A | `alphabeticalByArtist` (reverse client-side) |
| Year | `byYear` (with fromYear/toYear range) or client-side sort |
| Date Added (newest) | `newest` |
| Date Added (oldest) | `byDateAdded` |
| Random | `random` |

### 7.4 Cover art caching

- Disk cache: `~/.cache/navidrome-htpc/covers/<id>_<size>.jpg`
- In-memory LRU: `HashMap<cover_art_id, egui::TextureHandle>` with capacity ~200
- Concurrent fetch limit: 8 simultaneous requests
- Placeholder: gray square with music note icon while loading / on failure

### 7.5 Async fetching pattern

UI thread sends commands to Subsonic client thread via crossbeam channel:

```rust
enum SubsonicCommand {
    GetRecentlyAdded { limit: u32 },
    GetRecentlyPlayed { limit: u32 },
    GetArtists,
    GetArtistDetail { id: String },
    GetAlbumList { sort: SortType, offset: u32, limit: u32, filter: Option<Filter> },
    GetAlbumDetail { id: String },
    GetPlaylists,
    GetPlaylistDetail { id: String },
    Search { query: String, artist_count: u32, album_count: u32, song_count: u32 },
    GetCoverArt { id: String, size: u32 },
    Scrobble { id: String, submission: bool },
}
```

Each command carries a request ID (UUID). Results are written to `Arc<RwLock<HashMap<RequestId, FetchResult>>>`. UI thread polls each frame, matches results to pending requests, and consumes them. Stale results (request ID no longer in pending set) are discarded.

## 8. mpv Integration

### 8.1 Launch

```
mpv --idle \
    --input-ipc-server=/tmp/navidrome-htpc-mpv.sock \
    --gapless-audio=<yes|no from config> \
    --audio-exclusive=<yes|no from config> \
    --audio-device=<from config> \
    --volume=<from config> \
    --replaygain=<off|track|album from config> \
    --no-video \
    --terminal=no
```

### 8.2 IPC protocol

JSON over Unix socket (Linux/macOS) or named pipe (Windows).

**Commands (app → mpv):**
```json
{"command": ["loadfile", "<stream_url>", "replace"]}
{"command": ["loadfile", "<stream_url>", "append"]}
{"command": ["set_property", "pause", true]}
{"command": ["set_property", "time-pos", 120.5]}
{"command": ["set_property", "volume", 75]}
{"command": ["get_property", "time-pos"]}
{"command": ["get_property", "duration"]}
{"command": ["quit"]}
```

**Events (mpv → app), observed via property-change observations:**
```json
{"event": "start-file"}
{"event": "end-file", "reason": "eof"}
{"event": "property-change", "name": "time-pos", "data": 45.2}
{"event": "property-change", "name": "duration", "data": 213.0}
{"event": "property-change", "name": "pause", "data": false}
```

### 8.3 Queue management

The app maintains the queue, not mpv. Per-track `loadfile` with `replace`. On `end-file reason=eof`, app increments `current_index` and loads the next track. Gapless cross-track (preloading next URL before current ends) is a v2 optimization — v1 accepts a tiny gap between tracks.

### 8.4 Audio configuration

Populated from config, applied at mpv launch. Changes in Settings restart mpv. Audio device list comes from `mpv --audio-device=help` output parsing.

### 8.5 Lifecycle

- Spawned after wizard completes or if config exists on startup
- Crash detection: socket closed → show toast, respawn, resume from last position
- On app exit: send `quit`, wait for process exit

## 9. Error Handling

| Scenario | Behavior |
|---|---|
| Network error (API) | Non-blocking toast: "Connection error: \<msg\>". Retry on next interaction. |
| Auth failure (401/403) | Toast: "Authentication failed. Check settings." Does not auto-kick to wizard. |
| mpv crash | Detect socket close, toast "Audio engine restarted", respawn, resume from last position. |
| mpv not found | Wizard step 4 error: "mpv not found. Install: pacman -S mpv". UI works, no audio until installed. |
| Cover art fetch fails | Placeholder (gray square / music note). Non-blocking. |
| Empty library | "No music found on server" with hint to check Navidrome config. |
| No search results | "No results for '\<query\>'". |
| Empty album/playlist | "This album is empty". |
| Escape on Home | No-op (can't go back from root). |
| Corrupt config | Fall back to defaults, toast "Config reset to defaults". |
| Server unreachable on startup | "Reconnect" banner on Home with last error. Wizard not re-shown. |

## 10. Edge Cases

- **Large libraries (10k+ artists):** Virtualized rendering, paginated fetch (100 per page), lazy-load artist letter sections on scroll.
- **Multi-disc albums:** If disc number available (OpenSubsonic extension), group tracks by disc with sub-headers. Otherwise flat list by track number.
- **Slow connection:** mpv handles buffering. Show "Buffering..." on `paused-for-cache` event.
- **Sleep/wake:** API calls wrapped in retry (3 attempts, backoff). Reconnect banner on persistent failure.
- **Concurrent cover art requests:** Limit to 8 concurrent, queue excess.
- **Queue persistence:** Save queue + position to `~/.config/navidrome-htpc/queue.json` on exit. On startup, resume if `playback.resume_on_start = true`. Also use Subsonic `savePlayQueue`/`getPlayQueue` if server supports it.

## 11. Testing

No automated test framework. Manual verification with debug flags:

- `--mock-server`: hardcoded artists/albums/tracks instead of real API. Tests full UI + navigation + focus without a Navidrome server.
- `--mock-mpv`: simulate mpv events (time-pos advancing, end-file) without spawning mpv. Tests now-playing + transport + queue without audio.
- `--log-level debug`: verbose logging of keyboard events, focus transitions, API calls, mpv commands.

The `focus.rs` module contains pure functions (zone transitions) that could be unit tested if a test module is added later.

## 12. Project Structure

```
navidrome-htpc/
├── Cargo.toml
├── config.toml.example
├── docs/
│   └── superpowers/specs/
│       └── 2026-07-19-navidrome-htpc-design.md   (this file)
└── src/
    ├── main.rs                 # entry point, eframe launch
    ├── app.rs                  # AppState, eframe::App impl, keyboard dispatch
    ├── config.rs               # Config struct, load/save TOML
    ├── state.rs                # Shared app state (Arc<RwLock>)
    ├── focus.rs                # FocusZone enum, focus navigation logic
    ├── theme.rs                # Colors, fonts, styling
    ├── subsonic/
    │   ├── mod.rs              # SubsonicClient wrapper, command channel
    │   ├── commands.rs         # SubsonicCommand enum
    │   ├── models.rs           # Domain types (Artist, Album, Track, Playlist)
    │   └── cover_art.rs        # Cover art fetcher + disk/memory cache
    ├── mpv/
    │   ├── mod.rs              # MpvController, subprocess management
    │   ├── ipc.rs              # JSON IPC protocol
    │   └── events.rs           # MpvEvent enum, event parsing
    └── ui/
        ├── mod.rs              # UI dispatch (which view to render)
        ├── home.rs             # Home (rows, cards, recently added/played)
        ├── artist_list.rs      # Sortable/filterable artist list
        ├── artist_detail.rs    # Artist's album grid
        ├── album_list.rs       # Sortable/filterable album list
        ├── album_detail.rs     # Album track list
        ├── playlist_list.rs    # Playlist list
        ├── playlist_detail.rs  # Playlist track list
        ├── search.rs           # Search view
        ├── now_playing.rs      # Now playing + play queue
        ├── settings.rs         # Settings view
        ├── wizard.rs           # Connection wizard (4 steps)
        ├── transport.rs        # Bottom transport bar
        ├── menu.rs             # Bottom-left ☰ menu
        └── common.rs           # Shared widgets (cards, thumbnails, list items)
```

### 12.1 Crate dependencies

```toml
[dependencies]
eframe = { version = "0.30", default-features = false, features = ["default_fonts", "wayland", "glow"] }
egui = { version = "0.30", default-features = false, features = ["default_fonts"] }
egui_extras = { version = "0.30", features = ["image"] }
opensubsonic = "0.x"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
image = "0.25"
crossbeam = "0.8"
tracing = "0.1"
tracing-subscriber = "0.3"
dirs = "5"
uuid = { version = "1", features = ["v4"] }
```

### 12.2 Module responsibilities

- `app.rs`: owns `AppState`, implements `eframe::App::update()`, reads keyboard, dispatches to focus logic, renders view + transport + menu
- `state.rs`: `AppState` struct — view stack, focus zone, fetched data, playback state, config. Passed as `&mut` to render functions.
- `focus.rs`: pure logic for zone transitions (current state + key → new state)
- `subsonic/`: async client thread. `SubsonicClient::start()` spawns tokio task, reads command channel, writes results to shared state.
- `mpv/`: mpv subprocess thread. `MpvController::start()` spawns process + IPC reader/writer.
- `ui/`: each view is `pub fn render(ui: &mut egui::Ui, state: &mut AppState)`. No view owns state.

## 13. Implementation Phases (high-level — detailed plan follows)

1. **Skeleton:** Cargo project, eframe window, config load/save, AppState struct
2. **Connection wizard:** 4-step wizard, Subsonic client connection
3. **Subsonic data layer:** command channel, domain models, cover art cache
4. **Home view:** Row 1 cards, Row 2/3 horizontal-scroll album thumbnails
5. **Focus management:** FocusZone enum, directional navigation, keyboard dispatch
6. **Library views:** ArtistList, ArtistDetail, AlbumList, AlbumDetail, PlaylistList, PlaylistDetail
7. **Search view:** debounced search, results display
8. **mpv integration:** subprocess, IPC, play/pause/seek/volume
9. **Play queue + NowPlaying:** queue management, auto-switch, auto-scroll
10. **Transport bar:** painter-based controls, progress slider, volume
11. **Context menu:** Play Now / Shuffle / Add to Queue flyout
12. **Settings view:** all config categories, live apply
13. **Error handling:** toasts, reconnect, mpv crash recovery
14. **Polish:** theming, UI scale, animations, edge cases
