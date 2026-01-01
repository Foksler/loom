<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { i18n } from '$lib/i18n';
	import { Button } from '$lib/ui';

	interface Props {
		content: string;
		path: string;
		owner: string;
		repo: string;
		currentRef: string;
	}

	let { content, path, owner, repo, currentRef }: Props = $props();

	const lines = $derived(content.split('\n'));
	const fileName = $derived(path.split('/').pop() ?? '');
	const extension = $derived(fileName.split('.').pop()?.toLowerCase() ?? '');

	const isImage = $derived(['png', 'jpg', 'jpeg', 'gif', 'svg', 'webp', 'ico'].includes(extension));
	const isBinary = $derived(content.includes('\0') || (content.length > 0 && !/^[\x00-\x7F\u00A0-\u00FF\u0100-\uFFFF\n\r\t]*$/.test(content)));

	const languageClass = $derived(getLanguageClass(extension));

	function getLanguageClass(ext: string): string {
		const langMap: Record<string, string> = {
			js: 'javascript',
			jsx: 'javascript',
			ts: 'typescript',
			tsx: 'typescript',
			py: 'python',
			rb: 'ruby',
			rs: 'rust',
			go: 'go',
			java: 'java',
			c: 'c',
			cpp: 'cpp',
			h: 'c',
			hpp: 'cpp',
			cs: 'csharp',
			php: 'php',
			swift: 'swift',
			kt: 'kotlin',
			scala: 'scala',
			sh: 'bash',
			bash: 'bash',
			zsh: 'bash',
			fish: 'fish',
			ps1: 'powershell',
			sql: 'sql',
			html: 'html',
			htm: 'html',
			css: 'css',
			scss: 'scss',
			sass: 'sass',
			less: 'less',
			json: 'json',
			yaml: 'yaml',
			yml: 'yaml',
			xml: 'xml',
			md: 'markdown',
			mdx: 'markdown',
			toml: 'toml',
			ini: 'ini',
			cfg: 'ini',
			dockerfile: 'dockerfile',
			makefile: 'makefile',
			cmake: 'cmake',
			nix: 'nix',
			svelte: 'svelte',
			vue: 'vue',
			astro: 'astro',
		};
		return langMap[ext] ?? 'plaintext';
	}

	let copied = $state(false);

	async function copyContent() {
		await navigator.clipboard.writeText(content);
		copied = true;
		setTimeout(() => (copied = false), 2000);
	}

	const basePath = $derived(`/repos/${owner}/${repo}`);
</script>

<div class="border border-border rounded-lg overflow-hidden">
	<div class="flex items-center justify-between px-4 py-2 bg-bg-muted border-b border-border">
		<div class="flex items-center gap-4">
			<span class="text-sm font-medium text-fg">{fileName}</span>
			<span class="text-xs text-fg-muted">{lines.length} {i18n.t('client.repos.blob.lines')}</span>
			{#if !isImage && !isBinary}
				<span class="text-xs text-fg-muted">({new Blob([content]).size} {i18n.t('client.repos.blob.bytes')})</span>
			{/if}
		</div>
		<div class="flex items-center gap-2">
			<a href="{basePath}/blame/{currentRef}/{path}">
				<Button variant="ghost" size="sm">{i18n.t('client.repos.blob.blame')}</Button>
			</a>
			{#if !isImage && !isBinary}
				<Button variant="ghost" size="sm" onclick={copyContent}>
					{copied ? i18n.t('client.repos.blob.copied') : i18n.t('client.repos.blob.copy')}
				</Button>
			{/if}
			<Button variant="ghost" size="sm">{i18n.t('client.repos.blob.raw')}</Button>
		</div>
	</div>

	{#if isImage}
		<div class="flex items-center justify-center p-8 bg-bg">
			<img src="/api/repos/{owner}/{repo}/blob/{currentRef}/{path}" alt={fileName} class="max-w-full max-h-96" />
		</div>
	{:else if isBinary}
		<div class="flex items-center justify-center p-8 bg-bg text-fg-muted">
			{i18n.t('client.repos.blob.binary_not_shown')}
		</div>
	{:else}
		<div class="overflow-x-auto">
			<table class="w-full text-sm font-mono">
				<tbody>
					{#each lines as line, i}
						<tr class="hover:bg-bg-muted group">
							<td class="w-12 px-3 py-0.5 text-right text-fg-muted select-none border-r border-border bg-bg-muted sticky left-0">
								<a href="#{i + 1}" id={String(i + 1)} class="hover:text-accent">{i + 1}</a>
							</td>
							<td class="px-4 py-0.5 whitespace-pre text-fg">
								{line || ' '}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>
