<script lang="ts">
  interface Props {
    message: string;
    onDismiss: () => void;
  }

  let { message, onDismiss }: Props = $props();
</script>

<p class="error-line" role="alert">
  <span>{message}</span>
  <button aria-label="Dismiss" title="Dismiss" onclick={onDismiss}>×</button>
</p>

<style>
  .error-line {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    margin: 0.6rem 0.6rem 0;
    padding: 0.4rem 0.6rem;
    border-radius: var(--radius);
    background: var(--danger-soft);
    color: var(--danger);
    font-size: 0.85rem;
  }

  span {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  button {
    padding: 0 0.3rem;
    border: none;
    background: transparent;
    color: var(--text-muted);
    line-height: 1;
  }
</style>
