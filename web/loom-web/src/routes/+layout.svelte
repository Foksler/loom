<script lang="ts">
  import '../app.css';
  import { ThemeProvider } from '$lib/ui';
  import { I18nProvider } from '$lib/i18n';
  import type { Snippet } from 'svelte';

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();
</script>

<ThemeProvider>
  <I18nProvider>
    {@render children()}
  </I18nProvider>
</ThemeProvider>
