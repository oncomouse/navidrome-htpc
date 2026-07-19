use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub server: ServerConfig,
    pub audio: AudioConfig,
    pub display: DisplayConfig,
    pub playback: PlaybackConfig,
    pub cache: CacheConfig,
    pub wizard: WizardConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerConfig {
    pub url: String,
    pub username: String,
    pub auth_method: AuthMethod,
    pub password: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum AuthMethod {
    #[default]
    Token,
    ApiKey,
    Plain,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioConfig {
    pub device: String,
    pub exclusive: bool,
    pub gapless: bool,
    pub replaygain: ReplayGainMode,
    pub max_bitrate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ReplayGainMode {
    #[default]
    Off,
    Track,
    Album,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisplayConfig {
    pub scale: f32,
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaybackConfig {
    pub scrobble: bool,
    pub auto_advance: bool,
    pub resume_on_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheConfig {
    pub dir: String,
    pub cover_art_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WizardConfig {
    pub completed: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                url: String::new(),
                username: String::new(),
                auth_method: AuthMethod::Token,
                password: String::new(),
                api_key: String::new(),
            },
            audio: AudioConfig {
                device: "auto".to_string(),
                exclusive: true,
                gapless: true,
                replaygain: ReplayGainMode::Album,
                max_bitrate: 0,
            },
            display: DisplayConfig {
                scale: 1.5,
                theme: "dark".to_string(),
            },
            playback: PlaybackConfig {
                scrobble: true,
                auto_advance: true,
                resume_on_start: true,
            },
            cache: CacheConfig {
                dir: dirs::cache_dir()
                    .unwrap_or_else(|| PathBuf::from("~/.cache"))
                    .join("navidrome-htpc")
                    .to_string_lossy()
                    .to_string(),
                cover_art_size: 300,
            },
            wizard: WizardConfig { completed: false },
        }
    }
}

impl Config {
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("navidrome-htpc").join("config.toml"))
    }

    pub fn load() -> Option<Config> {
        let path = Self::config_path()?;
        let contents = std::fs::read_to_string(&path).ok()?;
        toml::from_str(&contents).ok()
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path()
            .ok_or("Could not determine config directory")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&path, contents)?;
        Ok(())
    }
}
