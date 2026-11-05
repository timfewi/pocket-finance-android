<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import '../mobile-first.css';
  let { children } = $props();

  onMount(() => {
    let frame = 0;
    const keepFieldVisible = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const active = document.activeElement;
        if (active instanceof HTMLElement && active.matches('.edit-fields input:not([type="checkbox"]), .edit-fields select')) {
          active.scrollIntoView({ block: 'nearest', inline: 'nearest' });
        }
      });
    };
    window.addEventListener('resize', keepFieldVisible);
    window.visualViewport?.addEventListener('resize', keepFieldVisible);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener('resize', keepFieldVisible);
      window.visualViewport?.removeEventListener('resize', keepFieldVisible);
    };
  });
</script>
{@render children()}
