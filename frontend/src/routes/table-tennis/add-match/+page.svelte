<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/stores/auth';
	import { matchesApi, seasonsApi, type ActiveSeasonPlayer, type Season, type GameWinner } from '$lib/api/client';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import LoginButton from '$lib/components/LoginButton.svelte';
	import Toast, { showToast } from '$lib/components/Toast.svelte';

	const user = $derived($authStore.user);

	let players = $state<ActiveSeasonPlayer[]>([]);
	let activeSeason = $state<Season | null>(null);
	let loading = $state(true);
	let submitting = $state(false);

	// Form state
	let player1Id = $state('');
	let player2Id = $state('');

	// Default to current local time rounded down to nearest 5-minute interval
	function getDefaultDateTime(): string {
		const now = new Date();
		// Round down to nearest 5-minute interval
		const minutes = now.getMinutes();
		const roundedMinutes = Math.floor(minutes / 5) * 5;
		now.setMinutes(roundedMinutes);
		now.setSeconds(0);
		now.setMilliseconds(0);

		// Format as YYYY-MM-DDTHH:MM for datetime-local input
		const year = now.getFullYear();
		const month = String(now.getMonth() + 1).padStart(2, '0');
		const day = String(now.getDate()).padStart(2, '0');
		const hours = String(now.getHours()).padStart(2, '0');
		const formattedMinutes = String(now.getMinutes()).padStart(2, '0');
		return `${year}-${month}-${day}T${hours}:${formattedMinutes}`;
	}

	let submittedAt = $state(getDefaultDateTime());

	// Games state - track winner for each game
	let games = $state<(GameWinner | null)[]>([null, null, null, null, null]); // Start with 5 empty games
	const MAX_GAMES = 11; // Reasonable limit for a match

	// Derived score
	let player1GamesWon = $derived(games.filter(g => g === 'Player1').length);
	let player2GamesWon = $derived(games.filter(g => g === 'Player2').length);
	let score = $derived(`${player1GamesWon}-${player2GamesWon}`);

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
			showToast(e instanceof Error ? e.message : 'Failed to load data', 'error');
			loading = false;
		}
	});

	async function handleGameWinner(gameIndex: number, winner: GameWinner) {
		// Set the winner for this game
		games[gameIndex] = winner;

		// Add a new empty game row if:
		// 1. This was the last game in the array
		// 2. We haven't reached the max games limit
		if (gameIndex === games.length - 1 && games.length < MAX_GAMES) {
			games = [...games, null];

			// Wait for DOM update, then scroll to the new row
			await tick();
			const newRow = document.getElementById(`game-${games.length - 1}`);
			if (newRow) {
				newRow.scrollIntoView({ behavior: 'smooth', block: 'center' });
			}
		}
	}

	function removeGame(gameIndex: number) {
		// Don't remove if it's the only game
		if (games.length === 1) {
			games = [null, null, null, null, null];
		} else {
			games = games.filter((_, i) => i !== gameIndex);
		}
	}

	async function handleSubmit(e: Event) {
		e.preventDefault();

		// Validate form
		if (!player1Id || !player2Id) {
			showToast('Please select both players', 'error');
			return;
		}

		if (player1Id === player2Id) {
			showToast('Please select different players', 'error');
			return;
		}

		// Filter out null games (unplayed games)
		const playedGames = games.filter(g => g !== null) as GameWinner[];

		if (playedGames.length === 0) {
			showToast('Please record at least one game', 'error');
			return;
		}

		try {
			submitting = true;

			// Convert datetime-local string to ISO 8601 format with timezone
			let submittedAtISO: string | undefined = undefined;
			if (submittedAt) {
				// datetime-local gives us "2025-11-04T14:30", we need to convert to ISO 8601 with timezone
				const localDate = new Date(submittedAt);
				submittedAtISO = localDate.toISOString();
			}

			await matchesApi.createMatch({
				player1_id: player1Id,
				player2_id: player2Id,
				games: playedGames,
				submitted_at: submittedAtISO
			});

			showToast('Match recorded successfully!', 'success');

			// Reset form
			player1Id = '';
			player2Id = '';
			games = [null, null, null, null, null];
			submittedAt = getDefaultDateTime();
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to record match', 'error');
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
	let bothPlayersSelected = $derived(player1Id && player2Id && player1Id !== player2Id);
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
			<button class="nav-link-btn" onclick={() => window.history.back()}>BACK</button>
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
		<div class="form-card">
			<div class="season-info">
				<h3>Current Season: {activeSeason.name}</h3>
				{#if activeSeason.description}
					<p>{activeSeason.description}</p>
				{/if}
			</div>

			<form onsubmit={handleSubmit}>
				<div class="player-selection">
					<div class="form-group">
						<label for="player1">Player 1</label>
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

					<div class="vs-divider">VS</div>

					<div class="form-group">
						<label for="player2">Player 2</label>
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
				</div>

				{#if bothPlayersSelected}
					<div class="games-section">
						<div class="games-header">
							<h3>Record Games</h3>
							<div class="score-display">{score}</div>
						</div>
						<p class="help-text">Click the winner's name for each game. New rows will appear automatically.</p>

						<div class="games-list">
							{#each games as game, index (index)}
								<div class="game-row" id="game-{index}">
									<div class="game-number">Game {index + 1}</div>
									<div class="game-winners">
										<button
											type="button"
											class="winner-btn"
											class:selected={game === 'Player1'}
											onclick={() => handleGameWinner(index, 'Player1')}
											disabled={submitting}
										>
											<span class="checkbox" class:checked={game === 'Player1'}>
												{#if game === 'Player1'}✓{/if}
											</span>
											<span class="player-name">{getPlayerName(player1Id)}</span>
										</button>

										<button
											type="button"
											class="winner-btn"
											class:selected={game === 'Player2'}
											onclick={() => handleGameWinner(index, 'Player2')}
											disabled={submitting}
										>
											<span class="checkbox" class:checked={game === 'Player2'}>
												{#if game === 'Player2'}✓{/if}
											</span>
											<span class="player-name">{getPlayerName(player2Id)}</span>
										</button>
									</div>
									<button
										type="button"
										class="remove-btn"
										class:invisible={!(games.length > 1 && game !== null)}
										onclick={() => removeGame(index)}
										disabled={submitting || !(games.length > 1 && game !== null)}
										title="Remove this game"
									>
										×
									</button>
								</div>
							{/each}
						</div>
					</div>
				{/if}

				<div class="form-group">
					<label for="submittedAt">Match Date/Time</label>
					<input
						type="datetime-local"
						id="submittedAt"
						bind:value={submittedAt}
						disabled={submitting}
					/>
					<p class="help-text">Adjust the date and time as needed. Individual game times will be calculated automatically.</p>
				</div>

				<div class="form-actions">
					<button type="submit" class="btn btn-primary" disabled={submitting || !bothPlayersSelected || player1GamesWon + player2GamesWon === 0}>
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

<Toast />

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

	.nav-link-btn {
		font-size: 0.875rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		text-decoration: none;
		color: inherit;
		opacity: 0.7;
		transition: opacity 0.2s ease;
		line-height: 1;
		background: none;
		border: none;
		cursor: pointer;
		padding: 0;
		margin: 0;
		font-family: inherit;
		appearance: none;
	}

	.nav-link-btn:hover {
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

	.player-selection {
		display: grid;
		grid-template-columns: 1fr auto 1fr;
		gap: 2rem;
		align-items: end;
		margin-bottom: 2rem;
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
		padding-bottom: 0.75rem;
		font-size: 1.25rem;
		font-weight: 200;
		letter-spacing: 0.2em;
		color: var(--text-primary);
		opacity: 0.5;
	}

	.games-section {
		margin: 2rem 0;
		padding: 1.5rem;
		border: 1px solid var(--border-subtle);
		background: transparent;
	}

	.games-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.games-header h3 {
		margin: 0;
		font-size: 1rem;
		font-weight: 300;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		color: var(--text-primary);
	}

	.score-display {
		font-size: 1.5rem;
		font-weight: 200;
		letter-spacing: 0.1em;
		color: var(--text-primary);
	}

	.games-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.game-row {
		display: grid;
		grid-template-columns: auto 1fr auto;
		gap: 1rem;
		align-items: center;
		padding: 0.75rem;
		border: 1px solid var(--border-subtle);
		transition: border-color 0.2s ease;
	}

	.game-row:hover {
		border-color: var(--border-active);
	}

	.game-number {
		font-size: 0.875rem;
		font-weight: 300;
		color: var(--text-primary);
		opacity: 0.7;
		min-width: 4rem;
	}

	.game-winners {
		display: flex;
		gap: 0.5rem;
		flex: 1;
	}

	.winner-btn {
		flex: 1;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 0.75rem;
		font-size: 0.875rem;
		font-family: inherit;
		background: transparent;
		border: 1px solid var(--border-subtle);
		color: var(--text-primary);
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.winner-btn:hover:not(:disabled) {
		border-color: var(--border-active);
		background: rgba(255, 255, 255, 0.05);
	}

	.winner-btn.selected {
		border-color: var(--border-active);
		background: rgba(255, 255, 255, 0.1);
	}

	.winner-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.checkbox {
		width: 1.25rem;
		height: 1.25rem;
		border: 1px solid var(--border-subtle);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.875rem;
		transition: all 0.2s ease;
	}

	.checkbox.checked {
		border-color: var(--border-active);
		background: var(--text-primary);
		color: var(--bg-primary);
	}

	.player-name {
		flex: 1;
		text-align: left;
		font-weight: 300;
	}

	.remove-btn {
		background: none;
		border: 1px solid var(--border-subtle);
		font-size: 1.25rem;
		cursor: pointer;
		padding: 0;
		width: 2rem;
		height: 2rem;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-primary);
		opacity: 0.5;
		transition: all 0.2s ease;
	}

	.remove-btn:hover:not(:disabled) {
		opacity: 1;
		border-color: rgba(220, 38, 38, 0.5);
	}

	.remove-btn:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.remove-btn.invisible {
		visibility: hidden;
		pointer-events: none;
	}

	.help-text {
		margin: 0.25rem 0 0.75rem;
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

		.player-selection {
			grid-template-columns: 1fr;
			gap: 1rem;
		}

		.vs-divider {
			padding: 0.5rem 0;
		}

		.games-section {
			padding: 1rem;
		}

		.game-row {
			grid-template-columns: 1fr;
			gap: 0.5rem;
		}

		.game-number {
			min-width: auto;
		}

		.game-winners {
			flex-direction: column;
		}

		.remove-btn {
			justify-self: end;
		}

		.form-actions {
			flex-direction: column;
		}

		.btn {
			width: 100%;
		}
	}
</style>
