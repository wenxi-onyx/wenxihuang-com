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
		authStore.subscribe((state) => {
			if (state.user && !state.loading) {
				goto('/');
			}
		});
	});

	async function handleSubmit() {
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
			handleSubmit();
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

		<form class="login-form" on:submit|preventDefault={handleSubmit}>
			<div class="form-group">
				<label for="username">USERNAME</label>
				<input
					id="username"
					name="username"
					type="text"
					autocomplete="username"
					required
					bind:value={username}
					on:keypress={handleKeyPress}
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
					on:keypress={handleKeyPress}
					disabled={loading}
				/>
			</div>

			<button type="submit" disabled={loading} class="submit-btn">
				{#if loading}
					SIGNING IN...
				{:else}
					SIGN IN
				{/if}
			</button>

			<a href="/" class="back-link">BACK TO HOME</a>
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

	.form-group {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	label {
		font-size: 0.75rem;
		font-weight: 300;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		color: var(--text-primary);
		opacity: 0.7;
	}

	:global([data-theme='light']) label {
		font-weight: 200;
	}

	input {
		padding: 0.75rem 1rem;
		font-size: 1rem;
		font-family: inherit;
		background: transparent;
		color: var(--text-primary);
		border: 1px solid;
		outline: none;
		transition: border-color 0.2s ease, opacity 0.2s ease;
	}

	:global([data-theme='dark']) input {
		border-color: #ffffff;
	}

	:global([data-theme='light']) input {
		border-color: #000000;
	}

	input:focus {
		opacity: 1;
	}

	:global([data-theme='dark']) input:focus {
		border-color: #ffffff;
	}

	:global([data-theme='light']) input:focus {
		border-color: #000000;
	}

	input:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	input::placeholder {
		color: var(--text-primary);
		opacity: 0.3;
	}

	.submit-btn {
		margin-top: 1rem;
		padding: 0.875rem 2rem;
		font-size: 0.875rem;
		font-weight: 300;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		background: transparent;
		color: var(--text-primary);
		border: 1px solid;
		cursor: pointer;
		transition: all 0.3s ease;
	}

	:global([data-theme='dark']) .submit-btn {
		border-color: #ffffff;
		font-weight: 500;
	}

	:global([data-theme='light']) .submit-btn {
		border-color: #000000;
		font-weight: 200;
	}

	.submit-btn:hover:not(:disabled) {
		opacity: 1;
	}

	:global([data-theme='dark']) .submit-btn:hover:not(:disabled) {
		background: #ffffff;
		color: #000000;
	}

	:global([data-theme='light']) .submit-btn:hover:not(:disabled) {
		background: #000000;
		color: #ffffff;
	}

	.submit-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.back-link {
		display: block;
		text-align: center;
		font-size: 0.75rem;
		font-weight: 300;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		text-decoration: underline;
		text-decoration-thickness: 0.5px;
		color: var(--text-primary);
		opacity: 0.6;
		transition: opacity 0.3s ease;
	}

	:global([data-theme='light']) .back-link {
		font-weight: 200;
	}

	.back-link:hover {
		opacity: 1;
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
