use std::path::PathBuf;

use serde::Deserialize;

pub const STORE_ENV: &str = "MDMARKS_STORE";

#[derive(Debug, Default, Deserialize)]
struct ConfigFile {
    store: Option<String>,
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

pub fn resolve_store_path() -> Result<PathBuf, ConfigError> {
    if let Some(raw) = std::env::var_os(STORE_ENV) {
        if !raw.is_empty() {
            return expand_tilde(&raw.to_string_lossy());
        }
    }

    if let Some(store) = store_from_config_file()? {
        return expand_tilde(&store);
    }

    expand_tilde("~/mdmarks")
}

fn store_from_config_file() -> Result<Option<String>, ConfigError> {
    let path = home_dir()?.join(".config/mdmarks/config.toml");
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(ConfigError::Io(e)),
    };
    let parsed: ConfigFile = toml::from_str(&contents).map_err(ConfigError::Toml)?;
    Ok(parsed.store)
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

fn home_dir() -> Result<PathBuf, ConfigError> {
    std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
        .ok_or(ConfigError::NoHome)
}
