//! `$XDG_CONFIG_HOME/egc/settings` (usually `~/.config/egc/settings`), as
//! `key = value` lines. Every setting has a
//! default that works with no file at all, and a line that makes no sense keeps
//! its default rather than failing, so a broken file never locks you out.

use std::fs;
use std::path::{Path, PathBuf};

pub struct Settings {
    /// What `egc init` calls a vault when you give it no name.
    pub name: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            name: "vault".into(),
        }
    }
}

pub fn path() -> PathBuf {
    crate::config_dir().join("settings")
}

pub fn load() -> Settings {
    load_from(&path())
}

pub fn load_from(path: &Path) -> Settings {
    let mut s = Settings::default();
    let Ok(text) = fs::read_to_string(path) else {
        return s;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        if key.trim() == "name" && !value.is_empty() {
            s.name = value.into();
        }
    }
    s
}
