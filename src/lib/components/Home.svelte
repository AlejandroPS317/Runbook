<script lang="ts">
  import { onMount } from "svelte";
  import {
    forgetRecentProject,
    openProject,
    pickFolder,
    recentProjects,
    takeInitialPath,
  } from "../ipc/project";
  import type { Project, RecentProject } from "../ipc/types";
  import { appVersion } from "../ipc/updates";
  import { shortcutFor, shortcutLabel } from "../keyboard";
  import { checkForUpdatesAtStartup, setCheckForUpdatesAtStartup } from "../settings";
  import { updates } from "../updates.svelte";
  import iconUrl from "../../../assets/icon.svg";

  interface Props {
    onOpen: (project: Project) => void;
  }

  let { onOpen }: Props = $props();
  let busy = $state(false);
  let error = $state<string | null>(null);
  let recent = $state<RecentProject[]>([]);
  let version = $state("");
  let checkAtStartup = $state(checkForUpdatesAtStartup());

  onMount(() => {
    void takeInitialPath().then((path) => {
      if (path) void open(path);
    });
    void recentProjects().then((list) => (recent = list));
    void appVersion().then((v) => (version = v));
  });

  async function chooseFolder() {
    error = null;
    const path = await pickFolder();
    if (path) await open(path);
  }

  async function open(path: string) {
    busy = true;
    try {
      onOpen(await openProject(path));
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }

  async function forget(path: string) {
    await forgetRecentProject(path);
    recent = recent.filter((p) => p.path !== path);
  }

  function onKeydown(event: KeyboardEvent) {
    if (shortcutFor(event) === "open-folder") {
      event.preventDefault();
      void chooseFolder();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<main class="home">
  <img class="logo" src={iconUrl} alt="" width="72" height="75" />
  <h1>Runbook</h1>
  <p class="muted">Run package.json scripts without opening an IDE.</p>
  <button
    class="primary"
    onclick={chooseFolder}
    disabled={busy}
    title={`Open folder (${shortcutLabel("open-folder")})`}
  >
    {busy ? "Opening…" : "Open folder…"}
  </button>
  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if recent.length > 0}
    <section class="recent">
      <h2>Recent</h2>
      <ul>
        {#each recent as project (project.path)}
          <li>
            <button class="open" onclick={() => open(project.path)} disabled={busy}>
              <span class="name">{project.name}</span>
              <span class="path">{project.path}</span>
            </button>
            <button
              class="forget"
              aria-label={`Remove ${project.name} from recent projects`}
              title="Remove from recent"
              onclick={() => forget(project.path)}>×</button
            >
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  <footer class="muted">
    <span>Runbook {version}</span>
    <button
      class="link"
      onclick={() => updates.check(true)}
      disabled={updates.checking || updates.installing}
    >
      {updates.checking ? "Checking…" : "Check for updates"}
    </button>
    <label>
      <input
        type="checkbox"
        bind:checked={checkAtStartup}
        onchange={() => setCheckForUpdatesAtStartup(checkAtStartup)}
      />
      Check at startup
    </label>
  </footer>
</main>

<style>
  .home {
    position: relative;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 0.75rem;
    height: 100%;
    text-align: center;
  }

  footer {
    position: absolute;
    bottom: 0.75rem;
    left: 0;
    right: 0;
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 1rem;
    font-size: 0.8rem;
  }

  .link {
    padding: 0;
    border: none;
    background: transparent;
    color: inherit;
    text-decoration: underline;
  }

  .link:hover:not(:disabled) {
    background: transparent;
    color: var(--text);
  }

  label {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .logo {
    margin-bottom: 0.25rem;
  }

  h1 {
    margin: 0;
    font-size: 1.75rem;
    font-weight: 600;
  }

  p {
    margin: 0;
  }

  .muted {
    color: var(--text-muted);
  }

  .primary {
    margin-top: 0.5rem;
  }

  .error {
    color: var(--danger);
    max-width: 40rem;
  }

  .recent {
    width: min(32rem, 90vw);
    margin-top: 1.5rem;
    text-align: left;
  }

  h2 {
    margin: 0 0 0.4rem;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
  }

  li {
    display: flex;
    align-items: center;
  }

  li + li {
    border-top: 1px solid var(--border);
  }

  .open {
    display: grid;
    flex: 1;
    min-width: 0;
    gap: 0.1rem;
    padding: 0.5rem 0.75rem;
    text-align: left;
    border: none;
    border-radius: 0;
    background: transparent;
  }

  .name {
    font-weight: 600;
  }

  .path {
    color: var(--text-muted);
    font-size: 0.8rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .forget {
    padding: 0.2rem 0.6rem;
    margin-right: 0.25rem;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 1rem;
  }

  .forget:hover {
    color: var(--text);
  }
</style>
