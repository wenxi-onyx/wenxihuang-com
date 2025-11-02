<script lang="ts">
	import { authStore } from '$lib/stores/auth';
	import UserMenu from './UserMenu.svelte';

	const user = $derived($authStore.user);
</script>

<div class="auth-container">
	{#if user}
		<UserMenu />
	{:else}
		<a href="/login" class="login-btn"> LOGIN </a>
	{/if}
</div>

<style>
	.auth-container {
		position: fixed;
		top: 2.5rem;
		right: 2rem;
		z-index: 100;
	}

	.login-btn {
		padding: 0.5rem 1.5rem;
		font-size: 0.875rem;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		background: transparent;
		text-decoration: none;
		cursor: pointer;
		transition: all 0.3s ease;
		color: var(--text-primary);
		display: inline-block;
	}

	/* Dark mode */
	:global([data-theme='dark']) .login-btn {
		color: #ffffff;
		border: 1px solid #ffffff;
	}

	:global([data-theme='dark']) .login-btn:hover {
		background: #ffffff;
		color: #000000;
	}

	/* Light mode */
	:global([data-theme='light']) .login-btn {
		color: #000000;
		border: 0.5px solid #000000;
		font-weight: 200;
	}

	:global([data-theme='light']) .login-btn:hover {
		background: #000000;
		color: #ffffff;
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
