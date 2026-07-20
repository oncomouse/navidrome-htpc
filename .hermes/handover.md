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
  - **INCONSISTENCY — FIXED (2026-07-20, commit pending):** `app.rs:297`
    opens the context menu on Space. The dead `Key::Space =>
    FocusAction::PlayPauseToggle` arm was removed from `focus.rs:handle_key`
    (that fn is only ever called with `Key::Escape`, so the Space arm was
    unreachable/misleading). `PlayPauseToggle` remains as an enum variant
    but is no longer implied to be wired to Space.
- **Shuffle icon — FIXED (2026-07-20, commit pending):** the two detail-view
  header buttons (`album_detail.rs:57`, `playlist_detail.rs:26`) now use
  `🔀` (U+1F500) to match the context menu (`common.rs:332`). No more
  duplicate `▶▶` glyph.

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
  clicks Pause, the auto-switch pushes NowPlaying.
  - **ROOT CAUSE (traced 2026-07-20):** mpv's `pause` command
    leaves `MpvState.is_playing = true` (only `start-file`/`end-file`
    touch `is_playing`; the `pause` property-change only flips
    `is_paused`). So for 1-2 frames after a Pause click, mpv still
    reports `is_playing=true` while `is_paused` is becoming true. The
    pending-action block sets `is_playing=false` on the click frame, but
    on the NEXT frame `was_playing` (prev end-state) is false while the
    poll's `is_playing` is still true → the auto-switch condition
    `is_playing && !was_playing` reads as "playback just started" and
    spuriously pushes NowPlaying.
  - **FIXED (2026-07-20, commit pending):** added `&&
    !mpv_state.is_paused` to the auto-switch guard in `app.rs`. Every
    legitimate auto-switch (initial play, queue advance) has
    `is_paused=false`; the pause-lag window has `is_paused=true`, so the
    spurious switch is suppressed without affecting real switches.
- **No auto-scroll gate:** the NowPlaying queue scrolls every frame to center the
  current track (pitfall #14). Add `last_scrolled_track: Option<usize>` to
  AppState and only scroll when the index changes.
  - **FIXED (2026-07-20, commit pending):** added `AppState::last_scrolled_track`
    and gated the `scroll_to_rect` call in `now_playing.rs` on
    `state.last_scrolled_track != Some(current_idx)`, updating it on scroll.
    The queue now only auto-centers when the current track changes, so manual
    queue scrolling is no longer fought every frame.
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
- **Play/pause button flicker (2026-07-20, FIXED):** the transport icon was
  drawn from `state.is_playing`, which the per-frame mpv poll
  (`app.rs:438`) overwrites for 1-2 frames after a click while mpv's IPC
  catches up — so the ▶/⏸ icon flickered. Fix: added an intent latch
  (`state.intended_playing: Option<bool>` + `intent_frames_remaining: u16`).
  The transport button renders from the latch when set; `app.rs`
  reconciliation clears it once mpv's *raw poll* (`mpv_state`, NOT
  `state.is_playing` which the pending-action block sets synchronously)
  converges to the intent, or after a 30-frame (~0.5s) safety budget, or if
  the track index clears. Constant `INTENT_LATCH_FRAMES` in transport.rs.
- **Transport bar visibility (2026-07-20):** Prev/Play/Stop/Next +
  progress slider are hidden unless `state.current_track_index.is_some()`
  (gated in `src/ui/transport.rs` render). Volume stays always
  visible. When empty, `FocusZone::Transport` bounces to Content
  so keyboard focus isn't trapped. (Todo item #2 done.)
- **Detail-view header buttons keyboard-accessible (2026-07-20, FIXED):**
  the Play / Shuffle / Add to Queue buttons at the top of album/playlist
  detail pages were plain egui `Button`s fired only by mouse `.clicked()`,
  outside the custom focus system — so they had no keyboard path or
  highlight. Fix: added a `FocusZone::Header` zone (with `header_index`
  0=Play,1=Shuffle,2=Add), a shared `render_header_button` helper in
  common.rs (painter-based, highlights via the focus zone), and a
  `handle_header_arrow` nav (Left/Right between buttons, Up/Down back to the
  track list). On detail views, ArrowUp from track row 0 enters the header;
  Enter in the header runs the same action as a click (dispatched centrally
  in app.rs, mirroring the per-view track activation). Supports both mouse
  and keyboard (per user convention "Click = mouse OR Enter").
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
