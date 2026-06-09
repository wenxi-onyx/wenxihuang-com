<script lang="ts" module>
	import { writable, get } from 'svelte/store';

	type AddMatchModalState = {
		isOpen: boolean;
		onSuccess?: () => void | Promise<void>;
		userName?: string;
	};

	const modalStore = writable<AddMatchModalState>({ isOpen: false });

	export function openAddMatchModal(onSuccess?: () => void | Promise<void>, userName?: string) {
		modalStore.set({ isOpen: true, onSuccess, userName });
	}

	export function closeAddMatchModal() {
		modalStore.set({ isOpen: false });
	}
</script>

<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { matchesApi, seasonsApi, type ActiveSeasonPlayer, type Season, type GameWinner } from '$lib/api/client';
	import { showToast } from '$lib/components/Toast.svelte';

	let modalState = $derived($modalStore);

	let players = $state<ActiveSeasonPlayer[]>([]);
	let activeSeason = $state<Season | null>(null);
	let loading = $state(false);
	let submitting = $state(false);

	// Track loading to cancel if modal closes
	let loadAbortController: AbortController | null = null;

	// Form state
	let player1Id = $state('');
	let player2Id = $state('');

	// Default to current local time rounded down to nearest 5-minute interval
	function getDefaultDateTime(): string {
		const now = new Date();
		// Round down to nearest 5-minute interval
		const minutes = now.getMinutes();
		const roundedMinutes = Math.floor(minutes / 5) * 5;
		now.setMinutes(roundedMinutes, 0, 0); // Set minutes, seconds, and milliseconds in one call

		// Format as YYYY-MM-DDTHH:MM for datetime-local input
		const year = now.getFullYear();
		const month = String(now.getMonth() + 1).padStart(2, '0');
		const day = String(now.getDate()).padStart(2, '0');
		const hours = String(now.getHours()).padStart(2, '0');
		const formattedMinutes = String(roundedMinutes).padStart(2, '0');
		return `${year}-${month}-${day}T${hours}:${formattedMinutes}`;
	}

	let submittedAt = $state(getDefaultDateTime());

	// Games state - track winner for each game
	let games = $state<(GameWinner | null)[]>([null, null, null, null, null]); // Start with 5 empty games
	const MAX_GAMES = 11; // Reasonable limit for a match

	// Reset form state
	function resetForm() {
		player1Id = '';
		player2Id = '';
		games = [null, null, null, null, null];
		submittedAt = getDefaultDateTime();
	}

	// Derived score
	let player1GamesWon = $derived(games.filter(g => g === 'Player1').length);
	let player2GamesWon = $derived(games.filter(g => g === 'Player2').length);
	let score = $derived(`${player1GamesWon}-${player2GamesWon}`);

	// Check if players are available for better UX
	let playersAvailable = $derived(players.length > 0);
	let bothPlayersSelected = $derived(player1Id && player2Id && player1Id !== player2Id);

	// Load data when modal opens, cleanup when it closes
	$effect(() => {
		if (modalState.isOpen) {
			// Reset form when modal opens
			resetForm();

			// Lock body scroll when modal opens
			document.body.style.overflow = 'hidden';

			// Always reload data to ensure freshness
			loadData();
		} else {
			// Cancel any in-flight load when modal closes
			if (loadAbortController) {
				loadAbortController.abort();
				loadAbortController = null;
			}

			// Restore body scroll when modal closes
			document.body.style.overflow = '';
		}
	});

	async function loadData() {
		// Cancel previous load if any
		if (loadAbortController) {
			loadAbortController.abort();
		}

		loadAbortController = new AbortController();
		const currentController = loadAbortController;

		try {
			loading = true;

			// Load active season and its players in parallel
			const [season, seasonPlayers] = await Promise.all([
				seasonsApi.getActiveSeason(),
				seasonsApi.getActiveSeasonPlayers()
			]);

			// Check if this load was cancelled
			if (currentController.signal.aborted) {
				return;
			}

			activeSeason = season;
			players = seasonPlayers;

			// Auto-populate player1 if user's last name matches exactly one player
			if (modalState.userName) {
				autoPopulatePlayer1(modalState.userName, seasonPlayers);
			}

			loading = false;
		} catch (e) {
			// Don't show error if request was cancelled
			if (currentController.signal.aborted) {
				return;
			}
			showToast(e instanceof Error ? e.message : 'Failed to load data', 'error');
			loading = false;
		}
	}

	function autoPopulatePlayer1(userName: string, playerList: ActiveSeasonPlayer[]) {
		// Extract last name (last word after splitting by space)
		const nameParts = userName.trim().split(/\s+/).filter(part => part.length > 0);
		if (nameParts.length === 0) return;

		const userLastName = nameParts[nameParts.length - 1].toLowerCase();
		if (!userLastName) return;

		// Find players whose last name matches
		const matchingPlayers = playerList.filter(player => {
			const playerNameParts = player.name.trim().split(/\s+/).filter(part => part.length > 0);
			if (playerNameParts.length === 0) return false;

			const playerLastName = playerNameParts[playerNameParts.length - 1].toLowerCase();
			return playerLastName === userLastName;
		});

		// Only auto-select if exactly one player matches
		if (matchingPlayers.length === 1) {
			player1Id = matchingPlayers[0].id;
		}
	}

	async function handleGameWinner(gameIndex: number, winner: GameWinner) {
		// If clicking the same winner again, uncheck it
		if (games[gameIndex] === winner) {
			games[gameIndex] = null;
			return;
		}

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

	async function handleSubmit(e: SubmitEvent) {
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

			// Save callback before closing modal (closing clears the state)
			const callback = modalState.onSuccess;

			// Close modal
			closeAddMatchModal();

			// Call onSuccess callback (which will show toast and refresh data on parent page)
			if (callback) {
				await callback();
			}
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

	function handleClose() {
		if (!submitting) {
			closeAddMatchModal();
		}
	}

	onMount(() => {
		function handleEscape(e: KeyboardEvent) {
			const currentModal = get(modalStore);
			if (e.key === 'Escape' && currentModal.isOpen && !submitting) {
				handleClose();
			}
		}

		document.addEventListener('keydown', handleEscape);
		return () => document.removeEventListener('keydown', handleEscape);
	});
</script>

{#if modalState.isOpen}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		class="modal-backdrop"
		onclick={handleClose}
		role="presentation"
	>
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<div
			class="modal modal-lg"
			onclick={(e) => e.stopPropagation()}
			role="dialog"
			tabindex="-1"
			aria-modal="true"
			aria-labelledby="modal-title"
		>
			<div class="modal-header">
				<h2 id="modal-title">Add Match Result</h2>
				<button class="modal-close" onclick={handleClose} disabled={submitting}>×</button>
			</div>

			<div class="modal-body">
				{#if loading}
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

						<div class="modal-actions">
							<button
								type="button"
								class="btn btn-cancel"
								onclick={handleClose}
								disabled={submitting}
							>
								Cancel
							</button>
							<button type="submit" class="btn-primary" disabled={submitting || !bothPlayersSelected || player1GamesWon + player2GamesWon === 0}>
								{submitting ? 'Recording...' : 'Record Match'}
							</button>
						</div>
					</form>
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	/* Using shared styles: modals.css (.modal-backdrop, .modal, .modal-lg, .modal-header, .modal-close, .modal-body, .modal-actions), forms.css (.form-group, label, input, select), buttons.css (.btn, .btn-primary, .btn-cancel), animations.css (fadeIn, slideUp), utilities.css (.loading) */

	.modal-backdrop {
		overflow-y: auto;
		padding: 2rem 0;
	}

	.modal-header {
		position: sticky;
		top: 0;
		background: var(--bg-primary);
		z-index: 10;
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

	@media (max-width: 768px) {
		.modal {
			max-width: none;
			width: 100%;
			border-radius: 0;
		}

		.modal-body {
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

	}
</style>
