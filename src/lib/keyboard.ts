// Keyboard shortcuts. Shortcuts that would collide with common terminal keys (Ctrl+W,
// Ctrl+K, Ctrl+C...) use Shift, so the shell inside a terminal tab keeps them.

export type Action =
  | "open-folder"
  | "focus-filter"
  | "next-tab"
  | "previous-tab"
  | "close-tab"
  | "restart-tab"
  | "stop-tab"
  | "open-shell";

export const isMac = navigator.userAgent.includes("Mac");

/** The action bound to `event`, or `null` when the key is not a shortcut. */
export function shortcutFor(event: KeyboardEvent): Action | null {
  const mod = isMac ? event.metaKey : event.ctrlKey;
  const { shiftKey: shift, altKey: alt, code } = event;
  if (alt) return null;

  // Tab switching uses Ctrl on every platform, like browsers and editors.
  if (event.ctrlKey && (code === "Tab" || code === "PageDown" || code === "PageUp")) {
    if (code === "PageDown") return "next-tab";
    if (code === "PageUp") return "previous-tab";
    return shift ? "previous-tab" : "next-tab";
  }
  if (!mod) return null;
  if (!shift && code === "KeyO") return "open-folder";
  if (!shift) return null;
  switch (code) {
    case "KeyF":
      return "focus-filter";
    case "KeyW":
      return "close-tab";
    case "KeyR":
      return "restart-tab";
    // K for "kill": Ctrl+Shift+S is grabbed by some Linux desktops before the app sees it.
    case "KeyK":
      return "stop-tab";
    // T for "terminal", as in terminal emulators' "new tab".
    case "KeyT":
      return "open-shell";
    default:
      return null;
  }
}

/**
 * The project (0-based position in the project bar) selected by Alt+1…9, or ⌘1…9 on macOS;
 * `null` when the key is not a project shortcut. Matches `event.code`, so keyboard layouts
 * do not matter. Any other modifier cancels it: Windows reports AltGr as Ctrl+Alt, and
 * AltGr+digit types characters such as `@` on Spanish layouts.
 */
export function projectIndexFor(event: KeyboardEvent): number | null {
  const { altKey: alt, ctrlKey: ctrl, metaKey: meta, shiftKey: shift, code } = event;
  const pressed = isMac ? meta && !alt && !ctrl && !shift : alt && !ctrl && !meta && !shift;
  if (!pressed) return null;
  const match = /^Digit([1-9])$/.exec(code);
  return match ? Number(match[1]) - 1 : null;
}

/** Key combination that selects the project at `index` (0-based), e.g. "Alt+1" or "⌘1". */
export function projectShortcutLabel(index: number): string | null {
  if (index > 8) return null;
  return `${isMac ? "⌘" : "Alt+"}${index + 1}`;
}

/** Human-readable key combination for tooltips, e.g. "Ctrl+Shift+R" or "⌘⇧R". */
export function shortcutLabel(action: Action): string {
  const mod = isMac ? "⌘" : "Ctrl+";
  const shift = isMac ? "⇧" : "Shift+";
  switch (action) {
    case "open-folder":
      return `${mod}O`;
    case "focus-filter":
      return `${mod}${shift}F`;
    case "next-tab":
      return isMac ? "⌃Tab" : "Ctrl+Tab";
    case "previous-tab":
      return isMac ? "⌃⇧Tab" : "Ctrl+Shift+Tab";
    case "close-tab":
      return `${mod}${shift}W`;
    case "restart-tab":
      return `${mod}${shift}R`;
    case "stop-tab":
      return `${mod}${shift}K`;
    case "open-shell":
      return `${mod}${shift}T`;
    default: {
      const unreachable: never = action;
      return unreachable;
    }
  }
}
