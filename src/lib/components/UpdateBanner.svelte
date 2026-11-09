<script lang="ts">
  import { updates } from "../updates.svelte";

  const current = $derived(updates.state);
</script>

{#if current.kind !== "idle"}
  <div class="banner" class:failed={current.kind === "failed"} role="status" aria-live="polite">
    {#if current.kind === "available"}
      {@const info = current.info}
      <p>
        <strong>Runbook {info.version}</strong> is available. You have {info.currentVersion}.
      </p>
      <button onclick={() => updates.dismiss()}>Later</button>
      <button class="primary" onclick={() => updates.requestInstall(info)}>Update and restart</button>
    {:else if current.kind === "confirm"}
      {@const info = current.info}
      <p>
        Updating stops {current.running} running script{current.running === 1 ? "" : "s"} and
        restarts Runbook.
      </p>
      <button onclick={() => updates.cancelInstall(info)}>Cancel</button>
      <button class="primary" onclick={() => updates.install(info)}>Stop scripts and update</button>
    {:else if current.kind === "downloading"}
      <p>
        Downloading Runbook {current.info.version}…{current.percent === null
          ? ""
          : ` ${current.percent}%`}
      </p>
      <progress max="100" value={current.percent}></progress>
    {:else if current.kind === "installing"}
      <p>
        Installing Runbook {current.info.version}. Runbook restarts when it is done; your
        system may ask for your password.
      </p>
    {:else if current.kind === "up-to-date"}
      <p>Runbook is up to date.</p>
      <button onclick={() => updates.dismiss()}>OK</button>
    {:else if current.kind === "failed"}
      <p>{current.message}</p>
      <button onclick={() => updates.dismiss()}>Dismiss</button>
    {/if}
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.75rem;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
    font-size: 0.9rem;
  }

  .banner p {
    flex: 1;
    margin: 0;
  }

  .failed p {
    color: var(--danger);
  }

  progress {
    width: 10rem;
    accent-color: var(--accent);
  }
</style>
