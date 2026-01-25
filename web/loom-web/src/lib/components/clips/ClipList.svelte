<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import type { Clip } from '$lib/api/clips';
	import ClipCard from './ClipCard.svelte';

	interface Props {
		clips: Clip[];
		onclipclick?: (clip: Clip) => void;
	}

	let { clips, onclipclick }: Props = $props();
</script>

<div class="clip-list">
	{#each clips as clip (clip.id)}
		<ClipCard {clip} {onclipclick} />
	{/each}
</div>

<style>
	.clip-list {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
		gap: var(--space-4);
	}
</style>
