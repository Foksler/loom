<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import type { ClipVisibility } from '$lib/api/clips';

	interface Props {
		visibility: ClipVisibility;
	}

	let { visibility }: Props = $props();

	const labels = {
		private: 'Private',
		internal: 'Internal',
		public: 'Public',
	};

	const colors = {
		private: 'badge-private',
		internal: 'badge-internal',
		public: 'badge-public',
	};
</script>

<span class="visibility-badge {colors[visibility]}">
	{labels[visibility]}
</span>

<style>
	.visibility-badge {
		display: inline-flex;
		align-items: center;
		padding: var(--space-1) var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		font-weight: 500;
		border-radius: var(--radius-sm);
		text-transform: uppercase;
	}

	.badge-private {
		background: color-mix(in srgb, var(--color-fg-muted) 15%, transparent);
		color: var(--color-fg-muted);
	}

	.badge-internal {
		background: color-mix(in srgb, var(--color-warning) 15%, transparent);
		color: var(--color-warning);
	}

	.badge-public {
		background: color-mix(in srgb, var(--color-success) 15%, transparent);
		color: var(--color-success);
	}
</style>
