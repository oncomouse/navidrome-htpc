# Task 5 Report: mpv subprocess + JSON IPC

**Status:** DONE
**Commit:** (see git log)
**Test summary:** `cargo check` passes (edition 2021, 0 errors, only expected dead-code warnings for fields/methods to be wired up in later tasks)

## What was done

Implemented the mpv audio subprocess controller for the Navidrome HTPC client:

### Files created/modified

- **`src/mpv/events.rs`** — Created `MpvEvent` enum (StartFile, EndFile, TimePos, Duration, PauseChanged, TrackChanged) and `MpvState` struct (is_playing, is_paused, current_time, total_duration, current_track_index, volume, crashed) with `Default` derive.
- **`src/mpv/ipc.rs`** — Created `MpvIpc` struct with Unix-socket JSON IPC:
  - `connect(path)` — opens `UnixStream` and clones for read/write halves
  - `send_command`, `set_property`, `get_property`, `loadfile`, `observe_property` — write JSON commands newline-terminated
  - `read_event` — blocking `BufRead::read_line` returning `Option<Value>`
- **`src/mpv/mod.rs`** — Created `MpvController` with:
  - `MpvCommand` enum (Play, Append, Pause, Resume, TogglePause, Stop, Seek, SetVolume, Next, Previous, Quit)
  - `start(config)` — spawns `mpv --idle --input-ipc-server=<socket>` with audio config flags, waits 500ms for socket, connects, observes `time-pos`/`duration`/`pause`, spawns event-loop thread
  - `send(cmd)` — push command via crossbeam channel
  - `poll()` — clone shared `Arc<RwLock<MpvState>>`
  - `run_mpv_loop` — 16ms-tick loop: drains command queue (non-blocking `try_recv`), reads one event via `read_event`, checks child exit via `try_wait`, sets `crashed=true` on socket close or child exit
  - `handle_mpv_event` — maps `start-file`/`end-file`/`property-change` events onto `MpvState` updates
- **`src/app.rs`** — Added `pub mpv: Option<MpvController>` field; updated `NavidromeApp::new` signature to accept it.
- **`src/main.rs`** — Spawns `MpvController::start(config.audio.clone())` when `server_configured`, passes `mpv` into `NavidromeApp::new`.

## Verification

- `cargo check` finishes cleanly: `Finished dev profile [unoptimized + debuginfo] target(s) in 0.38s`.
- 39 warnings, all dead-code (`MpvCommand` variants, `MpvController::send`/`poll`, `MpvEvent`, `MpvState::volume`, `MpvIpc::get_property`) — expected, since these APIs are wired into the UI in later tasks. No errors.
- Edition confirmed as 2021 (resolves an earlier lint tool false-positive about `async fn` in `src/subsonic/mod.rs`).
- Did NOT run `cargo run` per instructions — mpv binary may be absent and no display is available; runtime spawn of mpv is not tested.

## Issues encountered

- Initial write triggered sibling-subagent warnings (this workspace is shared with another agent's work). Re-read the affected files to confirm they matched the brief before continuing — no merge conflicts.
- The `Read` import in `ipc.rs` was unused (only `BufRead::read_line` and `Write::write_all` are needed); removed it.
- Rustfmt-style lint suggestions (import ordering, line wrapping) were applied to keep the linter green; no behavioral change.

## Notes for downstream tasks

- `MpvCommand::Next`/`Previous` are no-ops in `run_mpv_loop` — queue advancement is app-side (Task 6+).
- `MpvState::crashed` is set but never observed by the app yet; a later task should surface it as a toast or trigger a respawn.
- The event loop's `read_event` is a blocking call, so the 16ms `sleep` between iterations is a lower bound on responsiveness, not a hard tick rate. Acceptable for v1 per the task brief.
