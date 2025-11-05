<script lang="ts">
	import { onMount } from 'svelte';
	import { authStore } from '$lib/stores/auth';
	import { matchesApi, type MatchWithDetails } from '$lib/api/client';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import LoginButton from '$lib/components/LoginButton.svelte';
	import Toast, { showToast } from '$lib/components/Toast.svelte';
	import ConfirmModal, { confirm } from '$lib/components/ConfirmModal.svelte';

	const user = $derived($authStore.user);
	const isAdmin = $derived(user?.role === 'admin');

	let matches = $state<MatchWithDetails[]>([]);
	let loading = $state(true);
	let deletingMatchId = $state<string | null>(null);

	// Track which matches are expanded
	let expandedMatches = $state<Set<string>>(new Set());

	// Pagination state
	let currentPage = $state(1);
	let totalPages = $state(1);
	let total = $state(0);
	let limit = $state(50);

	onMount(async () => {
		await loadMatches();
	});

	async function loadMatches(page: number = 1) {
		try {
			loading = true;
			const response = await matchesApi.listMatches(page, limit);
			matches = response.matches;
			currentPage = response.page;
			totalPages = response.total_pages;
			total = response.total;
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to load matches', 'error');
		} finally {
			loading = false;
		}
	}

	function toggleExpanded(matchId: string) {
		if (expandedMatches.has(matchId)) {
			expandedMatches.delete(matchId);
		} else {
			expandedMatches.add(matchId);
		}
		expandedMatches = new Set(expandedMatches); // Trigger reactivity
	}

	async function handleDeleteMatch(match: MatchWithDetails) {
		if (!isAdmin) {
			showToast('Admin access required', 'error');
			return;
		}

		const confirmed = await confirm({
			title: 'Delete Match',
			message: `Delete match: ${match.player1_name} (${match.player1_games_won}) vs ${match.player2_name} (${match.player2_games_won})?\n\nThis will delete all ${match.total_games} game(s) and recalculate ELOs for the season. This action cannot be undone.`,
			confirmText: 'DELETE',
			cancelText: 'CANCEL',
			confirmStyle: 'danger',
		});

		if (!confirmed) {
			return;
		}

		deletingMatchId = match.id;
		try {
			await matchesApi.deleteMatch(match.id);
			showToast('Match deleted successfully', 'success');
			await loadMatches(currentPage);
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to delete match', 'error');
		} finally {
			deletingMatchId = null;
		}
	}

	async function goToPage(page: number) {
		if (page < 1 || page > totalPages) return;
		await loadMatches(page);
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

	function getMatchWinner(match: MatchWithDetails): 'player1' | 'player2' | 'tie' {
		if (match.player1_games_won > match.player2_games_won) return 'player1';
		if (match.player2_games_won > match.player1_games_won) return 'player2';
		return 'tie';
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
			{#if user}
				<a href="/table-tennis/add-match" class="btn-add">ADD MATCH</a>
			{/if}
		</nav>
	</header>

	{#if !loading && matches.length > 0}
		<div class="pagination-info">
			Showing {matches.length} of {total} matches
		</div>
	{/if}

	{#if loading}
		<div class="loading">Loading match history...</div>
	{:else if matches.length === 0}
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

		<div class="matches-list">
			{#each matches as match}
				<div class="match-card" class:deleting={deletingMatchId === match.id}>
					<div class="match-header">
						<div class="season-badge">{match.season_name}</div>
						<div class="date">{formatDate(match.submitted_at)}</div>
						{#if isAdmin}
							<div class="header-actions">
								<button
									class="btn-delete"
									onclick={() => handleDeleteMatch(match)}
									disabled={deletingMatchId === match.id}
								>
									{deletingMatchId === match.id ? 'DELETING...' : 'DELETE'}
								</button>
							</div>
						{/if}
					</div>

					<div class="match-summary">
						<div class="player" class:winner={getMatchWinner(match) === 'player1'}>
							<div class="player-info">
								<div class="player-name">{match.player1_name}</div>
								<div class="elo-info">
									<span class="elo-before">{match.player1_elo_before.toFixed(0)}</span>
									<span class="elo-arrow">→</span>
									<span class="elo-after">{match.player1_elo_after.toFixed(0)}</span>
									<span class="elo-change" class:positive={match.player1_elo_change >= 0} class:negative={match.player1_elo_change < 0}>
										{formatEloChange(match.player1_elo_change)}
									</span>
								</div>
							</div>
							<div class="match-score">{match.player1_games_won}</div>
						</div>

						<div class="vs">VS</div>

						<div class="player" class:winner={getMatchWinner(match) === 'player2'}>
							<div class="match-score">{match.player2_games_won}</div>
							<div class="player-info">
								<div class="player-name">{match.player2_name}</div>
								<div class="elo-info">
									<span class="elo-before">{match.player2_elo_before.toFixed(0)}</span>
									<span class="elo-arrow">→</span>
									<span class="elo-after">{match.player2_elo_after.toFixed(0)}</span>
									<span class="elo-change" class:positive={match.player2_elo_change >= 0} class:negative={match.player2_elo_change < 0}>
										{formatEloChange(match.player2_elo_change)}
									</span>
								</div>
							</div>
						</div>
					</div>

					<button
						class="expand-btn"
						onclick={() => toggleExpanded(match.id)}
						type="button"
					>
						{expandedMatches.has(match.id) ? '▲' : '▼'}
						{expandedMatches.has(match.id) ? 'Hide' : 'Show'} {match.total_games} Game{match.total_games !== 1 ? 's' : ''}
					</button>

					{#if expandedMatches.has(match.id)}
						<div class="games-details">
							<div class="games-header">
								<span class="game-col-label">Game</span>
								<span class="game-col-label">Winner</span>
								<span class="game-col-label">{match.player1_name} ELO</span>
								<span class="game-col-label">{match.player2_name} ELO</span>
							</div>
							{#each match.games as game, index}
								<div class="game-detail">
									<div class="game-number">#{index + 1}</div>
									<div class="game-winner">
										{game.winner === 'Player1' ? match.player1_name : match.player2_name}
									</div>
									<div class="game-elo">
										<span class="elo-value">{game.player1_elo_before.toFixed(0)}</span>
										<span class="elo-arrow-small">→</span>
										<span class="elo-value">{game.player1_elo_after.toFixed(0)}</span>
										<span class="elo-change-small" class:positive={game.player1_elo_change >= 0} class:negative={game.player1_elo_change < 0}>
											{formatEloChange(game.player1_elo_change)}
										</span>
									</div>
									<div class="game-elo">
										<span class="elo-value">{game.player2_elo_before.toFixed(0)}</span>
										<span class="elo-arrow-small">→</span>
										<span class="elo-value">{game.player2_elo_after.toFixed(0)}</span>
										<span class="elo-change-small" class:positive={game.player2_elo_change >= 0} class:negative={game.player2_elo_change < 0}>
											{formatEloChange(game.player2_elo_change)}
										</span>
									</div>
								</div>
							{/each}
						</div>
					{/if}
				</div>
			{/each}
		</div>

		<!-- Pagination Controls (Bottom) -->
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

	.btn-add {
		display: inline-block;
		padding: 0.5rem 1.5rem;
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
		margin-left: 1rem;
	}

	.btn-add:hover {
		border-color: var(--border-active);
		background: rgba(255, 255, 255, 0.05);
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

	.matches-list {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
		margin-bottom: 2rem;
	}

	.match-card {
		background: transparent;
		border: 1px solid var(--border-subtle);
		padding: 1.5rem;
		transition: all 0.2s ease;
	}

	.match-card:hover {
		border-color: var(--border-active);
	}

	.match-card.deleting {
		opacity: 0.6;
	}

	.match-header {
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
		max-width: 250px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
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

	.match-summary {
		display: grid;
		grid-template-columns: 1fr auto 1fr;
		gap: 2rem;
		align-items: center;
		margin-bottom: 1rem;
	}

	.player {
		display: flex;
		align-items: center;
		gap: 1.5rem;
	}

	.player.winner .player-name {
		font-weight: 400;
	}

	.player.winner .match-score {
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

	.match-score {
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

	.btn-delete {
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

	.btn-delete:hover:not(:disabled) {
		border-color: rgba(255, 100, 100, 0.5);
		background: rgba(255, 100, 100, 0.05);
		color: rgb(255, 150, 150);
	}

	.btn-delete:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.expand-btn {
		width: 100%;
		padding: 0.75rem;
		background: transparent;
		border: 1px solid var(--border-subtle);
		border-top: none;
		color: var(--text-primary);
		font-size: 0.75rem;
		font-weight: 300;
		font-family: inherit;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		cursor: pointer;
		transition: all 0.2s ease;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
	}

	.expand-btn:hover {
		border-color: var(--border-active);
		background: rgba(255, 255, 255, 0.02);
	}

	.games-details {
		margin-top: 1rem;
		padding-top: 1rem;
		border-top: 1px solid var(--border-subtle);
	}

	.games-header {
		display: grid;
		grid-template-columns: 80px 1fr 200px 200px;
		gap: 1rem;
		padding: 0.75rem 1rem;
		background: rgba(255, 255, 255, 0.02);
		border-bottom: 1px solid var(--border-subtle);
	}

	.game-col-label {
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--text-primary);
		opacity: 0.5;
	}

	.game-detail {
		display: grid;
		grid-template-columns: 80px 1fr 200px 200px;
		gap: 1rem;
		padding: 0.75rem 1rem;
		align-items: center;
		border-bottom: 1px solid var(--border-subtle);
	}

	.game-detail:last-child {
		border-bottom: none;
	}

	.game-number {
		font-size: 0.875rem;
		font-weight: 300;
		color: var(--text-primary);
		opacity: 0.6;
	}

	.game-winner {
		font-size: 0.875rem;
		font-weight: 300;
		color: var(--text-primary);
	}

	.game-elo {
		font-size: 0.75rem;
		font-weight: 300;
		color: var(--text-primary);
		opacity: 0.7;
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}

	.elo-value {
		min-width: 3rem;
	}

	.elo-arrow-small {
		opacity: 0.3;
	}

	.elo-change-small {
		font-weight: 400;
		min-width: 3rem;
	}

	.elo-change-small.positive {
		color: rgba(100, 255, 100, 0.8);
	}

	.elo-change-small.negative {
		color: rgba(255, 100, 100, 0.8);
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

		.match-summary {
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

		.match-score {
			font-size: 1.5rem;
		}

		.header-actions {
			flex-direction: column;
		}

		.season-badge {
			max-width: 150px;
			font-size: 0.65rem;
		}

		.btn-delete {
			width: 100%;
		}

		.games-header,
		.game-detail {
			grid-template-columns: 60px 1fr;
		}

		.game-col-label:nth-child(3),
		.game-col-label:nth-child(4),
		.game-elo {
			display: none;
		}

		.pagination {
			flex-wrap: wrap;
		}
	}
</style>
