import type { ITheme } from "@xterm/xterm";

// xterm.js paints on a canvas, so it cannot read the CSS custom properties in app.css; the
// backgrounds here match `--bg` in each color scheme.
const dark: ITheme = {
  background: "#0c0d10",
  foreground: "#e4e5e9",
  cursor: "#34d399",
  cursorAccent: "#0c0d10",
  selectionBackground: "#2b3d36",
  black: "#1b1d22",
  red: "#f87171",
  green: "#34d399",
  yellow: "#fbbf24",
  blue: "#60a5fa",
  magenta: "#c084fc",
  cyan: "#22d3ee",
  white: "#d4d6db",
  brightBlack: "#6c6f77",
  brightRed: "#fca5a5",
  brightGreen: "#6ee7b7",
  brightYellow: "#fcd34d",
  brightBlue: "#93c5fd",
  brightMagenta: "#d8b4fe",
  brightCyan: "#67e8f9",
  brightWhite: "#f5f6f8",
};

// Darker hues than the dark palette: yellow, cyan and bright colors written for dark
// terminals must stay readable on white.
const light: ITheme = {
  background: "#ffffff",
  foreground: "#1f2328",
  cursor: "#059669",
  cursorAccent: "#ffffff",
  selectionBackground: "#cdeee0",
  black: "#1f2328",
  red: "#cf222e",
  green: "#047857",
  yellow: "#9a6700",
  blue: "#0969da",
  magenta: "#8250df",
  cyan: "#0e7490",
  white: "#6e7781",
  brightBlack: "#57606a",
  brightRed: "#a40e26",
  brightGreen: "#065f46",
  brightYellow: "#7d4e00",
  brightBlue: "#0550ae",
  brightMagenta: "#6639ba",
  brightCyan: "#155e75",
  brightWhite: "#8c959f",
};

const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");

export function currentTerminalTheme(): ITheme {
  return darkQuery.matches ? dark : light;
}

/** Calls `onChange` with the new theme whenever the system color scheme changes. */
export function watchTerminalTheme(onChange: (theme: ITheme) => void): () => void {
  const listener = () => onChange(currentTerminalTheme());
  darkQuery.addEventListener("change", listener);
  return () => darkQuery.removeEventListener("change", listener);
}
