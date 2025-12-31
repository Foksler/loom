<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->

<!--
  SupportAccessDenied component
  
  Displayed when a support role user attempts to view a thread they don't have access to.
  Provides a "Request Access" button to initiate the support access workflow.
-->

<script lang="ts">
  import { Card, Button } from '$lib/ui';
  import { getApiClient } from '$lib/api';
  import { logger } from '$lib/logging';
  import type { SupportAccessRequest } from '$lib/api/types';

  interface Props {
    threadId: string;
    onAccessRequested?: (request: SupportAccessRequest) => void;
  }

  let { threadId, onAccessRequested }: Props = $props();

  let requestState = $state<'idle' | 'loading' | 'requested' | 'error'>('idle');
  let errorMessage = $state<string | null>(null);
  let pendingRequest = $state<SupportAccessRequest | null>(null);

  async function handleRequestAccess() {
    requestState = 'loading';
    errorMessage = null;

    try {
      const api = getApiClient();
      const request = await api.requestSupportAccess(threadId);
      
      pendingRequest = request;
      requestState = 'requested';
      
      logger.info('Support access requested', { threadId, requestId: request.request_id });
      onAccessRequested?.(request);
    } catch (error) {
      requestState = 'error';
      
      if (error instanceof Error) {
        try {
          const parsed = JSON.parse((error as { body?: string }).body || '{}');
          if (parsed.code === 'already_requested') {
            errorMessage = 'Access has already been requested for this thread. Waiting for owner approval.';
            requestState = 'requested';
          } else if (parsed.code === 'already_active') {
            errorMessage = 'You already have active access to this thread. Try refreshing the page.';
          } else {
            errorMessage = parsed.message || 'Failed to request access. Please try again.';
          }
        } catch {
          errorMessage = 'Failed to request access. Please try again.';
        }
      } else {
        errorMessage = 'An unexpected error occurred.';
      }
      
      logger.error('Failed to request support access', { threadId, error: String(error) });
    }
  }
</script>

<div class="flex flex-1 items-center justify-center p-8">
  <div class="max-w-md text-center">
  <Card padding="lg">
    {#if requestState === 'requested'}
      <div class="mb-4">
        <svg class="mx-auto h-12 w-12 text-success" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
      </div>
      <h2 class="text-lg font-semibold text-fg mb-2">Access Requested</h2>
      <p class="text-fg-muted mb-4">
        Your request has been sent to the thread owner. You'll be able to view this thread once they approve your request.
      </p>
      {#if pendingRequest}
        <p class="text-sm text-fg-subtle">
          Request ID: {pendingRequest.request_id.slice(0, 8)}...
        </p>
      {/if}
    {:else}
      <div class="mb-4">
        <svg class="mx-auto h-12 w-12 text-warning" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m0 0v2m0-2h2m-2 0H10m5-6a3 3 0 11-6 0 3 3 0 016 0zm-3 10a9 9 0 100-18 9 9 0 000 18z" />
        </svg>
      </div>
      <h2 class="text-lg font-semibold text-fg mb-2">Access Required</h2>
      <p class="text-fg-muted mb-4">
        This thread has not been shared with support. As a support team member, you can request access from the thread owner.
      </p>
      
      {#if errorMessage}
        <div class="mb-4 p-3 bg-error-soft text-error rounded-md text-sm">
          {errorMessage}
        </div>
      {/if}
      
      <Button
        variant="primary"
        onclick={handleRequestAccess}
        disabled={requestState === 'loading'}
      >
        {#if requestState === 'loading'}
          <span class="flex items-center gap-2">
            <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            Requesting...
          </span>
        {:else}
          Request Access
        {/if}
      </Button>
      
      <p class="mt-4 text-xs text-fg-subtle">
        The thread owner will receive a notification and can approve or deny your request.
        If approved, access will be granted for 31 days.
      </p>
    {/if}
  </Card>
  </div>
</div>
