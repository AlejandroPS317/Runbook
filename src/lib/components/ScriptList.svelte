<script lang="ts">
  import { SvelteSet } from "svelte/reactivity";
  import type { Package, PackageRole } from "../ipc/types";
  import type { ScriptState } from "../terminal/status";
  import ScriptRow from "./ScriptRow.svelte";

  interface Props {
    packages: Package[];
    /** State of the script's tab, or `null` when it has none. */
    stateOf: (pkg: Package, script: string) => ScriptState | null;
    /** Shows every package expanded, e.g. while filtering, so no match is hidden. */
    expandAll: boolean;
    onRun: (pkg: Package, script: string) => void;
    /** Opens a shell in the package folder; with `script`, its command is typed at the prompt. */
    onShell: (pkg: Package, script: string | null) => void;
  }

  let { packages, stateOf, expandAll, onRun, onShell }: Props = $props();

  const declared = $derived(packages.filter((p) => p.role !== "unlisted"));
  const unlisted = $derived(packages.filter((p) => p.role === "unlisted"));
  const collapsed = new SvelteSet<string>();

  function title(pkg: Package): string {
    return pkg.name ?? (pkg.relativeDir || "(root)");
  }

  /** Splits `@scope/name` after the slash, so a wrapped name breaks there before anywhere else. */
  function nameParts(name: string): string[] {
    const slash = name.indexOf("/");
    return slash === -1 ? [name] : [name.slice(0, slash + 1), name.slice(slash + 1)];
  }

  function badgeTitle(pkg: Package): string {
    return pkg.role === "root" ? roleLabel(pkg.role) : `${roleLabel(pkg.role)}: ${pkg.relativeDir}`;
  }

  function roleLabel(role: PackageRole): string {
    switch (role) {
      case "root":
        return "root";
      case "member":
        return "workspace";
      case "unlisted":
        return "outside workspaces";
      default: {
        const unreachable: never = role;
        return unreachable;
      }
    }
  }

  function toggle(pkg: Package) {
    if (collapsed.has(pkg.relativeDir)) collapsed.delete(pkg.relativeDir);
    else collapsed.add(pkg.relativeDir);
  }
</script>

{#snippet group(list: Package[])}
  {#each list as pkg (pkg.relativeDir)}
    {@const open = expandAll || !collapsed.has(pkg.relativeDir)}
    <section class="package">
      <header>
        <button
          class="toggle"
          aria-expanded={open}
          title={open ? "Collapse" : "Expand"}
          onclick={() => toggle(pkg)}
        >
          <span class="chevron" class:open></span>
          <span class="name"
            >{#each nameParts(title(pkg)) as part, i (i)}{#if i > 0}<wbr />{/if}{part}{/each}</span
          >
        </button>
        <span class="badge" class:root={pkg.role === "root"} title={badgeTitle(pkg)}>
          {pkg.role === "root" ? "ROOT" : pkg.relativeDir}
        </span>
        <button
          class="shell"
          title={`Open a terminal in ${pkg.relativeDir || "the project folder"}`}
          aria-label={`Open a terminal in ${title(pkg)}`}
          onclick={() => onShell(pkg, null)}>&gt;_</button
        >
      </header>
      {#if open}
        {#if pkg.scripts.length === 0}
          <p class="empty">No scripts</p>
        {:else}
          <ul>
            {#each pkg.scripts as script (script.name)}
              <ScriptRow
                {script}
                owner={title(pkg)}
                state={stateOf(pkg, script.name)}
                onRun={() => onRun(pkg, script.name)}
                onShell={() => onShell(pkg, script.name)}
              />
            {/each}
          </ul>
        {/if}
      {/if}
    </section>
  {/each}
{/snippet}

<nav class="scripts">
  {@render group(declared)}
  {#if unlisted.length > 0}
    <h2>Outside workspaces</h2>
    {@render group(unlisted)}
  {/if}
</nav>

<style>
  .scripts {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    overflow-y: auto;
    padding: 0.25rem 0.6rem 0.75rem;
  }

  h2 {
    margin: 0.5rem 0.25rem 0;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }

  .package {
    flex: 0 0 auto;
    padding: 0.2rem;
    background: var(--raised);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
  }

  header {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    min-width: 0;
    padding-right: 0.15rem;
  }

  button {
    border: none;
    background: transparent;
  }

  /* Sized by its content, so the name claims its width before the path badge gets any. */
  .toggle {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    flex: 0 1 auto;
    min-width: 0;
    padding: 0.4rem 0.4rem;
    text-align: left;
  }

  .toggle:hover:not(:disabled) {
    background: transparent;
  }

  .chevron {
    flex: 0 0 auto;
    width: 0.35rem;
    height: 0.35rem;
    border-right: 1.5px solid var(--text-muted);
    border-bottom: 1.5px solid var(--text-muted);
    transform: rotate(-45deg);
    transition: transform 120ms ease;
  }

  .chevron.open {
    transform: rotate(45deg);
  }

  /* Wraps instead of truncating: packages sharing a scope differ only in the suffix. */
  .name {
    min-width: 0;
    font-family: var(--mono);
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  /*
   * Zero basis: it only fills the width the package name leaves, so it truncates (full path in
   * its title) and the name wraps only when it does not fit on its own.
   */
  .badge {
    flex: 1 1 0;
    min-width: 0;
    font-family: var(--mono);
    font-size: 0.7rem;
    color: var(--text-faint);
    text-align: right;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .badge.root {
    flex: 0 0 auto;
    margin-left: auto;
    padding: 0.05rem 0.35rem;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    border: 1px solid var(--border);
    border-radius: 4px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0 0 0.15rem;
  }

  /* Shown on hover or keyboard focus, so the list stays as quiet as a plain script list. */
  .shell {
    flex: 0 0 auto;
    padding: 0.1rem 0.35rem;
    margin-right: 0.15rem;
    font-family: var(--mono);
    font-size: 0.7rem;
    color: var(--text-muted);
    opacity: 0;
  }

  header:hover .shell,
  .shell:focus-visible {
    opacity: 1;
  }

  .shell:hover:not(:disabled) {
    color: var(--text);
    background: var(--hover);
  }

  .empty {
    margin: 0;
    padding: 0.1rem 1.1rem 0.4rem;
    color: var(--text-muted);
    font-size: 0.85rem;
  }
</style>
