//! Recently opened projects, most recent first, stored as JSON in the app data folder.
//! Mirrors `RecentProject` in `src/lib/ipc/types.ts`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::CommandError;
use crate::state::lock;

/// The list is bounded so the file and the home screen stay small.
pub const MAX_RECENT: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub path: String,
    pub name: String,
    /// Seconds since the Unix epoch.
    pub opened_at: u64,
}

pub struct RecentProjects {
    file: PathBuf,
    list: Mutex<Vec<RecentProject>>,
}

impl RecentProjects {
    /// Loads the list from `file`. A missing or unreadable file starts an empty list: losing
    /// the recent list must never prevent the app from starting.
    pub fn load(file: PathBuf) -> Self {
        let list = fs::read_to_string(&file)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self {
            file,
            list: Mutex::new(list),
        }
    }

    /// Recent projects whose folder still exists, most recent first.
    pub fn list(&self) -> Vec<RecentProject> {
        lock(&self.list)
            .iter()
            .filter(|p| Path::new(&p.path).is_dir())
            .cloned()
            .collect()
    }

    /// Moves `path` to the front (adding it if new) and saves the list.
    pub fn record(&self, path: &Path, name: &str) -> io::Result<()> {
        let entry = RecentProject {
            path: path.display().to_string(),
            name: name.to_owned(),
            opened_at: now(),
        };
        let mut list = lock(&self.list);
        push_front(&mut list, entry);
        save(&self.file, &list)
    }

    /// Removes `path` from the list and saves it.
    pub fn forget(&self, path: &str) -> io::Result<()> {
        let mut list = lock(&self.list);
        list.retain(|p| p.path != path);
        save(&self.file, &list)
    }
}

/// Puts `entry` first, drops older entries for the same path and keeps at most
/// `MAX_RECENT` entries.
fn push_front(list: &mut Vec<RecentProject>, entry: RecentProject) {
    list.retain(|p| p.path != entry.path);
    list.insert(0, entry);
    list.truncate(MAX_RECENT);
}

/// Writes through a temporary file and renames it, so a crash never leaves a half-written
/// list behind.
fn save(file: &Path, list: &[RecentProject]) -> io::Result<()> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(list).map_err(io::Error::other)?;
    let temporary = file.with_extension("json.tmp");
    fs::write(&temporary, text)?;
    fs::rename(&temporary, file)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[tauri::command]
pub async fn recent_projects(
    recent: State<'_, RecentProjects>,
) -> Result<Vec<RecentProject>, CommandError> {
    Ok(recent.list())
}

#[tauri::command]
pub async fn forget_recent_project(
    path: String,
    recent: State<'_, RecentProjects>,
) -> Result<(), CommandError> {
    recent.forget(&path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(path: &str) -> RecentProject {
        RecentProject {
            path: path.into(),
            name: path.into(),
            opened_at: 0,
        }
    }

    fn paths(list: &[RecentProject]) -> Vec<&str> {
        list.iter().map(|p| p.path.as_str()).collect()
    }

    #[test]
    fn most_recent_first_without_duplicates() {
        let mut list = vec![project("/a"), project("/b")];
        push_front(&mut list, project("/b"));
        assert_eq!(paths(&list), ["/b", "/a"]);
        push_front(&mut list, project("/c"));
        assert_eq!(paths(&list), ["/c", "/b", "/a"]);
    }

    #[test]
    fn keeps_at_most_max_recent() {
        let mut list = Vec::new();
        for i in 0..MAX_RECENT + 5 {
            push_front(&mut list, project(&format!("/p{i}")));
        }
        assert_eq!(list.len(), MAX_RECENT);
        assert_eq!(list[0].path, format!("/p{}", MAX_RECENT + 4));
    }

    #[test]
    fn records_and_reloads_and_hides_missing_folders() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("state").join("recent.json");
        let existing = tmp.path().join("project");
        fs::create_dir(&existing).unwrap();

        let recent = RecentProjects::load(file.clone());
        recent.record(&existing, "project").unwrap();
        recent
            .record(&tmp.path().join("deleted"), "deleted")
            .unwrap();

        let reloaded = RecentProjects::load(file);
        let listed = reloaded.list();
        assert_eq!(listed.len(), 1, "the missing folder is hidden");
        assert_eq!(listed[0].name, "project");
    }

    #[test]
    fn forget_removes_the_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let recent = RecentProjects::load(tmp.path().join("recent.json"));
        recent.record(tmp.path(), "tmp").unwrap();
        recent.forget(&tmp.path().display().to_string()).unwrap();
        assert!(recent.list().is_empty());
    }

    #[test]
    fn corrupt_file_starts_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("recent.json");
        fs::write(&file, "{ not json").unwrap();
        assert!(RecentProjects::load(file).list().is_empty());
    }
}
