// User preferences, kept in the webview's local storage (inside the app data folder).

const CHECK_AT_STARTUP = "runbook.checkForUpdatesAtStartup";

/** Whether Runbook looks for a new version when it starts. On unless the user opted out. */
export function checkForUpdatesAtStartup(): boolean {
  return localStorage.getItem(CHECK_AT_STARTUP) !== "false";
}

export function setCheckForUpdatesAtStartup(enabled: boolean): void {
  localStorage.setItem(CHECK_AT_STARTUP, String(enabled));
}
