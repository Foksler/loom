<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    padding?: 'none' | 'sm' | 'md' | 'lg';
    hover?: boolean;
    header?: Snippet;
    footer?: Snippet;
    children: Snippet;
  }

  let {
    padding = 'md',
    hover = false,
    header,
    footer,
    children,
  }: Props = $props();

  const paddingClasses = {
    none: '',
    sm: 'p-3',
    md: 'p-4',
    lg: 'p-6',
  };
</script>

<div
  class="rounded-lg border border-border bg-bg shadow-card {hover ? 'transition-shadow hover:shadow-card-hover cursor-pointer' : ''}"
>
  {#if header}
    <div class="border-b border-border px-4 py-3">
      {@render header()}
    </div>
  {/if}
  
  <div class={paddingClasses[padding]}>
    {@render children()}
  </div>
  
  {#if footer}
    <div class="border-t border-border px-4 py-3 bg-bg-muted">
      {@render footer()}
    </div>
  {/if}
</div>
