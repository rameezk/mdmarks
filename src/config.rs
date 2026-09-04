use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

pub const STORE_ENV: &str = "MDMARKS_STORE";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SpaceConfig {
    pub browser: String,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub chromium_support_dir: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ConfigFile {
    store: Option<String>,
    #[serde(default)]
    default_space: Option<String>,
    #[serde(default)]
    spaces: HashMap<String, SpaceConfig>,
}

#[derive(Debug)]
pub struct Config {
    pub store: PathBuf,
    pub default_space: Option<String>,
    pub spaces: HashMap<String, SpaceConfig>,
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    NoHome,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "reading config: {e}"),
            ConfigError::Toml(e) => write!(f, "parsing config.toml: {e}"),
            ConfigError::NoHome => write!(f, "could not determine home directory"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn load() -> Result<Config, ConfigError> {
        let file = read_config_file()?.unwrap_or_default();
        let store = match env_store() {
            Some(raw) => expand_tilde(&raw)?,
            None => store_or_default(file.store.as_deref())?,
        };
        Ok(Config {
            store,
            default_space: file.default_space,
            spaces: file.spaces,
        })
    }
}

pub fn resolve_store_path() -> Result<PathBuf, ConfigError> {
    if let Some(raw) = env_store() {
        return expand_tilde(&raw);
    }
    let from_file = read_config_file()?.and_then(|f| f.store);
    store_or_default(from_file.as_deref())
}

fn store_or_default(from_file: Option<&str>) -> Result<PathBuf, ConfigError> {
    match from_file {
        Some(store) => expand_tilde(store),
        None => expand_tilde("~/mdmarks"),
    }
}

fn env_store() -> Option<String> {
    std::env::var_os(STORE_ENV)
        .filter(|raw| !raw.is_empty())
        .map(|raw| raw.to_string_lossy().into_owned())
}

fn read_config_file() -> Result<Option<ConfigFile>, ConfigError> {
    let path = home_dir()?.join(".config/mdmarks/config.toml");
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(ConfigError::Io(e)),
    };
    let parsed: ConfigFile = toml::from_str(&contents).map_err(ConfigError::Toml)?;
    Ok(Some(parsed))
}

fn expand_tilde(raw: &str) -> Result<PathBuf, ConfigError> {
    if let Some(rest) = raw.strip_prefix("~/") {
        Ok(home_dir()?.join(rest))
    } else if raw == "~" {
        home_dir()
    } else {
        Ok(PathBuf::from(raw))
    }
}

pub fn app_support_dir() -> Result<PathBuf, ConfigError> {
    Ok(home_dir()?.join("Library/Application Support"))
}

fn home_dir() -> Result<PathBuf, ConfigError> {
    std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
        .ok_or(ConfigError::NoHome)
}
