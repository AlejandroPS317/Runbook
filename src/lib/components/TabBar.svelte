<script module lang="ts">
  import type { TerminalStatus } from "../terminal/status";

  export interface TabInfo {
    key: number;
    label: string;
    status: TerminalStatus;
  }
</script>

<script lang="ts">
  import type { Snippet } from "svelte";
  import { shortcutLabel } from "../keyboard";
  import { statusText, statusTone } from "../terminal/status";

  interface Props {
    tabs: TabInfo[];
    activeKey: number | null;
    onSelect: (key: number) => void;
    onClose: (key: number) => void;
    onNewShell: () => void;
    /** Controls for the active tab, pinned to the right of the strip. */
    actions?: Snippet;
  }

  let { tabs, activeKey, onSelect, onClose, onNewShell, actions }: Props = $props();
  const elements: Record<number, HTMLDivElement> = {};

  $effect(() => {
    if (activeKey !== null) elements[activeKey]?.scrollIntoView({ block: "nearest", inline: "nearest" });
  });
</script>

<div class="bar">
  <div class="tabs" role="tablist">
    {#each tabs as tab (tab.key)}
      <div class="tab" class:active={tab.key === activeKey} bind:this={elements[tab.key]}>
        <button
          class="select"
          role="tab"
          aria-selected={tab.key === activeKey}
          title={`${tab.label} — ${statusText(tab.status)}`}
          onclick={() => onSelect(tab.key)}
        >
          <span class="dot {statusTone(tab.status)}"></span>
          <span class="label">{tab.label}</span>
        </button>
        <button
          class="close"
          aria-label={`Close ${tab.label} and stop its script`}
          title="Close and stop"
          onclick={() => onClose(tab.key)}>×</button
        >
      </div>
    {/each}
    <button
      class="new"
      aria-label="Open a terminal in the project folder"
      title={`Open a terminal in the project folder (${shortcutLabel("open-shell")})`}
      onclick={onNewShell}>+</button
    >
  </div>
  {#if actions}
    <div class="actions">{@render actions()}</div>
  {/if}
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
    padding: 0 0.5rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }

  .tabs {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    flex: 1;
    min-width: 0;
    padding: 0.4rem 0;
    overflow-x: auto;
  }

  /* macOS WKWebView draws an overlay scrollbar that widens on hover and paints over the
     tab labels. A styled ::-webkit-scrollbar is a classic scrollbar with its own fixed
     strip below the tabs in WKWebView (macOS), WebKitGTK (Linux) and WebView2 (Windows).
     Keep `scrollbar-width` unset: WebView2 ignores ::-webkit-scrollbar when it is set. */
  .tabs::-webkit-scrollbar {
    height: 6px;
  }

  .tabs::-webkit-scrollbar-track {
    background: transparent;
  }

  .tabs::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 3px;
  }

  .tabs::-webkit-scrollbar-thumb:hover {
    background: var(--text-muted);
  }

  .tab {
    display: flex;
    align-items: center;
    flex: 0 0 auto;
    max-width: 16rem;
    border: 1px solid transparent;
    border-radius: var(--radius);
  }

  .tab:hover {
    background: var(--hover);
  }

  .tab.active {
    background: var(--raised);
    border-color: var(--border-strong);
  }

  button {
    border: none;
    background: transparent;
  }

  button:hover:not(:disabled) {
    background: transparent;
  }

  .select {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    min-width: 0;
    padding: 0.3rem 0.2rem 0.3rem 0.6rem;
  }

  .label {
    font-family: var(--mono);
    font-size: 0.8rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tab:not(.active) .label {
    color: var(--text-muted);
  }

  .dot {
    flex: 0 0 auto;
    width: 0.45rem;
    height: 0.45rem;
    border-radius: 50%;
    background: var(--text-faint);
  }

  .dot.active {
    background: var(--accent);
  }

  .dot.error {
    background: var(--danger);
  }

  .close,
  .new {
    padding: 0.15rem 0.45rem;
    color: var(--text-faint);
    font-size: 1rem;
    line-height: 1;
  }

  .close:hover:not(:disabled),
  .new:hover:not(:disabled) {
    color: var(--text);
  }

  .new {
    flex: 0 0 auto;
    border-radius: var(--radius);
  }

  .actions {
    display: flex;
    align-items: center;
    flex: 0 0 auto;
    gap: 0.35rem;
  }
</style>
