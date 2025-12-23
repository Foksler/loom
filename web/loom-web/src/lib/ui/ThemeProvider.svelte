<script lang="ts">
  import { onMount } from 'svelte';
  import { themeStore } from './theme';

  onMount(() => {
    themeStore.init();
  });
</script>

<slot />
