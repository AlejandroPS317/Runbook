<script module lang="ts">
  import type { TerminalStatus } from "../terminal/status";

  export interface TabInfo {
    key: number;
    label: string;
    status: TerminalStatus;
  }
</script>

<script lang="ts">
  import { statusText, statusTone } from "../terminal/status";

  interface Props {
    tabs: TabInfo[];
    activeKey: number | null;
    onSelect: (key: number) => void;
    onClose: (key: number) => void;
  }

  let { tabs, activeKey, onSelect, onClose }: Props = $props();
</script>

<div class="tabs" role="tablist">
  {#each tabs as tab (tab.key)}
    <div class="tab" class:active={tab.key === activeKey}>
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
</div>

<style>
  .tabs {
    display: flex;
    overflow-x: auto;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    scrollbar-width: thin;
  }

  .tab {
    display: flex;
    align-items: center;
    flex: 0 0 auto;
    max-width: 16rem;
    border-right: 1px solid var(--border);
  }

  .tab.active {
    background: var(--surface);
    box-shadow: inset 0 -2px 0 var(--accent);
  }

  button {
    border: none;
    border-radius: 0;
    background: transparent;
  }

  .select {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    min-width: 0;
    padding: 0.4rem 0.25rem 0.4rem 0.75rem;
    font-size: 0.85rem;
  }

  .label {
    font-family: var(--mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tab:not(.active) .label {
    color: var(--text-muted);
  }

  .dot {
    flex: 0 0 auto;
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    background: var(--text-muted);
  }

  .dot.active {
    background: var(--accent);
  }

  .dot.error {
    background: var(--danger);
  }

  .close {
    padding: 0.2rem 0.5rem;
    color: var(--text-muted);
    font-size: 1rem;
    line-height: 1;
  }

  .close:hover {
    color: var(--text);
  }
</style>
