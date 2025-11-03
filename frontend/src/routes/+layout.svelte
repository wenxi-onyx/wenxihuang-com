<script lang="ts">
	import '../app.css';
	import { theme } from '$lib/stores/theme';
	import { authStore } from '$lib/stores/auth';
	import { onMount } from 'svelte';
	import Footer from '$lib/components/Footer.svelte';

	let { children } = $props();

	// Theme is already initialized in app.html inline script to prevent FOUC
	// Just sync the store with the current theme value
	if (typeof document !== 'undefined') {
		const currentTheme = document.documentElement.getAttribute('data-theme') as 'dark' | 'light';
		theme.syncTheme(currentTheme || 'dark');
	}

	// Check auth status in background - don't block page rendering
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
	<div class="content-wrapper">
		{@render children()}
	</div>

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
