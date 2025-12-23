<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    variant?: 'default' | 'accent' | 'success' | 'warning' | 'error' | 'muted';
    size?: 'sm' | 'md';
    children: Snippet;
  }

  let {
    variant = 'default',
    size = 'md',
    children,
  }: Props = $props();

  const variantClasses = {
    default: 'bg-bg-subtle text-fg',
    accent: 'bg-accent-soft text-accent',
    success: 'bg-success-soft text-success',
    warning: 'bg-warning-soft text-warning',
    error: 'bg-error-soft text-error',
    muted: 'bg-bg-muted text-fg-muted',
  };

  const sizeClasses = {
    sm: 'px-1.5 py-0.5 text-xs',
    md: 'px-2 py-1 text-sm',
  };
</script>

<span class="inline-flex items-center font-medium rounded-md {variantClasses[variant]} {sizeClasses[size]}">
  {@render children()}
</span>
