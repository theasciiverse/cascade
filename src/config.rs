use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub gameplay: GameplayConfig,
    pub keys: KeysConfig,
    pub audio: AudioConfig,
    pub display: DisplayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameplayConfig {
    pub scroll_speed: f64,
    pub difficulty: String,
    #[serde(default = "default_true")]
    pub health_enabled: bool,
    #[serde(default = "default_true")]
    pub holds_enabled: bool,
    /// Drain mode: health continuously falls while playing. Only Perfects
    /// restore it. Implies `health_enabled`. Off by default.
    #[serde(default)]
    pub drain_mode: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysConfig {
    pub lanes: [char; 5],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub volume: f64,
    pub offset_ms: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String {
    "classic".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gameplay: GameplayConfig {
                scroll_speed: 1.0,
                difficulty: "hard".to_string(),
                health_enabled: true,
                holds_enabled: true,
                drain_mode: false,
            },
            keys: KeysConfig {
                lanes: ['d', 'f', ' ', 'j', 'k'],
            },
            audio: AudioConfig {
                volume: 0.8,
                offset_ms: 0,
            },
            display: DisplayConfig {
                theme: default_theme(),
            },
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Where settings, songs, scores and history live: `$XDG_STATE_HOME/cascade`
    /// when set, else `~/.local/state/cascade` (Linux and the BSDs),
    /// `~/Library/Application Support/cascade` (macOS) or
    /// `%LOCALAPPDATA%\cascade` (Windows). A `~/.cascade` from an earlier
    /// version is copied over the first time, so a library carries on.
    pub fn cascade_dir() -> std::path::PathBuf {
        static DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
        DIR.get_or_init(|| {
            let dir = state_dir();
            if let Some(old) = dirs::home_dir().map(|h| h.join(".cascade"))
                && old.is_dir()
                && !dir.exists()
            {
                match copy_tree(&old, &dir) {
                    Ok(()) => log::info!("copied {} to {}", old.display(), dir.display()),
                    Err(e) => log::warn!("could not copy {}: {e}", old.display()),
                }
            }
            dir
        })
        .clone()
    }

    pub fn default_path() -> std::path::PathBuf {
        Self::cascade_dir().join("config.toml")
    }
}

fn state_dir() -> std::path::PathBuf {
    let env = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(std::path::PathBuf::from)
    };
    let home = || dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let base = env("XDG_STATE_HOME").unwrap_or_else(|| {
        if cfg!(windows) {
            env("LOCALAPPDATA").unwrap_or_else(home)
        } else if cfg!(target_os = "macos") {
            home().join("Library/Application Support")
        } else {
            home().join(".local/state")
        }
    });
    base.join("cascade")
}

/// Copy a folder and everything in it.
pub fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}
