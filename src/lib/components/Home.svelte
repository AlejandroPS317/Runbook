<script lang="ts">
  import { onMount } from "svelte";
  import { openProject, pickFolder, takeInitialPath } from "../ipc/project";
  import type { Project } from "../ipc/types";

  interface Props {
    onOpen: (project: Project) => void;
  }

  let { onOpen }: Props = $props();
  let busy = $state(false);
  let error = $state<string | null>(null);

  onMount(() => {
    void takeInitialPath().then((path) => {
      if (path) void open(path);
    });
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
</script>

<main class="home">
  <h1>Runbook</h1>
  <p class="muted">Run package.json scripts without opening an IDE.</p>
  <button class="primary" onclick={chooseFolder} disabled={busy}>
    {busy ? "Opening…" : "Open folder…"}
  </button>
  {#if error}
    <p class="error">{error}</p>
  {/if}
</main>

<style>
  .home {
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 0.75rem;
    height: 100%;
    text-align: center;
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
</style>
