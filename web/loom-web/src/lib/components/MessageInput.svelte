<script lang="ts">
  import { Button } from '../ui';

  interface Props {
    disabled?: boolean;
    placeholder?: string;
    onSubmit?: (content: string) => void;
  }

  let { disabled = false, placeholder = 'Type a message...', onSubmit }: Props = $props();

  let inputValue = $state('');

  function handleSubmit() {
    const content = inputValue.trim();
    if (content && !disabled) {
      onSubmit?.(content);
      inputValue = '';
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      handleSubmit();
    }
  }
</script>

<div class="border-t border-border p-4">
  <div class="flex gap-2">
    <textarea
      bind:value={inputValue}
      {placeholder}
      {disabled}
      rows="1"
      class="flex-1 resize-none rounded-lg border border-border bg-bg px-3 py-2 text-fg
             placeholder:text-fg-subtle focus:outline-none focus:ring-2 focus:ring-accent
             disabled:cursor-not-allowed disabled:opacity-50"
      onkeydown={handleKeydown}
    ></textarea>
    <Button
      variant="primary"
      disabled={disabled || !inputValue.trim()}
      onclick={handleSubmit}
    >
      Send
    </Button>
  </div>
</div>
