<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { i18n } from '$lib/i18n';
	import { getApiClient } from '$lib/api/client';
	import { Card, Button, Input } from '$lib/ui';
	import { trackButtonClick, trackFormSubmit, trackAction } from '$lib/analytics';

	const client = getApiClient();

	type Step = 'input' | 'verify';

	let step = $state<Step>('input');
	let phoneNumber = $state('');
	let verificationCode = $state('');
	let loading = $state(false);
	let error = $state<string | null>(null);
	let linkedPhone = $state<string | null>(null);
	let expiresIn = $state(0);

	async function requestVerification() {
		if (!phoneNumber.trim()) return;
		trackButtonClick('request_whatsapp_otp');
		loading = true;
		error = null;
		try {
			const response = await client.requestWhatsAppLink(phoneNumber);
			expiresIn = response.expires_in_seconds;
			step = 'verify';
		} catch (e) {
			error = e instanceof Error ? e.message : i18n._('general.error');
		} finally {
			loading = false;
		}
	}

	async function verifyCode() {
		if (!verificationCode.trim()) return;
		trackFormSubmit('whatsapp_verify');
		loading = true;
		error = null;
		try {
			const response = await client.verifyWhatsAppLink(phoneNumber, verificationCode);
			trackAction('link', 'whatsapp_phone', phoneNumber);
			linkedPhone = response.phone_number;
			step = 'input';
			phoneNumber = '';
			verificationCode = '';
		} catch (e) {
			error = e instanceof Error ? e.message : i18n._('general.error');
		} finally {
			loading = false;
		}
	}

	async function unlinkPhone() {
		if (!confirm('Unlink your WhatsApp number?')) return;
		trackAction('unlink', 'whatsapp_phone', linkedPhone!);
		loading = true;
		error = null;
		try {
			await client.unlinkWhatsApp();
			linkedPhone = null;
		} catch (e) {
			error = e instanceof Error ? e.message : i18n._('general.error');
		} finally {
			loading = false;
		}
	}

	function goBack() {
		step = 'input';
		verificationCode = '';
		error = null;
	}
</script>

<svelte:head>
	<title>WhatsApp - Profile Settings - Loom</title>
</svelte:head>

<div class="mb-6">
	<nav class="text-sm text-fg-muted mb-2">
		<a href="/settings/profile" class="hover:text-fg">Profile</a>
		<span class="mx-2">/</span>
		<span>WhatsApp</span>
	</nav>
	<h1 class="text-2xl font-bold text-fg">Link WhatsApp</h1>
	<p class="text-fg-muted">Connect your WhatsApp number to receive AI responses via WhatsApp</p>
</div>

{#if error}
	<div class="mb-4 p-3 rounded-md bg-error/10 text-error text-sm">{error}</div>
{/if}

<Card>
	{#if linkedPhone}
		<div class="flex items-center justify-between">
			<div>
				<p class="text-fg font-medium">Linked Phone Number</p>
				<p class="text-lg text-fg">{linkedPhone}</p>
			</div>
			<Button variant="danger" onclick={unlinkPhone} disabled={loading} loading={loading}>
				Unlink
			</Button>
		</div>
	{:else if step === 'input'}
		<p class="text-fg-muted mb-4">
			Link your WhatsApp number to receive AI responses directly in WhatsApp.
			We'll send a verification code to your phone.
		</p>

		<form onsubmit={(e) => { e.preventDefault(); requestVerification(); }} class="space-y-4">
			<Input
				label="Phone Number"
				placeholder="+1234567890"
				bind:value={phoneNumber}
							/>
			<p class="text-sm text-fg-muted">
				Enter your phone number in E.164 format (with country code)
			</p>

			<Button type="submit" disabled={loading || !phoneNumber.trim()} loading={loading}>
				Send Verification Code
			</Button>
		</form>
	{:else}
		<p class="text-fg-muted mb-4">
			Enter the 6-digit code sent to <span class="font-medium text-fg">{phoneNumber}</span>
		</p>

		<form onsubmit={(e) => { e.preventDefault(); verifyCode(); }} class="space-y-4">
			<Input
				label="Verification Code"
				placeholder="123456"
				bind:value={verificationCode}
				maxlength={6}
							/>
			<p class="text-sm text-fg-muted">
				Code expires in {Math.floor(expiresIn / 60)} minutes
			</p>

			<div class="flex gap-2">
				<Button variant="secondary" type="button" onclick={goBack}>
					Back
				</Button>
				<Button type="submit" disabled={loading || verificationCode.length !== 6} loading={loading}>
					Verify
				</Button>
			</div>
		</form>
	{/if}
</Card>
