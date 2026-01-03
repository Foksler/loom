<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { browser } from '$app/environment';

	interface SearchResult {
		url: string;
		meta: {
			title?: string;
		};
		excerpt: string;
	}

	let isOpen = $state(false);
	let query = $state('');
	let results = $state<SearchResult[]>([]);
	let selectedIndex = $state(0);
	let pagefind: any = null;
	let inputRef: HTMLInputElement;

	onMount(() => {
		if (browser) {
			(async () => {
				try {
					// Pagefind is generated at postbuild, use dynamic URL to avoid Vite analysis
					const pagefindUrl = '/pagefind/pagefind.js';
					pagefind = await import(/* @vite-ignore */ pagefindUrl);
					await pagefind.init();
				} catch (e) {
					console.warn('Pagefind not available (run build first):', e);
				}
			})();
		}

		function handleKeydown(e: KeyboardEvent) {
			if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
				e.preventDefault();
				isOpen = !isOpen;
				if (isOpen) {
					setTimeout(() => inputRef?.focus(), 0);
				}
			}

			if (e.key === 'Escape' && isOpen) {
				isOpen = false;
			}
		}

		document.addEventListener('keydown', handleKeydown);
		return () => document.removeEventListener('keydown', handleKeydown);
	});

	async function search() {
		if (!pagefind || !query.trim()) {
			results = [];
			return;
		}

		const searchResults = await pagefind.search(query);
		const data = await Promise.all(
			searchResults.results.slice(0, 10).map((r: any) => r.data())
		);
		results = data;
		selectedIndex = 0;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown') {
			e.preventDefault();
			selectedIndex = Math.min(selectedIndex + 1, results.length - 1);
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			selectedIndex = Math.max(selectedIndex - 1, 0);
		} else if (e.key === 'Enter' && results[selectedIndex]) {
			e.preventDefault();
			navigateTo(results[selectedIndex].url);
		}
	}

	function navigateTo(url: string) {
		isOpen = false;
		query = '';
		results = [];
		goto(url);
	}

	function close() {
		isOpen = false;
		query = '';
		results = [];
	}

	$effect(() => {
		if (query) {
			search();
		} else {
			results = [];
		}
	});
</script>

<button class="search-trigger" onclick={() => (isOpen = true)}>
	<span class="search-icon">⌕</span>
	<span class="search-text">Search docs...</span>
	<kbd class="search-kbd">⌘K</kbd>
</button>

{#if isOpen}
	<div class="search-overlay" role="dialog" aria-modal="true" aria-label="Search documentation">
		<button class="search-backdrop" onclick={close} aria-label="Close search"></button>

		<div class="search-modal">
			<div class="search-header">
				<span class="search-icon-large">⌕</span>
				<input
					bind:this={inputRef}
					bind:value={query}
					onkeydown={handleKeydown}
					type="text"
					class="search-input"
					placeholder="Search documentation..."
					aria-label="Search query"
				/>
				<button class="search-close" onclick={close}>
					<kbd>Esc</kbd>
				</button>
			</div>

			{#if results.length > 0}
				<ul class="search-results" role="listbox">
					{#each results as result, i}
						<li role="option" aria-selected={i === selectedIndex}>
							<button
								class="search-result"
								class:selected={i === selectedIndex}
								onclick={() => navigateTo(result.url)}
								onmouseenter={() => (selectedIndex = i)}
							>
								<span class="result-title">{result.meta?.title ?? 'Untitled'}</span>
								<span class="result-excerpt">{@html result.excerpt}</span>
							</button>
						</li>
					{/each}
				</ul>
			{:else if query.trim()}
				<div class="search-empty">
					<p>No results found for "{query}"</p>
				</div>
			{:else}
				<div class="search-empty">
					<p>Type to search documentation</p>
				</div>
			{/if}

			<div class="search-footer">
				<span class="search-hint">
					<kbd>↑</kbd><kbd>↓</kbd> to navigate
					<kbd>↵</kbd> to select
					<kbd>Esc</kbd> to close
				</span>
			</div>
		</div>
	</div>
{/if}

<style>
	.search-trigger {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		background: var(--color-bg-subtle);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
		cursor: pointer;
		transition: all 0.15s ease;
		width: 100%;
	}

	.search-trigger:hover {
		border-color: var(--color-accent);
		color: var(--color-fg);
	}

	.search-icon {
		font-size: var(--text-base);
	}

	.search-text {
		flex: 1;
		text-align: left;
	}

	.search-kbd {
		font-size: var(--text-xs);
		padding: 2px 6px;
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
	}

	.search-overlay {
		position: fixed;
		inset: 0;
		z-index: 1000;
		display: flex;
		align-items: flex-start;
		justify-content: center;
		padding-top: 10vh;
	}

	.search-backdrop {
		position: absolute;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		border: none;
		cursor: pointer;
	}

	.search-modal {
		position: relative;
		width: 90%;
		max-width: 600px;
		background: var(--color-bg);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-lg);
		overflow: hidden;
	}

	.search-header {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-4);
		border-bottom: 1px solid var(--color-border);
	}

	.search-icon-large {
		font-size: var(--text-xl);
		color: var(--color-fg-muted);
	}

	.search-input {
		flex: 1;
		background: transparent;
		border: none;
		font-family: var(--font-mono);
		font-size: var(--text-base);
		color: var(--color-fg);
		outline: none;
	}

	.search-input::placeholder {
		color: var(--color-fg-subtle);
	}

	.search-close {
		background: none;
		border: none;
		cursor: pointer;
		color: var(--color-fg-muted);
	}

	.search-close kbd {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		padding: 2px 6px;
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
	}

	.search-results {
		list-style: none;
		padding: 0;
		margin: 0;
		max-height: 400px;
		overflow-y: auto;
	}

	.search-result {
		display: block;
		width: 100%;
		padding: var(--space-3) var(--space-4);
		background: transparent;
		border: none;
		text-align: left;
		cursor: pointer;
		font-family: var(--font-mono);
		transition: background 0.1s ease;
	}

	.search-result:hover,
	.search-result.selected {
		background: var(--color-bg-muted);
	}

	.result-title {
		display: block;
		font-size: var(--text-sm);
		font-weight: 500;
		color: var(--color-fg);
		margin-bottom: var(--space-1);
	}

	.result-excerpt {
		display: block;
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
		line-height: 1.5;
	}

	.result-excerpt :global(mark) {
		background: var(--color-warning-soft);
		color: var(--color-warning);
		padding: 0 2px;
		border-radius: 2px;
	}

	.search-empty {
		padding: var(--space-8);
		text-align: center;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.search-footer {
		padding: var(--space-3) var(--space-4);
		border-top: 1px solid var(--color-border);
		background: var(--color-bg-muted);
	}

	.search-hint {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-subtle);
	}

	.search-hint kbd {
		display: inline-block;
		padding: 2px 4px;
		margin: 0 2px;
		background: var(--color-bg);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
	}
</style>
