<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/stores/auth';
	import { gamesApi, seasonsApi, type ActiveSeasonPlayer, type Season } from '$lib/api/client';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import LoginButton from '$lib/components/LoginButton.svelte';

	const user = $derived($authStore.user);

	let players = $state<ActiveSeasonPlayer[]>([]);
	let activeSeason = $state<Season | null>(null);
	let loading = $state(true);
	let submitting = $state(false);
	let error = $state('');
	let success = $state('');

	// Form state
	let player1Id = $state('');
	let player2Id = $state('');
	let player1Score = $state<number>(0);
	let player2Score = $state<number>(0);
	let playedAt = $state('');

	onMount(async () => {
		// Check if user is authenticated
		if (!user) {
			goto('/login');
			return;
		}

		try {
			loading = true;

			// Load active season and its players in parallel
			const [season, seasonPlayers] = await Promise.all([
				seasonsApi.getActiveSeason(),
				seasonsApi.getActiveSeasonPlayers()
			]);

			activeSeason = season;
			players = seasonPlayers;

			loading = false;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load data';
			loading = false;
		}
	});

	async function handleSubmit(e: Event) {
		e.preventDefault();

		// Reset messages
		error = '';
		success = '';

		// Validate form
		if (!player1Id || !player2Id) {
			error = 'Please select both players';
			return;
		}

		if (player1Id === player2Id) {
			error = 'Please select different players';
			return;
		}

		if (player1Score < 0 || player2Score < 0) {
			error = 'Scores cannot be negative';
			return;
		}

		if (player1Score === player2Score) {
			error = 'Game cannot be a tie';
			return;
		}

		try {
			submitting = true;

			// Convert datetime-local string to ISO 8601 format with timezone
			let playedAtISO: string | undefined = undefined;
			if (playedAt) {
				// datetime-local gives us "2025-11-04T14:30", we need to convert to ISO 8601 with timezone
				const localDate = new Date(playedAt);
				playedAtISO = localDate.toISOString();
			}

			await gamesApi.createGame({
				player1_id: player1Id,
				player2_id: player2Id,
				player1_score: player1Score,
				player2_score: player2Score,
				played_at: playedAtISO
			});

			success = 'Match recorded successfully!';

			// Reset form
			player1Id = '';
			player2Id = '';
			player1Score = 0;
			player2Score = 0;
			playedAt = '';

			// Clear success message after a few seconds
			setTimeout(() => {
				success = '';
			}, 3000);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to record match';
		} finally {
			submitting = false;
		}
	}

	function getPlayerName(playerId: string): string {
		const player = players.find(p => p.id === playerId);
		return player ? player.name : '';
	}

	// Check if players are available for better UX
	let playersAvailable = $derived(players.length > 0);
</script>

<svelte:head>
	<title>Add Match - Table Tennis</title>
</svelte:head>

<ThemeToggle />
<LoginButton />

