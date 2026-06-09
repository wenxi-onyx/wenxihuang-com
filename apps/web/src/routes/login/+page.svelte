<script lang="ts">
	import { authStore } from '$lib/stores/auth';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';

	let username = '';
	let password = '';
	let loading = false;
	let error = '';

	// Redirect if already logged in
	onMount(() => {
		const unsubscribe = authStore.subscribe((state) => {
			if (state.user && !state.loading) {
				goto('/');
			}
		});

		return unsubscribe; // Clean up subscription when component is destroyed
	});

	async function handleSubmit(e: Event) {
		e.preventDefault();
		if (!username || !password) {
			error = 'Please enter both username and password';
			return;
		}

		loading = true;
		error = '';

		const result = await authStore.login(username, password);

		if (!result.success) {
			error = result.error || 'Login failed';
			loading = false;
		}
		// On success, the store will redirect automatically
	}

	function handleKeyPress(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault();
			const form = (event.target as HTMLElement).closest('form');
			if (form) {
				form.requestSubmit();
			}
		}
	}
</script>

<ThemeToggle />

<main class="login-page">
	<div class="login-content">
		{#if error}
			<div class="error-message">
				{error}
			</div>
		{/if}

		<form class="login-form" onsubmit={handleSubmit}>
			<div class="form-group">
				<label for="username">USERNAME</label>
				<input
					id="username"
					name="username"
					type="text"
					autocomplete="username"
					required
					bind:value={username}
					onkeypress={handleKeyPress}
					disabled={loading}
				/>
			</div>

			<div class="form-group">
				<label for="password">PASSWORD</label>
				<input
					id="password"
					name="password"
					type="password"
					autocomplete="current-password"
					required
					bind:value={password}
					onkeypress={handleKeyPress}
					disabled={loading}
				/>
			</div>

			<button type="submit" disabled={loading} class="btn submit-btn">
				{#if loading}
					SIGNING IN...
				{:else}
					SIGN IN
				{/if}
			</button>

			<a href="/" class="btn-secondary back-link">BACK TO HOME</a>
		</form>
	</div>
</main>

<style>
	.login-page {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 4rem 2rem;
	}

	.login-content {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2rem;
		width: 100%;
		max-width: 400px;
		margin-top: 25vh;
	}

	.title {
		font-size: clamp(2rem, 4vw, 3rem);
		font-weight: 300;
		letter-spacing: 0.15em;
		text-align: center;
		margin: 0;
		color: var(--text-primary);
	}

	:global([data-theme='dark']) .title {
		font-family: -apple-system, BlinkMacSystemFont, 'Helvetica Neue', Arial, sans-serif;
		font-weight: 700;
	}

	:global([data-theme='light']) .title {
		font-family: 'Noto Sans JP', sans-serif;
		font-weight: 100;
		letter-spacing: 0.2em;
	}

	.error-message {
		width: 100%;
		padding: 0.75rem;
		text-align: center;
		font-size: 0.875rem;
		letter-spacing: 0.05em;
		border: 1px solid;
		color: var(--text-primary);
		opacity: 0.8;
	}

	:global([data-theme='dark']) .error-message {
		border-color: #ff6b6b;
		background: rgba(255, 107, 107, 0.1);
	}

	:global([data-theme='light']) .error-message {
		border-color: #d32f2f;
		background: rgba(211, 47, 47, 0.05);
	}

	.login-form {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
		width: 100%;
	}

	/* Form group, label, and input styles now use shared classes from forms.css */

	input::placeholder {
		color: var(--text-primary);
		opacity: 0.3;
	}

	/* Button and link styles now use shared .btn and .btn-secondary from buttons.css */
	.submit-btn {
		margin-top: 1rem;
		padding: 0.875rem 2rem;
	}

	.back-link {
		display: block;
		text-align: center;
	}

	@media (max-width: 768px) {
		.login-page {
			padding: 3rem 1.5rem;
		}

		.title {
			font-size: 2rem;
		}

		.login-content {
			max-width: 100%;
		}
	}
</style>
