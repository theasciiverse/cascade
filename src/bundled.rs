//! Songs that come with the game.
//!
//! A `songs` folder beside the program holds ready-made song folders, each
//! as `cascade add` leaves one (`metadata.json`, the audio and the four
//! beatmaps). The first time the game sees one, it is copied into the
//! player's library, where it plays, scores and can be renamed or deleted
//! like any song the player added. `bundled` in the state folder lists the
//! songs already offered, so a deleted one stays deleted.

use std::path::{Path, PathBuf};

use crate::config::{Config, copy_tree};

/// The `songs` folder beside the program, when there is one.
pub fn dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe = exe.canonicalize().unwrap_or(exe);
    let d = exe.parent()?.join("songs");
    d.is_dir().then_some(d)
}

/// Copy any song not offered before into the library.
pub fn seed() {
    let Some(from) = dir() else { return };
    seed_from(&from, &Config::cascade_dir());
}

pub fn seed_from(from: &Path, state: &Path) -> Vec<String> {
    let list = state.join("bundled");
    let offered: Vec<String> = std::fs::read_to_string(&list)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect();
    let mut entries: Vec<PathBuf> = match std::fs::read_dir(from) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.join("metadata.json").is_file())
            .collect(),
        Err(_) => return Vec::new(),
    };
    entries.sort();
    let mut added = Vec::new();
    for src in entries {
        let Some(slug) = src.file_name().and_then(|n| n.to_str()).map(str::to_string) else {
            continue;
        };
        if offered.contains(&slug) {
            continue;
        }
        let dest = state.join("songs").join(&slug);
        if !dest.exists() {
            if let Err(e) = copy_tree(&src, &dest) {
                log::warn!("could not add bundled song {slug}: {e}");
                let _ = std::fs::remove_dir_all(&dest);
                continue;
            }
            log::info!("added bundled song {slug}");
        }
        added.push(slug);
    }
    if !added.is_empty() {
        let mut text = offered.join("\n");
        for s in &added {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(s);
        }
        text.push('\n');
        if let Err(e) = std::fs::create_dir_all(state).and_then(|_| std::fs::write(&list, text)) {
            log::warn!("could not write {}: {e}", list.display());
        }
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn songs_are_offered_once() {
        let tmp = tempfile::tempdir().unwrap();
        let from = tmp.path().join("bundle");
        let state = tmp.path().join("state");
        for slug in ["one", "two"] {
            std::fs::create_dir_all(from.join(slug)).unwrap();
            std::fs::write(from.join(slug).join("metadata.json"), "{}").unwrap();
            std::fs::write(from.join(slug).join("audio.ogg"), "x").unwrap();
        }
        std::fs::create_dir_all(from.join("not-a-song")).unwrap();
        assert_eq!(seed_from(&from, &state), vec!["one", "two"]);
        assert!(state.join("songs/one/audio.ogg").is_file());
        // a deleted song is not brought back
        std::fs::remove_dir_all(state.join("songs/one")).unwrap();
        assert!(seed_from(&from, &state).is_empty());
        assert!(!state.join("songs/one").exists());
        // a new one in a later release is
        std::fs::create_dir_all(from.join("three")).unwrap();
        std::fs::write(from.join("three").join("metadata.json"), "{}").unwrap();
        assert_eq!(seed_from(&from, &state), vec!["three"]);
    }
}
