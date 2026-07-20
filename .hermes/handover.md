# Session Handover — July 20, 2026

## What was accomplished

### Fix: pending transport actions processed before auto-switch (71b05fe64)
**Root cause:** the `pending_transport_action` processing block ran AFTER the auto-switch
logic in the mpv Phase 4 poll block. When the user clicked Pause:

1. Transport handler set `state.is_playing = false` (desired)
2. mpv poll overwrote it to `true` (mpv still playing current frame)
3. Auto-switch saw `is_playing(true) && !was_playing(true) && …` and pushed
   NowPlaying — the view flipped while the user was just trying to pause
4. The actual Pause command was sent to mpv AFTER the auto-switch

**Fix:** moved pending-action processing to right after the crash check, BEFORE
initial-play, track-end, and auto-switch. Each action now directly sets
`state.is_playing` to the desired value so the UI icon is correct immediately.

### Keyboard navigation working
- Arrows = focus navigation (Up/Down/Left/Right)
- Enter = activate (Home cards, album thumbs, track lists, artist/playlist lists)
- Space = context menu (replaced Right arrow per user request)
  - **INCONSISTENCY (verified 2026-07-20):** `app.rs:297` opens the
    context menu on Space, which is correct. But `focus.rs:24` still maps
    `Key::Space => FocusAction::PlayPauseToggle`, and that `handle_key`
    branch is **dead** — `handle_key` is only ever called with `Key::Escape`
    (app.rs:252). The `PlayPauseToggle`/`Space` mapping in focus.rs is
    misleading leftover code; Space does NOT toggle play/pause.
- **Shuffle icon PARTIALLY STALE (verified 2026-07-20):** the context
  menu uses `🔀` (U+1F500) at `common.rs:332`, but the detail-view
  header buttons STILL use `▶▶` — `album_detail.rs:57` and
  `playlist_detail.rs:26` both render `"\u{25B6}\u{25B6} Shuffle"`.
  So the "changed from `▶▶` to `🔀`" claim is only half-true; the
  detail buttons were never updated. Two different shuffle glyphs ship.

### mpv IPC non-blocking (4c4fb7495)
- 100ms read timeout on UnixStream so commands are processed even when mpv is idle
- mpv thread uses `try_recv()` on timeout to drain command channel

## KNOWN BUG: Play after Stop is broken
## FIXED: Play after Stop (commit pending — 2026-07-20)

Root cause confirmed and fixed. Flow was:
1. Stop → `current_track_index = None`, mpv unloads file
2. Play → `TransportAction::Play` sent `MpvCommand::Resume` (just un-pauses)
   but mpv had no file loaded → silent no-op, UI stuck on ⏸ forever

Fix (3 changes + 1 init):
- `src/ui/transport.rs`: Stop handler now saves the index to the new
  `state.last_played_track_index` before clearing `current_track_index`.
- `src/state.rs`: added `last_played_track_index: Option<usize>` field.
- `src/main.rs`: initialized `last_played_track_index: None`.
- `src/app.rs` `TransportAction::Play` branch: if `current_track_index.is_some()`
  keep the old `MpvCommand::Resume` (un-pause). Else restore the index from
  `last_played_track_index` (falling back to `Some(0)` when the queue is
  non-empty but nothing has played yet), rebuild the stream URL via
  `subsonic.stream_url()`, and send `MpvCommand::Play { url }`. Empty/out-of-range
  queue leaves the UI stopped with a toast on URL failure.

Verification: `cargo check` passes (exit 0, only pre-existing dead-code
warnings). Runtime audio path NOT exercised here — needs a display + Navidrome
server (GUI app). Manual test: play a track, Stop, then Play → track should
restart from the beginning.

## Other known minor issues

- **Auto-switch on pause:** when user is deep in browsing (not on NowPlaying) and
  clicks Pause, the auto-switch pushes NowPlaying. Fixed in 71b05fe64 but the
  auto-switch still fires when mpv transitions from idle to playing — this is
  correct for queue advancement, but the Pause click also triggers a brief
  mpv-playing→idle transition that shouldn't auto-switch. The pending-action
  processing now sets `is_playing = false` before auto-switch runs, preventing
  this. Keep an eye on it.
