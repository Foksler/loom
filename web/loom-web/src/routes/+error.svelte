<script lang="ts">
  import { page } from '$app/stores';
  import { Button, Card } from '$lib/ui';
</script>

<div class="flex items-center justify-center h-screen bg-bg">
  <Card padding="lg">
    <h1 class="text-2xl font-bold text-error mb-2">
      {$page.status}
    </h1>
    <p class="text-fg-muted mb-4">
      {$page.error?.message || 'An unexpected error occurred'}
    </p>
    <Button variant="primary" onclick={() => window.location.href = '/'}>
      Go Home
    </Button>
  </Card>
</div>
