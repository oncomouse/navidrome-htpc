# Task 14 Report — Cover Art Cache + Error Handling + Polish (FINAL)

**Status:** DONE

## Summary

Implemented the final polish task: cover-art disk+memory cache, global
Space/Play-Pause key handling, and confirmed error toasts + mpv crash
recovery (already present from earlier tasks). App builds cleanly.

## Changes

### `src/subsonic/cover_art.rs` (replaced stub)
- `CoverArtCache` struct with in-memory `HashMap<String, TextureHandle>`
  and on-disk JPEG cache under a configurable cache dir.
- `new(cache_dir)`: creates dir tree best-effort.
- `get(id)`: in-memory lookup.
- `insert(id, texture)`: manual insert (for tests / future async path).
- `fetch_blocking(ctx, id, url, size)`: disk-first, then HTTP GET via
  `reqwest::blocking`, decode with `image`, upload as egui texture.
  Bakes `size` into the disk filename to avoid size collisions.
- `clear_memory()`: drop textures (e.g. on cover-art-size setting change).

### `Cargo.toml`
- Added `"blocking"` feature to `reqwest` so `reqwest::blocking::get`
  is available.

### `src/app.rs`
- Added global Space / Play-Pause key handling in `update()`:
  - Suppresses Space when the Search view's text field has focus so the
    literal space character reaches the input.
  - Delegates to `handle_play_pause_global()`.
- Added `handle_play_pause_global()` method on `NavidromeApp`:
  - **Playing** → `MpvCommand::TogglePause`, no view change (mpv is the
    source of truth for the paused state; next poll refreshes
    `state.is_playing`).
  - **Paused (current_track_index=Some, is_playing=false)** →
    `MpvCommand::Resume` + push NowPlaying if not already there.
  - **Fresh start (queue non-empty, nothing playing)** → set
    `current_track_index=Some(0)`, `is_playing=true`, push NowPlaying;
    the existing mpv poll loop detects the `is_playing && !mpv_playing`
    transition and sends the stream URL on the next frame.
  - **Empty queue** → toast "Queue is empty — nothing to play".

### Already-present features (verified in app.rs from earlier tasks)
- Error toasts on Subsonic errors (`results.error` → toast) — already in
  the poll path.
- mpv crash recovery: `mpv_state.crashed && was_playing` surfaces a
  "mpv subprocess crashed" toast — already in place.
- Stream-URL-build failures surface toasts — already in place.

## Verification

```
$ cargo check
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.15s
   (0 errors, 35 warnings — all pre-existing unused-import / f32-fallback
   warnings from earlier tasks)

$ cargo test
   test result: ok. 0 passed; 0 failed; 0 ignored
   (No unit tests in the project; verification is manual per spec §11.)
```

## Commits

- `feat: cover art cache, error handling, global key handling, polish`

## Test summary

`cargo check` passes (0 errors); `cargo test` passes (0 tests, manual
testing per spec).

## Known gaps (per brief's Self-Review)

- Cover art fetch is blocking — acceptable for v1, should be moved onto
  the async Subsonic client thread in a follow-up.
- `CoverArtCache` is defined and compiles, but not yet wired into
  `NavidromeApp` (no field on the struct, no call sites in views). This
  was deliberate: the brief said "the SubsonicClient already has a
  cover_art module declared. Just implement the struct and methods."
  Wiring into the views (Home/AlbumList/etc. rendering cover thumbnails)
  is a follow-up task — the type is ready to use.
- Virtualized rendering for 10k+ item lists: deferred (per brief).
- Queue persistence: deferred (per brief).

## Report file

`/home/andrew/Projects/navidrome-htpc/.superpowers/sdd/reports/task-14-report.md`
