<script lang="ts">
	import { onMount } from 'svelte';
	import { playersApi, type PlayerWithStats } from '$lib/api/client';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import LoginButton from '$lib/components/LoginButton.svelte';

	let players: PlayerWithStats[] = [];
	let filteredPlayers: PlayerWithStats[] = [];
	let loading = true;
	let error = '';
	let searchQuery = '';
	let sortField: 'rank' | 'name' | 'current_elo' | 'games_played' | 'wins' | 'losses' | 'win_rate' = 'current_elo';
	let sortDirection: 'asc' | 'desc' = 'desc';

	onMount(async () => {
		try {
			players = await playersApi.listPlayers();
			filteredPlayers = players;
			loading = false;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load players';
			loading = false;
		}
	});

	$: {
		// Filter players based on search query
		filteredPlayers = players.filter(player =>
			player.name.toLowerCase().includes(searchQuery.toLowerCase())
		);

		// Sort players
		filteredPlayers = [...filteredPlayers].sort((a, b) => {
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
	}

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
		<p class="subtitle">Track player rankings and statistics</p>
	</header>

	{#if loading}
		<div class="loading">Loading players...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else}
		<div class="controls">
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
						<th class="sortable" on:click={() => handleSort('rank')}>
							Rank {sortField === 'rank' ? (sortDirection === 'desc' ? '▼' : '▲') : ''}
						</th>
						<th class="sortable" on:click={() => handleSort('name')}>
							Player {sortField === 'name' ? (sortDirection === 'desc' ? '▼' : '▲') : ''}
						</th>
						<th class="sortable" on:click={() => handleSort('current_elo')}>
							ELO Rating {sortField === 'current_elo' ? (sortDirection === 'desc' ? '▼' : '▲') : ''}
						</th>
						<th class="sortable" on:click={() => handleSort('games_played')}>
							Games {sortField === 'games_played' ? (sortDirection === 'desc' ? '▼' : '▲') : ''}
						</th>
						<th class="sortable" on:click={() => handleSort('wins')}>
							Wins {sortField === 'wins' ? (sortDirection === 'desc' ? '▼' : '▲') : ''}
						</th>
						<th class="sortable" on:click={() => handleSort('losses')}>
							Losses {sortField === 'losses' ? (sortDirection === 'desc' ? '▼' : '▲') : ''}
						</th>
						<th class="sortable" on:click={() => handleSort('win_rate')}>
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
		padding: 4rem 2rem;
	}

	.page-header {
		text-align: center;
		margin-bottom: 3rem;
	}

	.page-header h1 {
		font-size: clamp(1.5rem, 4vw, 2rem);
		font-weight: 300;
		letter-spacing: 0.15em;
		margin-bottom: 0.5rem;
		color: var(--text-primary);
	}

	:global([data-theme='dark']) .page-header h1 {
		font-weight: 700;
	}

	:global([data-theme='light']) .page-header h1 {
		font-family: 'Noto Sans JP', sans-serif;
		font-weight: 100;
		letter-spacing: 0.2em;
	}

	.subtitle {
		font-size: 0.875rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		color: var(--text-primary);
		opacity: 0.7;
	}

	:global([data-theme='light']) .subtitle {
		font-weight: 200;
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
			padding: 3rem 1rem;
		}

		.page-header h1 {
			font-size: 1.5rem;
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