<div class="container">
	<header class="page-header">
		<h1>Add Match Result</h1>
		<nav class="nav-links">
			<a href="/table-tennis">BACK TO LEADERBOARD</a>
		</nav>
	</header>

	{#if !user}
		<div class="error-card">
			<p>You must be logged in to add match results.</p>
			<a href="/login" class="btn">Go to Login</a>
		</div>
	{:else if loading}
		<div class="loading">Loading...</div>
	{:else if !activeSeason}
		<div class="error-card">
			<p>No active season found. Please contact an administrator.</p>
		</div>
	{:else if !playersAvailable}
		<div class="error-card">
			<p>No players available in the active season. Please contact an administrator to add players to the season.</p>
		</div>
	{:else}
		{#if error}
			<div class="alert alert-error">
				{error}
				<button class="btn-close" onclick={() => error = ''}>×</button>
			</div>
		{/if}

		{#if success}
			<div class="alert alert-success">
				{success}
			</div>
		{/if}

		<div class="form-card">
			<div class="season-info">
				<h3>Current Season: {activeSeason.name}</h3>
				{#if activeSeason.description}
					<p>{activeSeason.description}</p>
				{/if}
			</div>

			<form onsubmit={handleSubmit}>
				<div class="form-row">
					<div class="form-group">
						<label for="player1">Player 1 (Winner if higher score)</label>
						<select
							id="player1"
							bind:value={player1Id}
							required
							disabled={submitting}
						>
							<option value="">Select Player 1</option>
							{#each players as player}
								<option value={player.id}>{player.name}</option>
							{/each}
						</select>
					</div>

					<div class="form-group">
						<label for="player1Score">Player 1 Score</label>
						<input
							type="number"
							id="player1Score"
							bind:value={player1Score}
							min="0"
							required
							disabled={submitting}
						/>
					</div>
				</div>

				<div class="vs-divider">VS</div>

				<div class="form-row">
					<div class="form-group">
						<label for="player2">Player 2 (Winner if higher score)</label>
						<select
							id="player2"
							bind:value={player2Id}
							required
							disabled={submitting}
						>
							<option value="">Select Player 2</option>
							{#each players as player}
								<option value={player.id}>{player.name}</option>
							{/each}
						</select>
					</div>

					<div class="form-group">
						<label for="player2Score">Player 2 Score</label>
						<input
							type="number"
							id="player2Score"
							bind:value={player2Score}
							min="0"
							required
							disabled={submitting}
						/>
					</div>
				</div>

				<div class="form-group">
					<label for="playedAt">Match Date/Time (optional, defaults to now)</label>
					<input
						type="datetime-local"
						id="playedAt"
						bind:value={playedAt}
						disabled={submitting}
					/>
					<p class="help-text">Leave blank to use current date and time</p>
				</div>

				<div class="form-actions">
					<button type="submit" class="btn btn-primary" disabled={submitting}>
						{submitting ? 'Recording...' : 'Record Match'}
					</button>
					<a href="/table-tennis" class="btn btn-secondary">
						Cancel
					</a>
				</div>
			</form>
		</div>
	{/if}
</div>

<style>
	.container {
		max-width: 800px;
		margin: 0 auto;
		padding: 6rem 2rem 4rem 2rem;
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 3rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid var(--border-subtle);
	}

	.page-header h1 {
		font-size: clamp(1.5rem, 4vw, 2.5rem);
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		margin: 0;
		color: var(--text-primary);
	}

	.nav-links a {
		font-size: 0.875rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		text-decoration: none;
		color: inherit;
		opacity: 0.7;
		transition: opacity 0.2s ease;
	}

	.nav-links a:hover {
		opacity: 1;
	}

	.loading {
		text-align: center;
		padding: 3rem;
		font-size: 1rem;
		color: var(--text-primary);
	}

	.error-card {
		text-align: center;
		padding: 3rem;
		border: 1px solid var(--border-subtle);
		background: transparent;
	}

	.error-card p {
		margin-bottom: 1.5rem;
		color: var(--text-primary);
	}

	.alert {
		padding: 1rem 1.5rem;
		border-radius: 0;
		margin-bottom: 1.5rem;
		display: flex;
		justify-content: space-between;
		align-items: center;
		border: 1px solid var(--border-subtle);
	}

	.alert-error {
		background: transparent;
		color: var(--text-primary);
		border-color: rgba(220, 38, 38, 0.3);
	}

	.alert-success {
		background: transparent;
		color: var(--text-primary);
		border-color: rgba(22, 163, 74, 0.3);
	}

	.btn-close {
		background: none;
		border: none;
		font-size: 1.5rem;
		cursor: pointer;
		padding: 0;
		width: 2rem;
		height: 2rem;
		display: flex;
		align-items: center;
		justify-content: center;
		color: inherit;
		opacity: 0.6;
	}

	.btn-close:hover {
		opacity: 1;
	}

	.form-card {
		background: transparent;
		border: 1px solid var(--border-subtle);
		padding: 2rem;
	}

	.season-info {
		margin-bottom: 2rem;
		padding-bottom: 1.5rem;
		border-bottom: 1px solid var(--border-subtle);
	}

	.season-info h3 {
		margin: 0 0 0.5rem 0;
		font-size: 1.25rem;
		font-weight: 300;
		color: var(--text-primary);
		letter-spacing: 0.05em;
	}

	.season-info p {
		margin: 0;
		font-size: 0.875rem;
		color: var(--text-primary);
		opacity: 0.7;
	}

	.form-row {
		display: grid;
		grid-template-columns: 2fr 1fr;
		gap: 1.5rem;
		margin-bottom: 1.5rem;
	}

	.form-group {
		display: flex;
		flex-direction: column;
	}

	label {
		display: block;
		margin-bottom: 0.5rem;
		font-weight: 300;
		font-size: 0.875rem;
		color: var(--text-primary);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	select,
	input[type="number"],
	input[type="datetime-local"] {
		width: 100%;
		padding: 0.75rem;
		font-size: 1rem;
		font-family: inherit;
		border: 1px solid var(--border-subtle);
		background: transparent;
		color: var(--text-primary);
		outline: none;
		transition: border-color 0.2s ease;
	}

	select:focus,
	input:focus {
		border-color: var(--border-active);
	}

	select:disabled,
	input:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	select option {
		background: var(--bg-primary, #ffffff);
		color: var(--text-primary);
	}

	:global([data-theme='dark']) select option {
		background: #1a1a1a;
	}

	.vs-divider {
		text-align: center;
		padding: 1rem 0;
		font-size: 1.5rem;
		font-weight: 200;
		letter-spacing: 0.2em;
		color: var(--text-primary);
		opacity: 0.5;
	}

	.help-text {
		margin: 0.25rem 0 0;
		font-size: 0.75rem;
		color: var(--text-primary);
		opacity: 0.5;
		text-transform: none;
		letter-spacing: normal;
	}

	.form-actions {
		display: flex;
		gap: 1rem;
		justify-content: flex-end;
		margin-top: 2rem;
		padding-top: 1.5rem;
		border-top: 1px solid var(--border-subtle);
	}

	.btn {
		padding: 0.75rem 2rem;
		font-size: 0.875rem;
		font-weight: 300;
		font-family: inherit;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		text-decoration: none;
		border: 1px solid var(--border-subtle);
		background: transparent;
		color: var(--text-primary);
		cursor: pointer;
		transition: all 0.2s ease;
		display: inline-block;
		text-align: center;
	}

	.btn:hover:not(:disabled) {
		border-color: var(--border-active);
		opacity: 0.8;
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-primary {
		background: var(--text-primary);
		color: var(--bg-primary);
		border-color: var(--text-primary);
	}

	.btn-primary:hover:not(:disabled) {
		opacity: 0.9;
	}

	.btn-secondary {
		background: transparent;
		color: var(--text-primary);
	}

	@media (max-width: 768px) {
		.container {
			padding: 5rem 1rem 3rem 1rem;
		}

		.page-header {
			flex-direction: column;
			gap: 1rem;
			align-items: flex-start;
		}

		.form-card {
			padding: 1.5rem;
		}

		.form-row {
			grid-template-columns: 1fr;
			gap: 1rem;
		}

		.form-actions {
			flex-direction: column;
		}

		.btn {
			width: 100%;
		}
	}
</style>
