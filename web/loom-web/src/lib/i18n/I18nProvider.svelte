<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import { loadCatalog, getPreferredLocale, type Locale } from './i18n';

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();
  let loaded = $state(false);

  onMount(async () => {
    const locale = getPreferredLocale();
    await loadCatalog(locale);
    loaded = true;
  });
</script>

{#if loaded}
  {@render children()}
{:else}
  <div class="flex items-center justify-center h-screen">
    <div class="animate-spin h-8 w-8 border-4 border-accent border-t-transparent rounded-full"></div>
  </div>
{/if}
