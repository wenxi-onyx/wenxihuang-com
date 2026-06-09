<script lang="ts">
	import { theme } from '$lib/stores/theme';

	// Use $derived to reactively track the theme store value
	let currentTheme = $derived($theme);
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

	@media (max-width: 768px) {
		.theme-toggle {
			top: 1rem;
			left: 1rem;
			padding: 0.25rem;
		}

		.icon {
			font-size: 1.25rem;
		}
	}
</style>