- **No auto-scroll gate:** the NowPlaying queue scrolls every frame to center the
  current track (pitfall #14). Add `last_scrolled_track: Option<usize>` to
  AppState and only scroll when the index changes.
- **Cover art cache IS wired (verified 2026-07-20):** `CoverArtCache` is
  fully integrated — `NavidromeApp.cover_art_cache` field constructed in
  `new()`, `fetch_cover_arts_for_current_view()` runs every frame from
  `update()` (app.rs ~line 231), collects visible `(album_id, cover_id)`
  pairs via `collect_visible_cover_ids()` (Home/AlbumList/AlbumDetail/
  ArtistDetail/NowPlaying), fetches misses through `build_cover_art_url` →
  `CoverArtCache::fetch_blocking` (disk+memory+HTTP), then syncs into
  `state.cover_textures` keyed by `album_id`. All five album views read
  `state.cover_textures.get(&album.id)` and render via `render_album_thumbnail`.
  `cargo check` is clean. The old "not wired" note was stale.
  - **KNOWN DEBT (not a bug):** the fetch is BLOCKING on the UI thread
    (`reqwest::blocking` inside `fetch_blocking`, called from `update()`). On a
    cold cache each uncached cover serially blocks the 60fps render until its
    HTTP download finishes. A future iteration should move cover fetches onto
    the async Subsonic client thread (the `cover_art.rs` doc already notes
    this). Don't do this unprompted — it's an architectural change.
- **No virtualized rendering:** large lists (10k+ artists) render all items.
- **No queue persistence:** queue is lost on app restart.
- **Wizard backends:** `self.subsonic` / `self.mpv` are `Some` after
  post-wizard init, but `SubsonicClient::start` with wrong credentials returns a
  broken handle silently (client=None, worker thread not spawned). No retry
  mechanism.

## Git log (relevant commits)

```
71b05fe64 fix: pending transport actions processed before auto-switch
d8eb08aa3 fix: Right arrow opens context menu instead of moving focus; shuffle icon
db7544950 fix: transport bar commands never reach mpv; mpv survives app exit
4c4fb7495 fix: mpv IPC blocking read prevents command processing, Enter key activation
2aec9bd86 fix: cover art lookup key mismatch, missing keyboard navigation
```

## Architecture quick reference

**mpv Phase 4 block** (`app.rs` ~line 430-613):
1. Captures `was_playing`
2. Polls mpv state (overwrites is_playing/current_time/total_duration)
3. Crash toast check
4. **Pending transport actions** ← processes here, sets is_playing directly
5. Initial-play detection (was_playing guard)
6. Track-end detection (total_duration > 0.0 guard)
7. Auto-switch to NowPlaying

**Keyboard dispatch** (`app.rs` ~line 330-430):
- `keys.0` = Escape (pop view / close context menu)
- `keys.1` = Enter (activate focused item)
- `keys.2` = Space (open context menu)
- `keys.3` = ArrowUp
- `keys.4` = ArrowDown
- `keys.5` = ArrowLeft
- `keys.6` = ArrowRight

**Mpv module** (`src/mpv/`):
- `mod.rs`: MpvController::start (spawns mpv subprocess, connects socket),
  MpvIpc::connect, run_mpv_loop (command/event loop on separate thread)
- `ipc.rs`: UnixStream with 100ms read timeout, loadfile/set_property/send_command
- `events.rs`: MpvState { is_playing, is_paused, current_time, total_duration,
  current_track_index, crashed }

**App module** (`src/app.rs`):
- `MpvController` handle stored as `self.mpv: Option<MpvController>`
- Commands sent via `mpv.send(MpvCommand::*)` (crossbeam channel)
- State polled via `mpv.poll()` returns MpvState

## Build

```bash
cargo build --release
cargo check  # faster validation
```
