use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

pub struct MpvIpc {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl MpvIpc {
    pub fn connect(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let stream = UnixStream::connect(path)?;
        // Set a read timeout so read_event() doesn't block forever when mpv
        // is idle (no file loaded → no events). Without this, the mpv loop
        // in run_mpv_loop() hangs on read_event() and never processes queued
        // commands (Play, Pause, etc.) — the command channel is never polled.
        stream.set_read_timeout(Some(Duration::from_millis(100)))?;
        let writer = stream.try_clone()?;
        Ok(Self {
            reader: BufReader::new(stream),
            writer,
        })
    }

    pub fn send_command(&mut self, command: &[Value]) -> Result<(), Box<dyn std::error::Error>> {
        let msg = json!({ "command": command });
        let line = serde_json::to_string(&msg)? + "\n";
        self.writer.write_all(line.as_bytes())?;
        Ok(())
    }

    pub fn set_property(
        &mut self,
        name: &str,
        value: Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.send_command(&[
            Value::String("set_property".into()),
            Value::String(name.into()),
            value,
        ])
    }

    pub fn get_property(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.send_command(&[
            Value::String("get_property".into()),
            Value::String(name.into()),
        ])
    }

    pub fn loadfile(&mut self, url: &str, mode: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.send_command(&[
            Value::String("loadfile".into()),
            Value::String(url.into()),
            Value::String(mode.into()),
        ])
    }

    /// Read one JSON event from mpv's IPC socket.
    ///
    /// Returns `Ok(None)` when the read times out (no event available) so the
    /// caller can poll its command channel and retry. Returns `Ok(Some(val))`
    /// on a successfully parsed event. Returns `Err(...)` on I/O errors or
    /// socket close.
    pub fn read_event(&mut self) -> Result<Option<Value>, Box<dyn std::error::Error>> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line);
        match n {
            Ok(0) => Err("mpv socket closed".into()),
            Ok(_) => {
                let val: Value = serde_json::from_str(line.trim())?;
                Ok(Some(val))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                // No event available within the read timeout — return None
                // so the caller can check for commands and spin again.
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn observe_property(
        &mut self,
        id: u64,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.send_command(&[
            Value::String("observe_property".into()),
            Value::Number(serde_json::Number::from(id)),
            Value::String(name.into()),
        ])
    }
}
