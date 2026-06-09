<script lang="ts">
	import { authStore } from '$lib/stores/auth';
	import UserMenu from './UserMenu.svelte';

	const user = $derived($authStore.user);
</script>

<div class="auth-container">
	{#if user}
		<UserMenu />
	{:else}
		<a href="/login" class="btn login-btn"> LOGIN </a>
	{/if}
</div>

<style>
	.auth-container {
		position: fixed;
		top: 2.5rem;
		right: 2rem;
		z-index: 100;
	}

	/* Button styles use shared .btn class from buttons.css */
	.login-btn {
		padding: 0.5rem 1.5rem;
	}

	@media (max-width: 768px) {
		.auth-container {
			top: 1rem;
			right: 1rem;
		}

		.login-btn {
			padding: 0.375rem 1rem;
			font-size: 0.75rem;
		}
	}
</style>
