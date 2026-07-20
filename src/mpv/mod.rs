pub mod events;
pub mod ipc;

use std::process::{Child, Command, Stdio};
use std::sync::{Arc, RwLock};

use crossbeam::channel::{self, Receiver, Sender};

use crate::config::AudioConfig;
use events::MpvState;
use ipc::MpvIpc;

#[derive(Debug)]
pub enum MpvCommand {
    Play { url: String },
    Append { url: String },
    Pause,
    Resume,
    TogglePause,
    Stop,
    Seek(f32),
    SetVolume(f32),
    Next,
    Previous,
    Quit,
}

pub struct MpvController {
    pub command_tx: Sender<MpvCommand>,
    pub state: Arc<RwLock<MpvState>>,
}

impl MpvController {
    pub fn start(config: AudioConfig) -> Option<Self> {
        let socket_path = format!("/tmp/navidrome-htpc-mpv-{}.sock", std::process::id());
        let replaygain = match config.replaygain {
            crate::config::ReplayGainMode::Off => "no",
            crate::config::ReplayGainMode::Track => "track",
            crate::config::ReplayGainMode::Album => "album",
        };

        // Spawn mpv
        let child = Command::new("mpv")
            .arg("--idle")
            .arg(format!("--input-ipc-server={socket_path}"))
            .arg(format!(
                "--gapless-audio={}",
                if config.gapless { "yes" } else { "no" }
            ))
            .arg(format!(
                "--audio-exclusive={}",
                if config.exclusive { "yes" } else { "no" }
            ))
            .arg(format!("--audio-device={}", config.device))
            .arg(format!("--replaygain={replaygain}"))
            .arg("--no-video")
            .arg("--terminal=no")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;

        // Wait for socket to appear
        std::thread::sleep(std::time::Duration::from_millis(500));

        let ipc = MpvIpc::connect(&socket_path).ok()?;

        // Observe properties
        let mut ipc = ipc;
        let _ = ipc.observe_property(1, "time-pos");
        let _ = ipc.observe_property(2, "duration");
        let _ = ipc.observe_property(3, "pause");

        let (command_tx, command_rx) = channel::unbounded::<MpvCommand>();
        let state = Arc::new(RwLock::new(MpvState::default()));
        let state_clone = state.clone();

        std::thread::spawn(move || {
            run_mpv_loop(child, ipc, command_rx, state_clone, socket_path);
        });

        Some(Self { command_tx, state })
    }

    pub fn send(&self, cmd: MpvCommand) {
        let _ = self.command_tx.send(cmd);
    }

    pub fn poll(&self) -> MpvState {
        self.state.read().unwrap().clone()
    }
}

fn run_mpv_loop(
    mut child: Child,
    mut ipc: MpvIpc,
    command_rx: Receiver<MpvCommand>,
    state: Arc<RwLock<MpvState>>,
    _socket_path: String,
) {
    loop {
        // Process commands (non-blocking)
        while let Ok(cmd) = command_rx.try_recv() {
            match cmd {
                MpvCommand::Play { url } => {
                    let _ = ipc.loadfile(&url, "replace");
                }
                MpvCommand::Append { url } => {
                    let _ = ipc.loadfile(&url, "append");
                }
                MpvCommand::Pause => {
                    let _ = ipc.set_property("pause", true.into());
                }
                MpvCommand::Resume => {
                    let _ = ipc.set_property("pause", false.into());
                }
                MpvCommand::TogglePause => {
                    let new_pause = {
                        let s = state.read().unwrap();
                        !s.is_paused
                    };
                    let _ = ipc.set_property("pause", new_pause.into());
                }
                MpvCommand::Stop => {
                    let _ = ipc.send_command(&["stop".into()]);
                    let mut s = state.write().unwrap();
                    s.is_playing = false;
                    s.current_time = 0.0;
                    s.total_duration = 0.0;
                    s.current_track_index = None;
                }
                MpvCommand::Seek(pos) => {
                    let _ = ipc.set_property("time-pos", pos.into());
                }
                MpvCommand::SetVolume(vol) => {
                    let _ = ipc.set_property("volume", (vol * 100.0).into());
                }
                MpvCommand::Quit => {
                    let _ = ipc.send_command(&["quit".into()]);
                    break;
                }
                MpvCommand::Next | MpvCommand::Previous => {
                    // Handled by app (queue management), not mpv
                }
            }
        }

        // Read events (blocking read_line; the loop sleeps 16ms between
        // iterations which is acceptable for v1 per the task brief).
        //
        // With the 100ms read timeout set on the IPC socket, read_event()
        // returns Ok(None) when no event arrives within the timeout. This
        // lets the loop check for commands and spin again instead of
        // blocking forever when mpv is idle.
        match ipc.read_event() {
            Ok(Some(val)) => {
                if let Some(event) = val.get("event").and_then(|e| e.as_str()) {
                    handle_mpv_event(event, &val, &state);
                }
            }
            Ok(None) => {}
            Err(_) => {
                // Socket closed — mpv crashed
                let mut s = state.write().unwrap();
                s.crashed = true;
                break;
            }
        }

        // Check if child process exited
        match child.try_wait() {
            Ok(Some(_)) => {
                let mut s = state.write().unwrap();
                s.crashed = true;
                break;
            }
            Ok(None) => {}
            Err(_) => break,
        }

        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    // Kill the mpv subprocess when the loop exits (app shutting down, socket
    // closed, or mpv crashed). Without this the child process survives as a
    // zombie that keeps playing audio indefinitely.
    let _ = child.kill();
    let _ = child.wait();
}

fn handle_mpv_event(event: &str, val: &serde_json::Value, state: &Arc<RwLock<MpvState>>) {
    let mut s = state.write().unwrap();
    match event {
        "start-file" => {
            s.is_playing = true;
            s.is_paused = false;
        }
        "end-file" => {
            // Track ended — app will handle advancing the queue
            s.is_playing = false;
        }
        "property-change" => {
            let name = val.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let data = val.get("data");
            match name {
                "time-pos" => {
                    if let Some(t) = data.and_then(|d| d.as_f64()) {
                        s.current_time = t as f32;
                    }
                }
                "duration" => {
                    if let Some(d) = data.and_then(|d| d.as_f64()) {
                        s.total_duration = d as f32;
                    }
                }
                "pause" => {
                    if let Some(p) = data.and_then(|d| d.as_bool()) {
                        s.is_paused = p;
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}
