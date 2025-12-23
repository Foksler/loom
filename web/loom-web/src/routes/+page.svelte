<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  onMount(() => {
    goto('/threads');
  });
</script>

<div class="flex items-center justify-center h-screen">
  <div class="animate-spin h-8 w-8 border-4 border-accent border-t-transparent rounded-full"></div>
</div>
