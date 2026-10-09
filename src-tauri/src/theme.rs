//! Reads the active Omarchy theme and notifies the frontend when it changes.
//!
//! Omarchy keeps the current theme in `~/.local/state/omarchy/current/theme/`,
//! with a flat `colors.toml` (`background = "#20221f"`, `accent = …`, `mode = "dark"`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use notify::{RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Theme {
    /// Theme name from `theme.name`, e.g. "silo". `None` outside Omarchy.
    pub name: Option<String>,
    /// Every string value from `colors.toml`, keyed as in the file.
    pub colors: HashMap<String, String>,
    /// Family that fontconfig resolves for `monospace` (Omarchy sets it with `omarchy font set`).
    pub mono_font: Option<String>,
}

fn current_dir() -> Option<PathBuf> {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".local/state")))?;
    Some(state.join("omarchy/current"))
}

pub fn parse_colors(src: &str) -> HashMap<String, String> {
    let Ok(table) = src.parse::<toml::Table>() else {
        return HashMap::new();
    };
    table
        .into_iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_owned())))
        .collect()
}

fn mono_font() -> Option<String> {
    let out = Command::new("fc-match")
        .args(["-f", "%{family[0]}", "monospace"])
        .output()
        .ok()?;
    let family = String::from_utf8(out.stdout).ok()?.trim().to_owned();
    (!family.is_empty()).then_some(family)
}

pub fn load() -> Theme {
    let dir = current_dir();
    let read = |rel: &str| dir.as_ref().and_then(|d| std::fs::read_to_string(d.join(rel)).ok());
    Theme {
        name: read("theme.name").map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()),
        colors: read("theme/colors.toml").map(|s| parse_colors(&s)).unwrap_or_default(),
        mono_font: mono_font(),
    }
}

#[tauri::command]
pub fn get_theme() -> Theme {
    load()
}

/// Watches the Omarchy state directory and emits `theme-changed` with the new theme.
/// `omarchy theme set` rewrites many files at once, so events are debounced.
pub fn watch(app: AppHandle) {
    let Some(dir) = current_dir().filter(|d| d.exists()) else {
        return;
    };
    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel();
        let Ok(mut watcher) = notify::recommended_watcher(tx) else {
            return;
        };
        if watcher.watch(&dir, RecursiveMode::Recursive).is_err() {
            return;
        }
        let mut last = load();
        while rx.recv().is_ok() {
            while rx.recv_timeout(Duration::from_millis(250)).is_ok() {}
            let theme = load();
            if theme != last && !theme.colors.is_empty() {
                let _ = app.emit("theme-changed", &theme);
                last = theme;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_flat_colors_and_skips_non_strings() {
        let colors = parse_colors("# comment\nmode = \"dark\"\naccent = \"#87925a\"\nopacity = 0.9\n");
        assert_eq!(colors.get("accent").map(String::as_str), Some("#87925a"));
        assert_eq!(colors.get("mode").map(String::as_str), Some("dark"));
        assert!(!colors.contains_key("opacity"));
    }

    #[test]
    fn invalid_toml_gives_empty_map() {
        assert!(parse_colors("not = = toml").is_empty());
    }
}
