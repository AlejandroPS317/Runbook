// Update flow shared by the update banner and the home screen (ADR 0007).

import type { InstallEvent, UpdateInfo } from "./ipc/types";
import { checkForUpdate, installUpdate, runningScripts } from "./ipc/updates";

export type UpdateState =
  | { kind: "idle" }
  | { kind: "up-to-date" }
  | { kind: "available"; info: UpdateInfo }
  | { kind: "confirm"; info: UpdateInfo; running: number }
  | { kind: "downloading"; info: UpdateInfo; percent: number | null }
  | { kind: "installing"; info: UpdateInfo }
  | { kind: "failed"; message: string };

class Updates {
  state = $state<UpdateState>({ kind: "idle" });
  checking = $state(false);

  /** True while an update downloads or installs; checking again would race it. */
  get installing(): boolean {
    return this.state.kind === "downloading" || this.state.kind === "installing";
  }

  /**
   * Looks for a newer version. A `manual` check also reports "up to date" and errors; the
   * startup check stays silent unless an update exists.
   */
  async check(manual: boolean): Promise<void> {
    if (this.checking || this.installing) return;
    this.checking = true;
    try {
      const info = await checkForUpdate();
      if (info) this.state = { kind: "available", info };
      else if (manual) this.state = { kind: "up-to-date" };
    } catch (err) {
      if (manual) this.state = { kind: "failed", message: String(err) };
      else console.warn("update check failed:", err);
    } finally {
      this.checking = false;
    }
  }

  /** Installs right away when no script is running, otherwise asks first. */
  async requestInstall(info: UpdateInfo): Promise<void> {
    const running = await runningScripts();
    if (running > 0) this.state = { kind: "confirm", info, running };
    else await this.install(info);
  }

  /** Stops every script, installs and restarts. Only returns if the install failed. */
  async install(info: UpdateInfo): Promise<void> {
    this.state = { kind: "downloading", info, percent: null };
    try {
      await installUpdate((event) => this.onInstallEvent(info, event));
    } catch (err) {
      this.state = { kind: "failed", message: String(err) };
    }
  }

  /** Backs out of the "stop running scripts?" question; the update stays offered. */
  cancelInstall(info: UpdateInfo): void {
    this.state = { kind: "available", info };
  }

  dismiss(): void {
    this.state = { kind: "idle" };
  }

  private onInstallEvent(info: UpdateInfo, event: InstallEvent): void {
    switch (event.kind) {
      case "progress":
        this.state = { kind: "downloading", info, percent: event.percent };
        return;
      case "installing":
        this.state = { kind: "installing", info };
        return;
      default: {
        const unreachable: never = event;
        return unreachable;
      }
    }
  }
}

export const updates = new Updates();
