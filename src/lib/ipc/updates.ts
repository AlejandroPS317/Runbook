import { getVersion } from "@tauri-apps/api/app";
import { Channel, invoke } from "@tauri-apps/api/core";
import type { InstallEvent, UpdateInfo } from "./types";

/** The version of the running app, e.g. `0.1.1`. */
export function appVersion(): Promise<string> {
  return getVersion();
}

/** Fetches the release manifest. Resolves to `null` when Runbook is up to date. */
export function checkForUpdate(): Promise<UpdateInfo | null> {
  return invoke<UpdateInfo | null>("check_for_update");
}

/** Scripts that installing an update would stop. */
export function runningScripts(): Promise<number> {
  return invoke<number>("running_scripts");
}

/**
 * Downloads the update found by the last check, stops every script, installs the update
 * and restarts Runbook. Only resolves or rejects if the install fails; on success the app
 * restarts first.
 */
export function installUpdate(onEvent: (event: InstallEvent) => void): Promise<void> {
  const channel = new Channel<InstallEvent>();
  channel.onmessage = onEvent;
  return invoke("install_update", { onEvent: channel });
}
