<script lang="ts">
  import type { Package, PackageRole } from "../ipc/types";

  interface Props {
    packages: Package[];
    onRun: (pkg: Package, script: string) => void;
  }

  let { packages, onRun }: Props = $props();

  const declared = $derived(packages.filter((p) => p.role !== "unlisted"));
  const unlisted = $derived(packages.filter((p) => p.role === "unlisted"));

  function title(pkg: Package): string {
    return pkg.name ?? (pkg.relativeDir || "(root)");
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
</script>

{#snippet group(list: Package[])}
  {#each list as pkg (pkg.relativeDir)}
    <section class="package">
      <header>
        <span class="name">{title(pkg)}</span>
        <span class="dir" title={roleLabel(pkg.role)}>{pkg.relativeDir || "."}</span>
      </header>
      {#if pkg.scripts.length === 0}
        <p class="empty">No scripts</p>
      {:else}
        <ul>
          {#each pkg.scripts as script (script.name)}
            <li>
              <button
                class="run"
                title={`Run: ${script.command}`}
                aria-label={`Run ${script.name} in ${title(pkg)}`}
                onclick={() => onRun(pkg, script.name)}>▶</button
              >
              <span class="script" title={script.command}>{script.name}</span>
            </li>
          {/each}
        </ul>
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
    overflow-y: auto;
    padding: 0.5rem 0;
  }

  h2 {
    margin: 1rem 0.75rem 0.25rem;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }

  .package {
    padding: 0.25rem 0;
  }

  header {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    padding: 0.25rem 0.75rem;
    min-width: 0;
  }

  .name {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dir {
    color: var(--text-muted);
    font-size: 0.8rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.1rem 0.75rem 0.1rem 1rem;
  }

  li:hover {
    background: var(--hover);
  }

  .run {
    padding: 0.1rem 0.4rem;
    font-size: 0.7rem;
    color: var(--accent);
    background: transparent;
    border-color: transparent;
  }

  .run:hover {
    border-color: var(--border);
  }

  .script {
    font-family: var(--mono);
    font-size: 0.85rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .empty {
    margin: 0;
    padding: 0.1rem 1rem;
    color: var(--text-muted);
    font-size: 0.85rem;
  }
</style>
