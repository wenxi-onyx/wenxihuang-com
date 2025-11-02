<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { theme } from '$lib/stores/theme';
	import Footer from '$lib/components/Footer.svelte';

	let { children } = $props();

	onMount(() => {
		theme.init();
	});
</script>

<svelte:head>
	<link rel="icon" href="/favicon.svg" type="image/svg+xml" />
	<title>Wenxi Huang</title>
	<meta name="description" content="Personal website of Wenxi Huang" />
</svelte:head>

{@render children()}

<Footer />
