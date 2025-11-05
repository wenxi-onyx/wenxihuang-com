<script lang="ts">
	import { onMount } from 'svelte';
	import { authStore } from '$lib/stores/auth';
	import { gamesApi, type GameWithDetails, type UpdateGameRequest } from '$lib/api/client';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import LoginButton from '$lib/components/LoginButton.svelte';
	import Toast, { showToast } from '$lib/components/Toast.svelte';
	import ConfirmModal, { confirm } from '$lib/components/ConfirmModal.svelte';

	const user = $derived($authStore.user);
	const isAdmin = $derived(user?.role === 'admin');

	let games = $state<GameWithDetails[]>([]);
	let loading = $state(true);
	let deletingGameId = $state<string | null>(null);
	let editingGameId = $state<string | null>(null);

	// Pagination state
	let currentPage = $state(1);
	let totalPages = $state(1);
	let total = $state(0);
	let limit = $state(50);

	// Edit form state
	let editPlayer1Score = $state(0);
	let editPlayer2Score = $state(0);
	let editPlayedAt = $state('');
	let saving = $state(false);

	onMount(async () => {
		await loadGames();
	});

	async function loadGames(page: number = 1) {
		try {
			loading = true;
			const response = await gamesApi.listGames(page, limit);
			games = response.games;
			currentPage = response.page;
			totalPages = response.total_pages;
			total = response.total;
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to load games', 'error');
		} finally {
			loading = false;
		}
	}

	async function handleDeleteGame(game: GameWithDetails) {
		if (!isAdmin) {
			showToast('Admin access required', 'error');
			return;
		}

		const confirmed = await confirm({
			title: 'Delete Match',
			message: `Delete match: ${game.player1_name} (${game.player1_score}) vs ${game.player2_name} (${game.player2_score})?\n\nThis will recalculate all ELOs for the season. This action cannot be undone.`,
			confirmText: 'DELETE',
			cancelText: 'CANCEL',
			confirmStyle: 'danger',
		});

		if (!confirmed) {
			return;
		}

		deletingGameId = game.id;
		try {
			await gamesApi.deleteGame(game.id);
			showToast('Match deleted successfully', 'success');
			await loadGames(currentPage);
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to delete match', 'error');
		} finally {
			deletingGameId = null;
		}
	}

	async function startEditing(game: GameWithDetails) {
		if (!isAdmin) {
			showToast('Admin access required', 'error');
			return;
		}

		// Show confirmation dialog with warning
		const confirmed = await confirm({
			title: 'Edit Match',
			message: `Edit match: ${game.player1_name} (${game.player1_score}) vs ${game.player2_name} (${game.player2_score})?\n\nWarning: This will recalculate all ELO ratings for the entire season. This may take a few seconds.`,
			confirmText: 'EDIT',
			cancelText: 'CANCEL',
			confirmStyle: 'warning',
		});

		if (!confirmed) {
			return;
		}

		editingGameId = game.id;
		editPlayer1Score = game.player1_score;
		editPlayer2Score = game.player2_score;

		// Convert played_at to datetime-local format (using UTC to avoid timezone issues)
		const date = new Date(game.played_at);
		const year = date.getUTCFullYear();
		const month = String(date.getUTCMonth() + 1).padStart(2, '0');
		const day = String(date.getUTCDate()).padStart(2, '0');
		const hours = String(date.getUTCHours()).padStart(2, '0');
		const minutes = String(date.getUTCMinutes()).padStart(2, '0');
		editPlayedAt = `${year}-${month}-${day}T${hours}:${minutes}`;
	}

	function cancelEditing() {
		editingGameId = null;
		editPlayer1Score = 0;
		editPlayer2Score = 0;
		editPlayedAt = '';
	}

	async function handleSaveEdit(game: GameWithDetails) {
		if (!isAdmin) {
			showToast('Admin access required', 'error');
			return;
		}

		// Validate scores
		if (editPlayer1Score < 0 || editPlayer2Score < 0) {
			showToast('Scores cannot be negative', 'error');
			return;
		}

		if (editPlayer1Score === editPlayer2Score) {
			showToast('Game cannot be a tie', 'error');
			return;
		}

		saving = true;
		try {
			const updateData: UpdateGameRequest = {
				player1_score: editPlayer1Score,
				player2_score: editPlayer2Score,
				played_at: editPlayedAt ? new Date(editPlayedAt).toISOString() : undefined,
			};

			await gamesApi.updateGame(game.id, updateData);
			showToast('Match updated successfully', 'success');
			cancelEditing();
			await loadGames(currentPage);
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to update match', 'error');
		} finally {
			saving = false;
		}
	}

	async function goToPage(page: number) {
		if (page < 1 || page > totalPages) return;
		await loadGames(page);
		// Scroll to top of page
		window.scrollTo({ top: 0, behavior: 'smooth' });
	}

	function formatDate(dateString: string): string {
		const date = new Date(dateString);
		return new Intl.DateTimeFormat('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: 'numeric',
			minute: '2-digit',
			hour12: true
		}).format(date);
	}

	function formatEloChange(change: number): string {
		return change >= 0 ? `+${change.toFixed(1)}` : change.toFixed(1);
	}

	function getWinner(game: GameWithDetails): 'player1' | 'player2' {
		return game.player1_score > game.player2_score ? 'player1' : 'player2';
	}
