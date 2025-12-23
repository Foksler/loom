<script lang="ts">
  interface Props {
    width?: string;
    height?: string;
    rounded?: 'sm' | 'md' | 'lg' | 'full';
  }

  let {
    width = '100%',
    height = '1rem',
    rounded = 'md',
  }: Props = $props();

  const roundedClasses = {
    sm: 'rounded-sm',
    md: 'rounded-md',
    lg: 'rounded-lg',
    full: 'rounded-full',
  };
</script>

<div
  class="animate-pulse bg-bg-subtle {roundedClasses[rounded]}"
  style="width: {width}; height: {height};"
></div>
