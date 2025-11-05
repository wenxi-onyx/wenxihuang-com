<script lang="ts">
	import { onMount } from 'svelte';
	import { authStore } from '$lib/stores/auth';
	import { playersApi, seasonsApi, type PlayerWithStats, type Season, type PlayerSeasonStats } from '$lib/api/client';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import LoginButton from '$lib/components/LoginButton.svelte';

	const user = $derived($authStore.user);

	let players = $state<PlayerWithStats[]>([]);
	let filteredPlayers = $state<PlayerWithStats[]>([]);
	let seasons = $state<Season[]>([]);
	let selectedSeason = $state<Season | null>(null);
	let loading = $state(true);
	let loadingPlayers = $state(false); // Track player data loading separately
	let error = $state('');
	let searchQuery = $state('');
	let sortField = $state<'rank' | 'name' | 'current_elo' | 'games_played' | 'wins' | 'losses' | 'win_rate'>('current_elo');
	let sortDirection = $state<'asc' | 'desc'>('desc');
	let abortController: AbortController | null = null; // For canceling in-flight requests

	async function loadSeasons() {
		try {
			seasons = await seasonsApi.listSeasons();
			// Try to get active season first
			const activeSeason = await seasonsApi.getActiveSeason();
			if (activeSeason) {
				// Find the matching season in the array by ID
				selectedSeason = seasons.find(s => s.id === activeSeason.id) || seasons[0] || null;
			} else {
				selectedSeason = seasons[0] || null;
			}
		} catch (e) {
			console.error('Failed to load seasons:', e);
			// Continue without seasons - will fall back to all-time
		}
	}

	async function loadPlayers() {
		// Cancel any in-flight request
		if (abortController) {
			abortController.abort();
		}
		abortController = new AbortController();
		const currentController = abortController;

		try {
			loadingPlayers = true;
			error = '';

			if (selectedSeason) {
				// Load season-specific leaderboard
				const seasonPlayers = await seasonsApi.getSeasonLeaderboard(selectedSeason.id);

				// Check if this request was aborted
				if (currentController.signal.aborted) {
					return;
				}

				players = seasonPlayers.map(sp => ({
					id: sp.player_id,
					name: sp.player_name,
					current_elo: sp.current_elo,
					games_played: sp.games_played,
					wins: sp.wins,
					losses: sp.losses,
					is_active: sp.is_active,
					created_at: '', // Not needed for display
					updated_at: ''  // Not needed for display
				}));
			} else {
				// Load all-time stats
				const allPlayers = await playersApi.listPlayers();

				// Check if this request was aborted
				if (currentController.signal.aborted) {
					return;
				}

				players = allPlayers;
			}

			filteredPlayers = players;
			loading = false;
			loadingPlayers = false;
		} catch (e) {
			// Don't show error if request was aborted (user switched seasons)
			if (currentController.signal.aborted) {
				return;
			}
			error = e instanceof Error ? e.message : 'Failed to load players';
			loading = false;
			loadingPlayers = false;
		}
	}

	let previousSeasonId: string | null = null; // Non-reactive tracker
	let initialized = false; // Track if initial load is complete

	onMount(async () => {
		await loadSeasons();
		await loadPlayers();
		// After initial load, track the season to detect future changes
		if (selectedSeason) {
			previousSeasonId = selectedSeason.id;
		}
		initialized = true;
	});

	// Reload players when season changes (but not on initial load)
	$effect(() => {
		if (initialized && selectedSeason !== null) {
			const currentSeasonId = selectedSeason.id;
			if (previousSeasonId !== currentSeasonId) {
				loadPlayers();
				previousSeasonId = currentSeasonId;
			}
		}
	});

	// Filter and sort players reactively
	$effect(() => {
		// Filter players based on search query
		let filtered = players.filter(player =>
			player.name.toLowerCase().includes(searchQuery.toLowerCase())
		);

		// Sort players
		filtered = [...filtered].sort((a, b) => {
			let aValue: number | string;
			let bValue: number | string;

			if (sortField === 'win_rate') {
				aValue = a.games_played > 0 ? (a.wins / a.games_played) * 100 : 0;
				bValue = b.games_played > 0 ? (b.wins / b.games_played) * 100 : 0;
			} else if (sortField === 'rank') {
				// Rank is based on ELO (inverse sort)
				aValue = a.current_elo;
				bValue = b.current_elo;
				return sortDirection === 'desc' ? bValue - aValue : aValue - bValue;
			} else if (sortField === 'name') {
				aValue = a.name;
				bValue = b.name;
				return sortDirection === 'asc'
					? aValue.localeCompare(bValue)
					: bValue.localeCompare(aValue);
			} else {
				aValue = a[sortField];
				bValue = b[sortField];
			}

			if (typeof aValue === 'number' && typeof bValue === 'number') {
				return sortDirection === 'asc' ? aValue - bValue : bValue - aValue;
			}
			return 0;
		});

		filteredPlayers = filtered;
	});

	function handleSort(field: typeof sortField) {
		if (sortField === field) {
			sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
		} else {
			sortField = field;
			sortDirection = field === 'name' ? 'asc' : 'desc';
		}
	}

	function getWinRate(player: PlayerWithStats): string {
		if (player.games_played === 0) return '0.0';
		return ((player.wins / player.games_played) * 100).toFixed(1);
	}
