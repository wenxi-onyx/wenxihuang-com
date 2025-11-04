<script lang="ts">
	import { onMount } from 'svelte';
	import { playersApi, adminApi, type PlayerWithStats } from '$lib/api/client';

	let players: PlayerWithStats[] = [];
	let filteredPlayers: PlayerWithStats[] = [];
	let loading = true;
	let error = '';
	let searchQuery = '';
	let filterStatus: 'all' | 'active' | 'inactive' = 'all';
	let sortField: 'name' | 'current_elo' | 'games_played' = 'current_elo';
	let sortDirection: 'asc' | 'desc' = 'desc';

	onMount(async () => {
		await loadPlayers();
	});

	async function loadPlayers() {
		try {
			loading = true;
			players = await playersApi.listPlayers();
			loading = false;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load players';
			loading = false;
		}
	}

	$: {
		// Filter players
		filteredPlayers = players.filter(player => {
			const matchesSearch = player.name.toLowerCase().includes(searchQuery.toLowerCase());
			const matchesStatus =
				filterStatus === 'all' ||
				(filterStatus === 'active' && player.is_active) ||
				(filterStatus === 'inactive' && !player.is_active);
			return matchesSearch && matchesStatus;
		});

		// Sort players
		filteredPlayers = [...filteredPlayers].sort((a, b) => {
			if (sortField === 'name') {
				return sortDirection === 'asc'
					? a.name.localeCompare(b.name)
					: b.name.localeCompare(a.name);
			}

			let aValue = a[sortField];
			let bValue = b[sortField];

			if (typeof aValue === 'number' && typeof bValue === 'number') {
				return sortDirection === 'asc' ? aValue - bValue : bValue - aValue;
			}
			return 0;
		});
	}

	async function handleToggleActive(playerId: string, currentStatus: boolean) {
		const action = currentStatus ? 'deactivate' : 'activate';
		if (!confirm(`Are you sure you want to ${action} this player?`)) {
			return;
		}

		try {
			await adminApi.togglePlayerActive(playerId);
			await loadPlayers();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to update player status';
		}
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

	$: activeCount = players.filter(p => p.is_active).length;
	$: inactiveCount = players.filter(p => !p.is_active).length;
</script>

<svelte:head>
	<title>Player Management</title>
</svelte:head>

<div class="container">
	<div class="header-section">
		<a href="/table-tennis/admin" class="btn-back">← Back to Configurator</a>
		<h1>Player Management</h1>
		<p class="subtitle">Manage player active status and view statistics</p>
	</div>

	{#if error}
		<div class="alert alert-error">
			{error}
			<button class="btn-close" on:click={() => error = ''}>×</button>
		</div>
	{/if}

	{#if loading}
		<div class="loading">Loading players...</div>
	{:else}
		<div class="stats-bar">
			<div class="stat-card">
				<div class="stat-value">{players.length}</div>
				<div class="stat-label">Total Players</div>
			</div>
			<div class="stat-card active">
				<div class="stat-value">{activeCount}</div>
				<div class="stat-label">Active</div>
			</div>
			<div class="stat-card inactive">
				<div class="stat-value">{inactiveCount}</div>
				<div class="stat-label">Inactive</div>
			</div>
		</div>

		<div class="controls">
			<input
				type="text"
				placeholder="Search players..."
				bind:value={searchQuery}
				class="search-input"
			/>

			<div class="filter-buttons">
				<button
					class="filter-btn"
					class:active={filterStatus === 'all'}
					on:click={() => filterStatus = 'all'}
				>
					All
				</button>
				<button
					class="filter-btn"
					class:active={filterStatus === 'active'}
					on:click={() => filterStatus = 'active'}
				>
					Active
				</button>
				<button
					class="filter-btn"
					class:active={filterStatus === 'inactive'}
					on:click={() => filterStatus = 'inactive'}
				>
					Inactive
				</button>
			</div>
		</div>

		<div class="table-wrapper">
			<table class="players-table">
				<thead>
					<tr>
						<th class="sortable" on:click={() => handleSort('name')}>
							Player {sortField === 'name' ? (sortDirection === 'desc' ? '▼' : '▲') : ''}
						</th>
						<th class="sortable" on:click={() => handleSort('current_elo')}>
							ELO Rating {sortField === 'current_elo' ? (sortDirection === 'desc' ? '▼' : '▲') : ''}
						</th>
						<th class="sortable" on:click={() => handleSort('games_played')}>
							Games {sortField === 'games_played' ? (sortDirection === 'desc' ? '▼' : '▲') : ''}
						</th>
						<th>Wins / Losses</th>
						<th>Win Rate</th>
						<th>Status</th>
						<th>Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each filteredPlayers as player}
						<tr class:inactive-row={!player.is_active}>
							<td class="name-cell">
								<div class="player-info">
									<span class="player-name">{player.name}</span>
								</div>
							</td>
							<td class="elo-cell">
								<span class="elo-value">{player.current_elo.toFixed(1)}</span>
							</td>
							<td>{player.games_played}</td>
							<td>
								<span class="wins">{player.wins}</span>
								<span class="separator">/</span>
								<span class="losses">{player.losses}</span>
							</td>
							<td class="win-rate">{getWinRate(player)}%</td>
							<td>
								<span class="status-badge" class:active={player.is_active} class:inactive={!player.is_active}>
									{player.is_active ? 'Active' : 'Inactive'}
								</span>
							</td>
							<td class="actions-cell">
								<button
									class="btn btn-sm"
									class:btn-warning={player.is_active}
									class:btn-success={!player.is_active}
									on:click={() => handleToggleActive(player.id, player.is_active)}
								>
									{player.is_active ? 'Deactivate' : 'Activate'}
								</button>
								<a href="/table-tennis/players/{player.id}" class="btn btn-sm btn-secondary">
									View History
								</a>
							</td>
						</tr>
					{:else}
						<tr>
							<td colspan="7" class="no-results">
								{searchQuery || filterStatus !== 'all' ? 'No players found matching filters' : 'No players available'}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<div class="results-summary">
			Showing {filteredPlayers.length} of {players.length} players
		</div>
	{/if}
</div>

<style>
	.container {
		max-width: 1400px;
		margin: 0 auto;
		padding: 2rem;
	}

	.header-section {
		margin-bottom: 3rem;
	}

	.btn-back {
		display: inline-block;
		margin-bottom: 1rem;
		padding: 0.5rem 1rem;
		font-size: 0.95rem;
		color: var(--text-secondary, #666);
		text-decoration: none;
		border: 1px solid var(--border-color, #e5e7eb);
		border-radius: 6px;
		transition: all 0.2s;
	}

	.btn-back:hover {
		background: var(--bg-secondary, #f9fafb);
		border-color: var(--accent-color, #3b82f6);
		color: var(--accent-color, #3b82f6);
	}

	h1 {
		font-size: 2.5rem;
		font-weight: 700;
		margin-bottom: 0.5rem;
		color: var(--text-primary, #1a1a1a);
	}

	.subtitle {
		font-size: 1.1rem;
		color: var(--text-secondary, #666);
	}

	.alert {
		padding: 1rem 1.5rem;
		border-radius: 8px;
		margin-bottom: 1.5rem;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.alert-error {
		background: #fef2f2;
		color: #991b1b;
		border: 1px solid #fee2e2;
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

	.loading {
		text-align: center;
		padding: 3rem;
		font-size: 1.2rem;
		color: var(--text-secondary, #666);
	}

	.stats-bar {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1.5rem;
		margin-bottom: 2rem;
	}

	.stat-card {
		background: var(--bg-primary, white);
		border: 2px solid var(--border-color, #e5e7eb);
		border-radius: 12px;
		padding: 1.5rem;
		text-align: center;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}

	.stat-card.active {
		border-color: #16a34a;
		background: linear-gradient(to bottom, #f0fdf4, white);
	}

	.stat-card.inactive {
		border-color: #d97706;
		background: linear-gradient(to bottom, #fffbeb, white);
	}

	.stat-value {
		font-size: 2.5rem;
		font-weight: 700;
		color: var(--text-primary, #1a1a1a);
		margin-bottom: 0.25rem;
	}

	.stat-label {
		font-size: 0.95rem;
		color: var(--text-secondary, #666);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.controls {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 1rem;
		margin-bottom: 2rem;
		flex-wrap: wrap;
	}

	.search-input {
		flex: 1;
		min-width: 250px;
		padding: 0.75rem 1rem;
		font-size: 1rem;
		border: 2px solid var(--border-color, #e5e7eb);
		border-radius: 8px;
		background: var(--bg-primary, white);
		color: var(--text-primary, #1a1a1a);
		transition: border-color 0.2s;
	}

	.search-input:focus {
		outline: none;
		border-color: var(--accent-color, #3b82f6);
	}

	.filter-buttons {
		display: flex;
		gap: 0.5rem;
	}

	.filter-btn {
		padding: 0.75rem 1.25rem;
		font-size: 0.95rem;
		font-weight: 500;
		border: 2px solid var(--border-color, #e5e7eb);
		border-radius: 8px;
		background: var(--bg-primary, white);
		color: var(--text-primary, #1a1a1a);
		cursor: pointer;
		transition: all 0.2s;
	}

	.filter-btn:hover {
		border-color: var(--accent-color, #3b82f6);
		background: var(--bg-secondary, #f9fafb);
	}

	.filter-btn.active {
		background: var(--accent-color, #3b82f6);
		color: white;
		border-color: var(--accent-color, #3b82f6);
	}

	.table-wrapper {
		overflow-x: auto;
		background: var(--bg-primary, white);
		border-radius: 12px;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
		margin-bottom: 1rem;
	}

	.players-table {
		width: 100%;
		border-collapse: collapse;
	}

	.players-table thead {
		background: var(--bg-secondary, #f9fafb);
		border-bottom: 2px solid var(--border-color, #e5e7eb);
	}

	.players-table th {
		padding: 1rem;
		text-align: left;
		font-weight: 600;
		color: var(--text-primary, #1a1a1a);
		font-size: 0.875rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.players-table th.sortable {
		cursor: pointer;
		user-select: none;
		transition: background 0.2s;
	}

	.players-table th.sortable:hover {
		background: var(--hover-bg, #f3f4f6);
	}

	.players-table tbody tr {
		border-bottom: 1px solid var(--border-color, #e5e7eb);
		transition: background 0.15s;
	}

	.players-table tbody tr:hover {
		background: var(--hover-bg, #f9fafb);
	}

	.players-table tbody tr.inactive-row {
		opacity: 0.7;
	}

	.players-table td {
		padding: 1rem;
		font-size: 0.95rem;
		color: var(--text-primary, #1a1a1a);
	}

	.name-cell {
		font-weight: 500;
	}

	.player-name {
		font-size: 1.05rem;
	}

	.elo-cell {
		font-weight: 600;
	}

	.elo-value {
		color: var(--accent-color, #3b82f6);
		font-size: 1.1rem;
	}

	.wins {
		color: var(--success-color, #16a34a);
		font-weight: 600;
	}

	.separator {
		color: var(--text-secondary, #999);
		margin: 0 0.25rem;
	}

	.losses {
		color: var(--error-color, #dc2626);
		font-weight: 600;
	}

	.win-rate {
		font-weight: 600;
	}

	.status-badge {
		display: inline-block;
		padding: 0.25rem 0.75rem;
		font-size: 0.875rem;
		border-radius: 6px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.025em;
	}

	.status-badge.active {
		background: #d1fae5;
		color: #065f46;
	}

	.status-badge.inactive {
		background: #fef3c7;
		color: #92400e;
	}

	.actions-cell {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.btn {
		padding: 0.5rem 1rem;
		font-size: 0.875rem;
		font-weight: 500;
		border-radius: 6px;
		border: none;
		cursor: pointer;
		transition: all 0.2s;
		text-decoration: none;
		display: inline-block;
		text-align: center;
	}

	.btn-sm {
		padding: 0.5rem 0.875rem;
		font-size: 0.8125rem;
	}

	.btn-success {
		background: #16a34a;
		color: white;
	}

	.btn-success:hover {
		background: #15803d;
	}

	.btn-warning {
		background: #d97706;
		color: white;
	}

	.btn-warning:hover {
		background: #b45309;
	}

	.btn-secondary {
		background: var(--bg-secondary, #f9fafb);
		color: var(--text-primary, #1a1a1a);
		border: 1px solid var(--border-color, #e5e7eb);
	}

	.btn-secondary:hover {
		background: #f3f4f6;
	}

	.no-results {
		text-align: center;
		padding: 3rem !important;
		color: var(--text-secondary, #666);
	}

	.results-summary {
		text-align: center;
		padding: 1rem;
		font-size: 0.95rem;
		color: var(--text-secondary, #666);
	}

	@media (max-width: 768px) {
		.container {
			padding: 1rem;
		}

		h1 {
			font-size: 2rem;
		}

		.stats-bar {
			grid-template-columns: 1fr;
		}

		.controls {
			flex-direction: column;
			align-items: stretch;
		}

		.search-input {
			min-width: 100%;
		}

		.filter-buttons {
			justify-content: center;
		}

		.players-table {
			font-size: 0.875rem;
		}

		.players-table th,
		.players-table td {
			padding: 0.75rem 0.5rem;
		}

		/* Stack actions vertically on mobile */
		.actions-cell {
			flex-direction: column;
		}

		.btn {
			width: 100%;
		}
	}
</style>
