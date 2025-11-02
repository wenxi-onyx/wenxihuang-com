<script lang="ts">
	import { theme } from '$lib/stores/theme';

	let currentTheme: 'dark' | 'light' = $state('dark');

	// Subscribe to theme changes
	$effect(() => {
		const unsubscribe = theme.subscribe((value) => {
			currentTheme = value;
		});
		return unsubscribe;
	});
</script>

<button class="theme-toggle" onclick={() => theme.toggle()} aria-label="Toggle theme">
	{#if currentTheme === 'dark'}
		<span class="icon">☾</span>
	{:else}
		<span class="icon">☀</span>
	{/if}
</button>

<style>
	.theme-toggle {
		position: fixed;
		top: 2rem;
		left: 2rem;
		background: transparent;
		border: none;
		cursor: pointer;
		padding: 0.5rem;
		transition: transform 0.3s ease;
		z-index: 100;
	}

	.theme-toggle:hover {
		transform: rotate(15deg);
	}

	.icon {
		font-size: 1.5rem;
		display: block;
		color: var(--text-primary);
		transition: color 0.3s ease;
	}
</style>
