# Navidrome HTPC Client

A desktop music client for [Navidrome](https://www.navidrome.org/) servers, designed for HTPC (Home Theater PC) use with a Pepper Jobs W10 gyro remote. Controllable entirely via D-pad (arrow keys), Enter, Space, and media keys, with optional mouse support. Targets a 10-foot UI paradigm with large, navigable elements.

## Features

- **Remote-first navigation:** Pure directional (D-pad) focus management — no Tab cycling. Three zones (Content, Menu, Transport) navigated by arrow keys.
- **Full-screen Now Playing view:** Play queue with auto-scroll to current track, large album art, progress bar.
- **Drill-in navigation:** Home page with section cards, Recently Added/Played rows. Escape to go back at each level.
- **Audiophile audio:** Bit-perfect output via mpv subprocess with exclusive mode, gapless playback, and ReplayGain support.
- **Subsonic API:** Uses [opensubsonic-rs](https://github.com/M0Rf30/opensubsonic-rs) for full Subsonic v1.16.1 + OpenSubsonic compatibility.
- **Connection wizard:** 4-step first-launch setup (server URL, credentials, test, audio output).
- **Context menu:** Play Now / Shuffle Play / Add to Queue flyout on any album card or track row.

## Prerequisites

- **Rust** 1.85+ (edition 2021)
- **mpv** — audio playback engine
  - Arch: `pacman -S mpv`
  - Debian/Ubuntu: `apt install mpv`
  - macOS: `brew install mpv`
- **A Navidrome server** (or any Subsonic-compatible server)

## Building

```bash
git clone <repo-url>
cd navidrome-htpc
cargo build --release
```

The binary will be at `target/release/navidrome-htpc`.

## Running

```bash
cargo run --release
```

Or run the compiled binary directly:

```bash
./target/release/navidrome-htpc
```

### First Launch

On first launch, the connection wizard appears:

1. **Server URL** — Enter your Navidrome server URL (e.g., `https://music.example.com:4533`)
2. **Credentials** — Username, password, and auth method (Token recommended, API Key, or Plain)
3. **Test Connection** — Verifies connectivity (currently auto-succeeds; real test is a follow-up)
4. **Audio Output** — Select audio device and exclusive mode toggle

After completing the wizard, config is saved to `~/.config/navidrome-htpc/config.toml` and the Home view appears.

### Configuration

Config is stored at `~/.config/navidrome-htpc/config.toml` (platform-specific via the `dirs` crate). See `config.toml.example` for the format. Settings can also be changed in-app via the Settings view (☰ menu → Settings).

```toml
[server]
url = "https://music.local:4533"
username = "admin"
auth_method = "token"  # token | api_key | plain
password = ""
api_key = ""

[audio]
device = "auto"         # "auto" or specific alsa/hw device string
exclusive = true        # bit-perfect exclusive mode
gapless = true          # gapless playback
replaygain = "album"    # off | track | album
max_bitrate = 0         # 0 = unlimited

[display]
scale = 1.5             # UI scale (1.0, 1.25, 1.5, 2.0)
theme = "dark"

[playback]
scrobble = true
auto_advance = true
resume_on_start = true

[cache]
dir = "~/.cache/navidrome-htpc"
cover_art_size = 300    # pixels

[wizard]
completed = false
```

## Usage

### Navigation

The UI uses a drill-in model with three focus zones:

- **Content** — Main viewing area (rows, grids, lists)
- **Menu** — Bottom-left ☰ icon (expands to Search, Settings, Now Playing)
- **Transport** — Bottom bar (Prev, Play/Pause, Stop, Next, Progress, Volume)

| Key | Action |
|-----|--------|
| Arrow keys | Navigate within/between zones (directional, no Tab) |
| Enter | Activate focused item (drill in, play, select) |
| Escape | Go back (pop view stack) or close menu |
| Space / Play-Pause | If nothing playing: start queue + jump to Now Playing. If playing: toggle play/pause. |
| Stop (media key) | Stop playback, clear current track |
| Next/Prev (media key) | Skip tracks in queue |
| Volume Up/Down/Mute | Adjust volume |

### Views

- **Home** — Section cards (Artists, Albums, Playlists) + Recently Added/Played horizontal scrolls
- **Artist List** — Sortable list of all artists
- **Artist Detail** — Artist's album grid
- **Album List** — Sortable album grid (Newest, Name, Artist, Random)
- **Album Detail** — Album art, track list, Play/Shuffle/Add to Queue buttons
- **Playlist List/Detail** — Browse and play playlists
- **Search** — Full-screen search with debounced auto-search, results by Artists/Albums/Songs
- **Now Playing** — Large album art, progress bar, play queue with auto-scroll
- **Settings** — Connection, Audio, Display, Playback, Cache categories

### Play Queue

- **Play** (from album/playlist detail) — Replaces queue, starts playing, switches to Now Playing
- **Shuffle** — Replaces queue (shuffled), starts playing
- **Add to Queue** — Appends to current queue, shows toast
- **Context menu** (Right arrow on a card/track) — Play Now / Shuffle Play / Add to Queue flyout

## Debug Features

```bash
# Mock server (uses hardcoded data instead of real API)
cargo run --features mock-server

# Mock mpv (simulates playback events without spawning mpv)
cargo run --features mock-mpv

# Verbose logging
RUST_LOG=debug cargo run
```

## Architecture

Three-thread model:

```
┌──────────┐  channels  ┌──────────────────┐
│  egui    │◄──────────►│  Subsonic Client  │
│  UI      │            │  (opensubsonic-rs)│
│ (eframe) │            │  async / tokio    │
│          │  channels  ┌──────────────────┐
│          │◄──────────►│  mpv subprocess   │
│          │            │  (JSON IPC socket)│
└──────────┘            └──────────────────┘
```

- **UI thread** — egui immediate-mode rendering at 60fps, keyboard/mouse input, focus management
- **Subsonic client thread** — async HTTP via opensubsonic-rs, receives commands via crossbeam channel, returns results via `Arc<RwLock<>>` shared state
- **mpv subprocess thread** — spawns `mpv --idle --input-ipc-server=<socket>`, sends JSON commands, reads events, monitors process health

## Project Structure

```
src/
├── main.rs                 # Entry point, eframe launch
├── app.rs                  # NavidromeApp, eframe::App impl, keyboard dispatch
├── config.rs               # Config struct, TOML load/save
├── state.rs                # AppState, View/FocusZone enums, FocusState
├── focus.rs                # Directional focus navigation logic
├── theme.rs                # Colors, apply_theme()
├── subsonic/
│   ├── mod.rs              # SubsonicClient, command channel, API mapping
│   ├── commands.rs         # SubsonicCommand enum, FetchResults
│   ├── models.rs           # Domain types (Artist, Album, Track, Playlist)
│   └── cover_art.rs        # CoverArtCache (disk + memory LRU)
├── mpv/
│   ├── mod.rs              # MpvController, subprocess lifecycle
│   ├── ipc.rs              # JSON IPC protocol (Unix socket)
│   └── events.rs           # MpvEvent, MpvState
└── ui/
    ├── mod.rs              # View dispatch
    ├── home.rs             # Home view
    ├── artist_list.rs      # Sortable artist list
    ├── artist_detail.rs    # Artist's album grid
    ├── album_list.rs       # Sortable album grid
    ├── album_detail.rs     # Album track list + Play/Shuffle/Add
    ├── playlist_list.rs    # Playlist list
    ├── playlist_detail.rs  # Playlist track list
    ├── search.rs           # Search view (debounced)
    ├── now_playing.rs      # Now Playing + play queue + auto-scroll
    ├── settings.rs         # Settings view
    ├── wizard.rs           # 4-step connection wizard
    ├── transport.rs        # Transport bar (painter-based)
    ├── menu.rs             # Bottom-left ☰ menu
    └── common.rs           # Shared widgets (Card, Thumbnail, TrackRow, Toast, ContextMenu)
```

## Known Limitations / Follow-ups

- **Virtualized rendering** — Lists/grids use egui's ScrollArea (renders all items). For 10k+ item libraries, virtualized rendering should be added.
- **Cover art wiring** — `CoverArtCache` is implemented but not yet wired into views as a field on `NavidromeApp`.
- **Queue persistence** — Save/load play queue on app exit/startup is not yet implemented.
- **Wizard connection test** — Step 3 auto-succeeds; real server ping is a follow-up.
- **Keyboard navigation** — Full arrow-key zone transitions and Enter-on-card activation are partially implemented (Escape works, Space/Play-Pause works, transport focus works).
- **Cross-platform audio** — mpv IPC uses Unix sockets (Linux/macOS). Windows named pipe support is a follow-up.

## License

MIT
