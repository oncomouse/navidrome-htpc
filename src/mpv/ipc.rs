use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

pub struct MpvIpc {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl MpvIpc {
    pub fn connect(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let stream = UnixStream::connect(path)?;
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

    pub fn read_event(&mut self) -> Result<Option<Value>, Box<dyn std::error::Error>> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line)?;
        if n == 0 {
            return Err("mpv socket closed".into());
        }
        let val: Value = serde_json::from_str(line.trim())?;
        Ok(Some(val))
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