</script>

<svelte:head>
	<title>ELO Leaderboard</title>
</svelte:head>

<ThemeToggle />
<LoginButton />

<div class="container">
	<header class="page-header">
		<h1>Table Tennis Leaderboard</h1>
		<nav class="nav-links">
			{#if user}
				<a href="/table-tennis/add-match">ADD MATCH</a>
			{/if}
			<a href="/table-tennis/matches">MATCH HISTORY</a>
			{#if user?.role === 'admin'}
				<a href="/table-tennis/seasons">MANAGE SEASONS</a>
			{/if}
			<a href="/">BACK</a>
		</nav>
	</header>

	{#if loading}
		<div class="loading">Loading players...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else}
		{#if loadingPlayers}
			<div class="loading-overlay">
				<div class="loading-message">Loading players...</div>
			</div>
		{/if}
		<div class="controls">
			{#if seasons.length > 0}
				<select
					bind:value={selectedSeason}
					class="season-selector"
				>
					{#each seasons as season}
						<option value={season}>
							{season.name}{season.is_active ? ' (Active)' : ''}
						</option>
					{/each}
				</select>
			{/if}
			<input
				type="text"
				placeholder="Search players..."
				bind:value={searchQuery}
				class="search-input"
			/>
			<div class="stats-summary">
				<span class="stat">
					<strong>{filteredPlayers.length}</strong> {filteredPlayers.length === 1 ? 'player' : 'players'}
				</span>
			</div>
		</div>

		<div class="table-wrapper">
			<table class="leaderboard-table">
				<thead>
					<tr>
						<th class="sortable" onclick={() => handleSort('rank')}>
							Rank {sortField === 'rank' ? (sortDirection === 'desc' ? '▼' : '▲') : ''}
						</th>
						<th class="sortable" onclick={() => handleSort('name')}>
							Player {sortField === 'name' ? (sortDirection === 'desc' ? '▼' : '▲') : ''}
						</th>
						<th class="sortable" onclick={() => handleSort('current_elo')}>
							ELO Rating {sortField === 'current_elo' ? (sortDirection === 'desc' ? '▼' : '▲') : ''}
						</th>
						<th class="sortable" onclick={() => handleSort('games_played')}>
							Games {sortField === 'games_played' ? (sortDirection === 'desc' ? '▼' : '▲') : ''}
						</th>
						<th class="sortable" onclick={() => handleSort('wins')}>
							Wins {sortField === 'wins' ? (sortDirection === 'desc' ? '▼' : '▲') : ''}
						</th>
						<th class="sortable" onclick={() => handleSort('losses')}>
							Losses {sortField === 'losses' ? (sortDirection === 'desc' ? '▼' : '▲') : ''}
						</th>
						<th class="sortable" onclick={() => handleSort('win_rate')}>
							Win Rate {sortField === 'win_rate' ? (sortDirection === 'desc' ? '▼' : '▲') : ''}
						</th>
						<th>Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each filteredPlayers as player, index}
						<tr class:inactive={!player.is_active}>
							<td class="rank-cell">
								{#if sortField === 'rank' || sortField === 'current_elo'}
									<span class="rank">#{index + 1}</span>
								{:else}
									-
								{/if}
							</td>
							<td class="name-cell">
								<span class="player-name">{player.name}</span>
								{#if !player.is_active}
									<span class="inactive-badge">Inactive</span>
								{/if}
							</td>
							<td class="elo-cell">
								<span class="elo-value">{player.current_elo.toFixed(1)}</span>
							</td>
							<td>{player.games_played}</td>
							<td class="wins">{player.wins}</td>
							<td class="losses">{player.losses}</td>
							<td class="win-rate">{getWinRate(player)}%</td>
							<td>
								<a href="/table-tennis/players/{player.id}" class="btn-view">View History</a>
							</td>
						</tr>
					{:else}
						<tr>
							<td colspan="8" class="no-results">No players found</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>

<style>
	.container {
		max-width: 1400px;
		margin: 0 auto;
		padding: 6rem 2rem 4rem 2rem;
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 3rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.1);
	}

	.page-header h1 {
		font-size: clamp(1.5rem, 4vw, 2.5rem);
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		margin: 0;
		color: var(--text-primary);
	}

	.nav-links {
		display: flex;
		gap: 2rem;
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

	.loading, .error {
		text-align: center;
		padding: 3rem;
		font-size: 1rem;
		color: var(--text-primary);
	}

	.error {
		opacity: 0.8;
	}

	.loading-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: var(--bg-primary, #ffffff);
		opacity: 0.9;
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	:global([data-theme='dark']) .loading-overlay {
		background: #0a0a0a;
	}

	.loading-message {
		font-size: 1rem;
		color: var(--text-primary);
		padding: 2rem;
		border: 1px solid var(--border-subtle);
		background: var(--bg-primary, #ffffff);
	}

	:global([data-theme='dark']) .loading-message {
		background: #1a1a1a;
	}

	.controls {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 2rem;
		gap: 1rem;
		flex-wrap: wrap;
	}

	.search-input {
		flex: 1;
		min-width: 250px;
		padding: 0.75rem 1rem;
		font-size: 1rem;
		font-family: inherit;
		background: transparent;
		color: var(--text-primary);
		border: 1px solid var(--border-subtle);
		outline: none;
		transition: border-color 0.2s ease;
	}

	.search-input:focus {
		border-color: var(--border-active);
	}

	.search-input::placeholder {
		color: var(--text-primary);
		opacity: 0.3;
	}

	.season-selector {
		min-width: 200px;
		padding: 0.75rem 1rem;
		font-size: 0.875rem;
		font-family: inherit;
		font-weight: 300;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		background: transparent;
		color: var(--text-primary);
		border: 1px solid var(--border-subtle);
		outline: none;
		transition: border-color 0.2s ease;
		cursor: pointer;
	}

	:global([data-theme='dark']) .season-selector {
		font-weight: 400;
	}

	:global([data-theme='light']) .season-selector {
		font-weight: 200;
	}

	.season-selector:focus {
		border-color: var(--border-active);
	}

	.season-selector option {
		background: var(--bg-primary, #ffffff);
		color: var(--text-primary);
	}

	:global([data-theme='dark']) .season-selector option {
		background: #1a1a1a;
	}

	.stats-summary {
		display: flex;
		gap: 2rem;
	}

	.stat {
		font-size: 0.875rem;
		color: var(--text-primary);
		opacity: 0.7;
	}

	.table-wrapper {
		overflow-x: auto;
		border: 1px solid var(--border-subtle);
		background: transparent;
	}

	.leaderboard-table {
		width: 100%;
		border-collapse: collapse;
	}

	.leaderboard-table thead {
		background: transparent;
		border-bottom: 1px solid var(--border-subtle);
	}

	.leaderboard-table th {
		padding: 1rem;
		text-align: left;
		font-weight: 300;
		color: var(--text-primary);
		font-size: 0.75rem;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		opacity: 0.8;
	}

	:global([data-theme='dark']) .leaderboard-table th {
		font-weight: 500;
	}

	:global([data-theme='light']) .leaderboard-table th {
		font-weight: 200;
	}

	.leaderboard-table th.sortable {
		cursor: pointer;
		user-select: none;
		transition: opacity 0.2s;
	}

	.leaderboard-table th.sortable:hover {
		opacity: 1;
	}

	.leaderboard-table tbody tr {
		border-bottom: 1px solid var(--border-subtle);
		transition: opacity 0.15s;
	}

	.leaderboard-table tbody tr:hover {
		opacity: 0.8;
	}

	.leaderboard-table tbody tr.inactive {
		opacity: 0.5;
	}

	.leaderboard-table td {
		padding: 1rem;
		font-size: 0.875rem;
		color: var(--text-primary);
	}

	.rank-cell {
		font-weight: 300;
	}

	.rank {
		display: inline-block;
		min-width: 2rem;
		text-align: center;
	}

	.name-cell {
		font-weight: 300;
	}

	:global([data-theme='dark']) .name-cell {
		font-weight: 400;
	}

	.player-name {
		margin-right: 0.5rem;
	}

	.inactive-badge {
		display: inline-block;
		padding: 0.125rem 0.5rem;
		font-size: 0.625rem;
		border: 1px solid var(--border-subtle);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-weight: 300;
		opacity: 0.6;
	}

	:global([data-theme='light']) .inactive-badge {
		font-weight: 200;
	}

	.elo-cell {
		font-weight: 300;
	}

	:global([data-theme='dark']) .elo-cell {
		font-weight: 500;
	}

	.elo-value {
		font-size: 1rem;
	}

	.wins {
		font-weight: 300;
	}

	.losses {
		font-weight: 300;
	}

	.win-rate {
		font-weight: 300;
	}

	.btn-view {
		display: inline-block;
		padding: 0.5rem 1rem;
		font-size: 0.75rem;
		font-weight: 300;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		color: var(--text-primary);
		text-decoration: underline;
		text-decoration-thickness: 0.5px;
		border: none;
		background: transparent;
		transition: opacity 0.3s ease;
	}

	:global([data-theme='light']) .btn-view {
		font-weight: 200;
	}

	.btn-view:hover {
		opacity: 0.6;
	}

	.no-results {
		text-align: center;
		padding: 3rem !important;
		color: var(--text-primary);
		opacity: 0.6;
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

		.controls {
			flex-direction: column;
			align-items: stretch;
		}

		.search-input {
			min-width: 100%;
		}

		.leaderboard-table {
			font-size: 0.8rem;
		}

		.leaderboard-table th,
		.leaderboard-table td {
			padding: 0.75rem 0.5rem;
		}

		/* Hide less important columns on mobile */
		.leaderboard-table th:nth-child(4),
		.leaderboard-table td:nth-child(4),
		.leaderboard-table th:nth-child(6),
		.leaderboard-table td:nth-child(6) {
			display: none;
		}
	}
</style>
