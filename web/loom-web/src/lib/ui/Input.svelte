<script lang="ts">
  interface Props {
    value?: string;
    placeholder?: string;
    type?: 'text' | 'email' | 'password' | 'search';
    disabled?: boolean;
    error?: string;
    label?: string;
    id?: string;
    class?: string;
    oninput?: (event: Event) => void;
    onkeydown?: (event: KeyboardEvent) => void;
  }

  let {
    value = $bindable(''),
    placeholder = '',
    type = 'text',
    disabled = false,
    error,
    label,
    id,
    class: className,
    oninput,
    onkeydown,
  }: Props = $props();

  const fallbackId = `input-${Math.random().toString(36).slice(2)}`;
  const inputId = $derived(id || fallbackId);
</script>

<div class="w-full {className ?? ''}">
  {#if label}
    <label for={inputId} class="block text-sm font-medium text-fg mb-1.5">
      {label}
    </label>
  {/if}
  
  <input
    {type}
    id={inputId}
    bind:value
    {placeholder}
    {disabled}
    class="w-full h-10 px-3 rounded-md border bg-bg text-fg placeholder:text-fg-subtle
           focus:outline-none focus:ring-2 focus:ring-accent focus:ring-offset-2 focus:ring-offset-bg
           disabled:cursor-not-allowed disabled:opacity-50
           {error ? 'border-error focus:ring-error' : 'border-border'}"
    oninput={oninput}
    onkeydown={onkeydown}
  />
  
  {#if error}
    <p class="mt-1.5 text-sm text-error">{error}</p>
  {/if}
</div>