</script>

<svelte:head>
	<title>Match History - Table Tennis</title>
</svelte:head>

<ThemeToggle />
<LoginButton />
<Toast />
<ConfirmModal />

<div class="container">
	<header class="page-header">
		<h1>Match History</h1>
		<nav class="nav-links">
			<a href="/table-tennis">BACK TO LEADERBOARD</a>
		</nav>
	</header>

	{#if !loading && games.length > 0}
		<div class="pagination-info">
			Showing {games.length} of {total} matches
		</div>
	{/if}

	{#if loading}
		<div class="loading">Loading match history...</div>
	{:else if games.length === 0}
		<div class="empty-state">
			<p>No matches found.</p>
			{#if user}
				<a href="/table-tennis/add-match" class="btn">Add First Match</a>
			{/if}
		</div>
	{:else}
		<!-- Pagination Controls (Top) -->
		{#if totalPages > 1}
			<div class="pagination pagination-top">
				<button
					class="page-btn"
					onclick={() => goToPage(1)}
					disabled={currentPage === 1 || loading}
				>
					«
				</button>
				<button
					class="page-btn"
					onclick={() => goToPage(currentPage - 1)}
					disabled={currentPage === 1 || loading}
				>
					‹
				</button>

				<span class="page-info">
					Page {currentPage} of {totalPages}
				</span>

				<button
					class="page-btn"
					onclick={() => goToPage(currentPage + 1)}
					disabled={currentPage === totalPages || loading}
				>
					›
				</button>
				<button
					class="page-btn"
					onclick={() => goToPage(totalPages)}
					disabled={currentPage === totalPages || loading}
				>
					»
				</button>
			</div>
		{/if}

		<div class="games-list">
			{#each games as game}
				<div class="game-card" class:deleting={deletingGameId === game.id} class:editing={editingGameId === game.id}>
					<div class="game-header">
						<div class="season-badge">{game.season_name}</div>
						<div class="date">{formatDate(game.played_at)}</div>
						{#if isAdmin && editingGameId !== game.id}
							<div class="header-actions">
								<button
									class="btn-edit"
									onclick={() => startEditing(game)}
									disabled={deletingGameId === game.id || editingGameId !== null}
								>
									EDIT
								</button>
								<button
									class="btn-delete"
									onclick={() => handleDeleteGame(game)}
									disabled={deletingGameId === game.id || editingGameId !== null}
								>
									{deletingGameId === game.id ? 'DELETING...' : 'DELETE'}
								</button>
							</div>
						{/if}
					</div>

					{#if editingGameId === game.id}
						<!-- Edit Mode -->
						<div class="edit-form">
							<div class="edit-row">
								<div class="edit-player">
									<div class="player-name">{game.player1_name}</div>
									<input
										type="number"
										bind:value={editPlayer1Score}
										min="0"
										class="score-input"
									/>
								</div>
								<div class="vs">VS</div>
								<div class="edit-player">
									<div class="player-name">{game.player2_name}</div>
									<input
										type="number"
										bind:value={editPlayer2Score}
										min="0"
										class="score-input"
									/>
								</div>
							</div>
							<div class="edit-date">
								<label for="edit-date-{game.id}">Match Date/Time</label>
								<input
									id="edit-date-{game.id}"
									type="datetime-local"
									bind:value={editPlayedAt}
									class="date-input"
								/>
							</div>
							<div class="edit-actions">
								<button
									class="btn-save"
									onclick={() => handleSaveEdit(game)}
									disabled={saving}
								>
									{saving ? 'SAVING...' : 'SAVE'}
								</button>
								<button
									class="btn-cancel"
									onclick={cancelEditing}
									disabled={saving}
								>
									CANCEL
								</button>
							</div>
						</div>
					{:else}
						<!-- View Mode -->
						<div class="game-content">
							<div class="player" class:winner={getWinner(game) === 'player1'}>
								<div class="player-info">
									<div class="player-name">{game.player1_name}</div>
									<div class="elo-info">
										<span class="elo-before">{game.player1_elo_before.toFixed(0)}</span>
										<span class="elo-arrow">→</span>
										<span class="elo-after">{game.player1_elo_after.toFixed(0)}</span>
										<span class="elo-change" class:positive={game.player1_elo_change >= 0} class:negative={game.player1_elo_change < 0}>
											{formatEloChange(game.player1_elo_change)}
										</span>
									</div>
								</div>
								<div class="score">{game.player1_score}</div>
							</div>

							<div class="vs">VS</div>

							<div class="player" class:winner={getWinner(game) === 'player2'}>
								<div class="score">{game.player2_score}</div>
								<div class="player-info">
									<div class="player-name">{game.player2_name}</div>
									<div class="elo-info">
										<span class="elo-before">{game.player2_elo_before.toFixed(0)}</span>
										<span class="elo-arrow">→</span>
										<span class="elo-after">{game.player2_elo_after.toFixed(0)}</span>
										<span class="elo-change" class:positive={game.player2_elo_change >= 0} class:negative={game.player2_elo_change < 0}>
											{formatEloChange(game.player2_elo_change)}
										</span>
									</div>
								</div>
							</div>
						</div>
					{/if}
				</div>
			{/each}
		</div>

		<!-- Pagination Controls -->
		{#if totalPages > 1}
			<div class="pagination">
				<button
					class="page-btn"
					onclick={() => goToPage(1)}
					disabled={currentPage === 1 || loading}
				>
					«
				</button>
				<button
					class="page-btn"
					onclick={() => goToPage(currentPage - 1)}
					disabled={currentPage === 1 || loading}
				>
					‹
				</button>

				<span class="page-info">
					Page {currentPage} of {totalPages}
				</span>

				<button
					class="page-btn"
					onclick={() => goToPage(currentPage + 1)}
					disabled={currentPage === totalPages || loading}
				>
					›
				</button>
				<button
					class="page-btn"
					onclick={() => goToPage(totalPages)}
					disabled={currentPage === totalPages || loading}
				>
					»
				</button>
			</div>
		{/if}
	{/if}
</div>

<style>
	.container {
		max-width: 1200px;
		margin: 0 auto;
		padding: 6rem 2rem 4rem 2rem;
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid var(--border-subtle);
	}

	h1 {
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

	.pagination-info {
		text-align: center;
		font-size: 0.875rem;
		color: var(--text-primary);
		opacity: 0.6;
		margin-bottom: 2rem;
	}

	.loading {
		text-align: center;
		padding: 3rem;
		font-size: 1rem;
		color: var(--text-primary);
		opacity: 0.7;
	}

	.empty-state {
		text-align: center;
		padding: 3rem;
	}

	.empty-state p {
		margin-bottom: 1.5rem;
		color: var(--text-primary);
		opacity: 0.7;
	}

	.btn {
		display: inline-block;
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
	}

	.btn:hover {
		border-color: var(--border-active);
		opacity: 0.8;
	}

	.games-list {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
		margin-bottom: 2rem;
	}

	.game-card {
		background: transparent;
		border: 1px solid var(--border-subtle);
		padding: 1.5rem;
		transition: all 0.2s ease;
	}

	.game-card:hover {
		border-color: var(--border-active);
	}

	.game-card.deleting,
	.game-card.editing {
		opacity: 0.6;
	}

	.game-card.editing {
		opacity: 1;
		border-color: var(--border-active);
	}

	.game-header {
		display: grid;
		grid-template-columns: 1fr auto 1fr;
		align-items: center;
		margin-bottom: 1.5rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid var(--border-subtle);
	}

	.season-badge {
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		padding: 0.25rem 0.75rem;
		border: 1px solid var(--border-subtle);
		color: var(--text-primary);
		opacity: 0.7;
		justify-self: start;
	}

	.date {
		font-size: 0.875rem;
		font-weight: 300;
		color: var(--text-primary);
		opacity: 0.6;
		text-align: center;
		grid-column: 2;
	}

	.header-actions {
		display: flex;
		gap: 0.75rem;
		justify-self: end;
		grid-column: 3;
	}

	.game-content {
		display: grid;
		grid-template-columns: 1fr auto 1fr;
		gap: 2rem;
		align-items: center;
	}

	.player {
		display: flex;
		align-items: center;
		gap: 1.5rem;
	}

	.player.winner .player-name {
		font-weight: 400;
	}

	.player.winner .score {
		font-weight: 400;
		opacity: 1;
	}

	.player-info {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.player:last-of-type .player-info {
		text-align: right;
	}

	.player-name {
		font-size: 1.125rem;
		font-weight: 300;
		color: var(--text-primary);
	}

	.elo-info {
		font-size: 0.75rem;
		font-weight: 300;
		color: var(--text-primary);
		opacity: 0.6;
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.player:last-of-type .elo-info {
		justify-content: flex-end;
	}

	.elo-arrow {
		opacity: 0.4;
	}

	.elo-change {
		font-weight: 400;
	}

	.elo-change.positive {
		color: rgba(100, 255, 100, 0.8);
	}

	.elo-change.negative {
		color: rgba(255, 100, 100, 0.8);
	}

	.score {
		font-size: 2rem;
		font-weight: 300;
		color: var(--text-primary);
		opacity: 0.7;
		min-width: 3rem;
		text-align: center;
	}

	.vs {
		font-size: 0.875rem;
		font-weight: 200;
		letter-spacing: 0.2em;
		color: var(--text-primary);
		opacity: 0.4;
		text-align: center;
	}

	.btn-edit,
	.btn-delete,
	.btn-save,
	.btn-cancel {
		padding: 0.5rem 1rem;
		background: transparent;
		border: 1px solid var(--border-subtle);
		color: var(--text-primary);
		font-size: 0.75rem;
		font-weight: 300;
		font-family: inherit;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.btn-edit:hover:not(:disabled) {
		border-color: rgba(100, 200, 255, 0.5);
		background: rgba(100, 200, 255, 0.05);
		color: rgb(150, 220, 255);
	}

	.btn-delete:hover:not(:disabled) {
		border-color: rgba(255, 100, 100, 0.5);
		background: rgba(255, 100, 100, 0.05);
		color: rgb(255, 150, 150);
	}

	.btn-save:hover:not(:disabled) {
		border-color: rgba(100, 255, 100, 0.5);
		background: rgba(100, 255, 100, 0.05);
		color: rgb(150, 255, 150);
	}

	.btn-cancel:hover:not(:disabled) {
		border-color: var(--border-active);
		opacity: 0.8;
	}

	.btn-edit:disabled,
	.btn-delete:disabled,
	.btn-save:disabled,
	.btn-cancel:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	/* Edit Form Styles */
	.edit-form {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.edit-row {
		display: grid;
		grid-template-columns: 1fr auto 1fr;
		gap: 2rem;
		align-items: center;
	}

	.edit-player {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.score-input {
		width: 100%;
		padding: 0.75rem;
		font-size: 1.5rem;
		font-family: inherit;
		font-weight: 300;
		text-align: center;
		border: 1px solid var(--border-subtle);
		background: transparent;
		color: var(--text-primary);
		outline: none;
		transition: border-color 0.2s ease;
	}

	.score-input:focus {
		border-color: var(--border-active);
	}

	.edit-date {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.edit-date label {
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--text-primary);
		opacity: 0.7;
	}

	.date-input {
		padding: 0.75rem;
		font-size: 0.875rem;
		font-family: inherit;
		border: 1px solid var(--border-subtle);
		background: transparent;
		color: var(--text-primary);
		outline: none;
		transition: border-color 0.2s ease;
	}

	.date-input:focus {
		border-color: var(--border-active);
	}

	.edit-actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
	}

	/* Pagination Styles */
	.pagination {
		display: flex;
		justify-content: center;
		align-items: center;
		gap: 1rem;
		margin-top: 3rem;
	}

	.pagination-top {
		margin-top: 1rem;
		margin-bottom: 2rem;
	}

	.page-btn {
		padding: 0.5rem 1rem;
		background: transparent;
		border: 1px solid var(--border-subtle);
		color: var(--text-primary);
		font-size: 0.875rem;
		font-weight: 300;
		font-family: inherit;
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.page-btn:hover:not(:disabled) {
		border-color: var(--border-active);
		background: var(--border-subtle);
	}

	.page-btn:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.page-info {
		font-size: 0.875rem;
		font-weight: 300;
		color: var(--text-primary);
		opacity: 0.7;
		min-width: 120px;
		text-align: center;
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

		.game-content,
		.edit-row {
			grid-template-columns: 1fr;
			gap: 1.5rem;
		}

		.player {
			flex-direction: row;
			justify-content: space-between;
		}

		.player:last-of-type {
			flex-direction: row-reverse;
		}

		.player:last-of-type .player-info {
			text-align: left;
		}

		.player:last-of-type .elo-info {
			justify-content: flex-start;
		}

		.vs {
			display: none;
		}

		.score {
			font-size: 1.5rem;
		}

		.header-actions,
		.edit-actions {
			flex-direction: column;
		}

		.btn-edit,
		.btn-delete,
		.btn-save,
		.btn-cancel {
			width: 100%;
		}

		.pagination {
			flex-wrap: wrap;
		}
	}
</style>
