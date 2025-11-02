<script lang="ts">
	import '../app.css';
	import { theme } from '$lib/stores/theme';
	import Footer from '$lib/components/Footer.svelte';

	let { children } = $props();

	// Theme is already initialized in app.html inline script to prevent FOUC
	// Just sync the store with the current theme value
	if (typeof document !== 'undefined') {
		const currentTheme = document.documentElement.getAttribute('data-theme') as 'dark' | 'light';
		theme.syncTheme(currentTheme || 'dark');
	}
</script>

<svelte:head>
	<link rel="icon" href="/favicon.svg?v=2" type="image/svg+xml" />
	<title>Wenxi Huang</title>
	<meta name="description" content="Personal website of Wenxi Huang" />
</svelte:head>

{@render children()}

<Footer />
