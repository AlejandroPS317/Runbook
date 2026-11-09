//! In-app updates (ADR 0007): check the release manifest, then download the update, stop
//! every script, install it and restart. Mirrors `src/lib/ipc/updates.ts`.

use std::sync::Mutex;

use runbook_process::DEFAULT_GRACE;
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::error::CommandError;
use crate::state::{AppState, lock};

/// The update found by the last check, so installing does not fetch the manifest again.
#[derive(Default)]
pub struct PendingUpdate(Mutex<Option<Update>>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    version: String,
    current_version: String,
    notes: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum InstallEvent {
    /// Whole percent downloaded; only sent when the server announces the download size.
    Progress { percent: u8 },
    /// The download is verified; scripts are being stopped and the update installed.
    Installing,
}

/// Fetches the release manifest. Resolves to `None` when this is already the newest version.
#[tauri::command]
pub async fn check_for_update(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
) -> Result<Option<UpdateInfo>, CommandError> {
    let update = app.updater()?.check().await?;
    let info = update.as_ref().map(|update| UpdateInfo {
        version: update.version.clone(),
        current_version: update.current_version.clone(),
        notes: update.body.clone(),
    });
    *lock(&pending.0) = update;
    Ok(info)
}

/// Scripts that installing an update would stop, so the user can confirm first.
#[tauri::command]
pub async fn running_scripts(state: State<'_, AppState>) -> Result<usize, CommandError> {
    Ok(state.sessions.running())
}

/// Downloads and verifies the pending update while scripts keep running, then stops every
/// script and its process tree, installs the update and restarts Runbook.
///
/// If installing fails (for example, the user cancels the password prompt of a `.deb`
/// update) the scripts are already stopped; the app keeps running on the old version.
#[tauri::command]
pub async fn install_update(
    app: AppHandle,
    on_event: Channel<InstallEvent>,
    pending: State<'_, PendingUpdate>,
) -> Result<(), CommandError> {
    let update = lock(&pending.0)
        .clone()
        .ok_or(CommandError::NoPendingUpdate)?;
    let mut progress = Progress::default();
    // A send fails only when the webview is gone; there is nobody left to tell.
    let bytes = update
        .download(
            |chunk, total| {
                if let Some(percent) = progress.advance(chunk, total) {
                    let _ = on_event.send(InstallEvent::Progress { percent });
                }
            },
            || {},
        )
        .await?;
    let _ = on_event.send(InstallEvent::Installing);

    // Stopping waits up to the grace period and a `.deb` install waits for the password
    // prompt, so both run on the blocking pool. Per platform, `Update::install`:
    // - Linux: rewrites the AppImage in place, or runs `dpkg -i` through `pkexec` for a
    //   `.deb` install (`rpm -U` for `.rpm`).
    // - macOS: replaces the `.app` bundle.
    // - Windows: launches the NSIS or MSI installer and exits the app; the installer
    //   relaunches Runbook, so the restart below is never reached.
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        handle.state::<AppState>().sessions.stop_all(DEFAULT_GRACE);
        update.install(bytes)
    })
    .await
    .map_err(|_| CommandError::UpdateInterrupted)??;
    app.restart()
}

/// Turns download chunks into whole percentages, so the frontend gets at most ~100 events
/// instead of one per network chunk.
#[derive(Default)]
struct Progress {
    downloaded: u64,
    percent: Option<u8>,
}

impl Progress {
    /// Adds a chunk and returns the new percentage when it changed.
    fn advance(&mut self, chunk: usize, total: Option<u64>) -> Option<u8> {
        self.downloaded = self.downloaded.saturating_add(chunk as u64);
        let total = total.filter(|&total| total > 0)?;
        let percent = u8::try_from(self.downloaded.min(total) * 100 / total).unwrap_or(100);
        if self.percent == Some(percent) {
            return None;
        }
        self.percent = Some(percent);
        Some(percent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_each_whole_percent_once() {
        let mut progress = Progress::default();
        assert_eq!(progress.advance(10, Some(1000)), Some(1));
        assert_eq!(progress.advance(5, Some(1000)), None, "still 1%");
        assert_eq!(progress.advance(5, Some(1000)), Some(2));
        assert_eq!(progress.advance(980, Some(1000)), Some(100));
    }

    #[test]
    fn unknown_or_zero_size_reports_nothing() {
        let mut progress = Progress::default();
        assert_eq!(progress.advance(500, None), None);
        assert_eq!(progress.advance(500, Some(0)), None);
    }

    #[test]
    fn never_exceeds_100_when_the_server_sends_more_than_announced() {
        let mut progress = Progress::default();
        assert_eq!(progress.advance(2000, Some(1000)), Some(100));
        assert_eq!(progress.advance(10, Some(1000)), None);
    }
}
