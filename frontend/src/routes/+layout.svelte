<script lang="ts">
	import '../app.css';
	import { theme } from '$lib/stores/theme';
	import { authStore } from '$lib/stores/auth';
	import { onMount } from 'svelte';
	import Footer from '$lib/components/Footer.svelte';

	let { children } = $props();

	const authLoading = $derived($authStore.loading);

	// Theme is already initialized in app.html inline script to prevent FOUC
	// Just sync the store with the current theme value
	if (typeof document !== 'undefined') {
		const currentTheme = document.documentElement.getAttribute('data-theme') as 'dark' | 'light';
		theme.syncTheme(currentTheme || 'dark');
	}

	// Check auth status on mount
	onMount(async () => {
		await authStore.checkAuth();
	});
</script>

<svelte:head>
	<link rel="icon" href="/favicon.svg?v=2" type="image/svg+xml" />
	<title>Wenxi Huang</title>
	<meta name="description" content="Personal website of Wenxi Huang" />
</svelte:head>

<div class="app-container">
	{#if authLoading}
		<div class="min-h-screen flex items-center justify-center">
			<div class="text-center">
				<svg class="animate-spin h-12 w-12 mx-auto" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
					<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
					<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
				</svg>
				<p class="mt-4">Loading...</p>
			</div>
		</div>
	{:else}
		<div class="content-wrapper">
			{@render children()}
		</div>
	{/if}

	<Footer />
</div>

<style>
	.app-container {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
	}

	.content-wrapper {
		flex: 1;
	}
</style>
