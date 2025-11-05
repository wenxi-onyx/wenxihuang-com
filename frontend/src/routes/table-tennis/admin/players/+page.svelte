<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/stores/auth';
	import { playersApi, adminApi, type PlayerWithStats } from '$lib/api/client';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import LoginButton from '$lib/components/LoginButton.svelte';
	import Toast, { showToast } from '$lib/components/Toast.svelte';
	import ConfirmModal, { confirm } from '$lib/components/ConfirmModal.svelte';

	let players = $state<PlayerWithStats[]>([]);
	let loading = $state(true);
	let searchQuery = $state('');
	let filterStatus = $state<'all' | 'active' | 'inactive'>('all');
	let sortField = $state<'name' | 'current_elo' | 'games_played'>('current_elo');
	let sortDirection = $state<'asc' | 'desc'>('desc');

	onMount(async () => {
		const currentUser = await authStore.checkAuth();
		if (!currentUser) {
			showToast('You must be logged in to access this page', 'error');
			goto('/login');
			return;
		}
		await loadPlayers();
	});

	async function loadPlayers() {
		try {
			loading = true;
			players = await playersApi.listPlayers();
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to load players', 'error');
		} finally {
			loading = false;
		}
	}

	// Filter and sort players reactively
	const filteredPlayers = $derived.by(() => {
		// Filter players
		const filtered = players.filter(player => {
			const matchesSearch = player.name.toLowerCase().includes(searchQuery.toLowerCase());
			const matchesStatus =
				filterStatus === 'all' ||
				(filterStatus === 'active' && player.is_active) ||
				(filterStatus === 'inactive' && !player.is_active);
			return matchesSearch && matchesStatus;
		});

		// Sort players
		return [...filtered].sort((a, b) => {
			if (sortField === 'name') {
				return sortDirection === 'asc'
					? a.name.localeCompare(b.name)
					: b.name.localeCompare(a.name);
			}

			const aValue = a[sortField];
			const bValue = b[sortField];

			if (typeof aValue === 'number' && typeof bValue === 'number') {
				return sortDirection === 'asc' ? aValue - bValue : bValue - aValue;
			}
			return 0;
		});
	});

	const activeCount = $derived(players.filter(p => p.is_active).length);
	const inactiveCount = $derived(players.filter(p => !p.is_active).length);

	async function handleToggleActive(playerId: string, currentStatus: boolean) {
		const action = currentStatus ? 'deactivate' : 'activate';
		const confirmed = await confirm({
			title: `${action.charAt(0).toUpperCase() + action.slice(1)} Player`,
			message: `Are you sure you want to ${action} this player?`,
			confirmText: action.toUpperCase(),
			cancelText: 'CANCEL',
			confirmStyle: currentStatus ? 'danger' : 'primary'
		});

		if (!confirmed) return;

		try {
			await adminApi.togglePlayerActive(playerId);
			showToast(`Player ${action}d successfully`, 'success');
			await loadPlayers();
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to update player status', 'error');
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
</script>

<svelte:head>
	<title>Player Management</title>
</svelte:head>

<ThemeToggle />
<LoginButton />
<Toast />
<ConfirmModal />

<div class="container">
	<header class="page-header">
		<h1>Player Management</h1>
		<nav class="nav-links">
			<a href="/table-tennis/seasons">SEASONS</a>
			<a href="/table-tennis/admin">ELO ALGORITHMS</a>
			<a href="/table-tennis">BACK</a>
		</nav>
	</header>

	{#if loading}
		<div class="loading">Loading players...</div>
	{:else}
		<div class="stats-summary">
			<span class="stat">{players.length} Total</span>
			<span class="stat">{activeCount} Active</span>
			<span class="stat">{inactiveCount} Inactive</span>
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
					onclick={() => filterStatus = 'all'}
				>
					All
				</button>
				<button
					class="filter-btn"
					class:active={filterStatus === 'active'}
					onclick={() => filterStatus = 'active'}
				>
					Active
				</button>
				<button
					class="filter-btn"
					class:active={filterStatus === 'inactive'}
					onclick={() => filterStatus = 'inactive'}
				>
					Inactive
				</button>
			</div>
		</div>

		<div class="table-wrapper">
			<table class="players-table">
				<thead>
					<tr>
						<th class="sortable" onclick={() => handleSort('name')}>
							Player {sortField === 'name' ? (sortDirection === 'desc' ? '▼' : '▲') : ''}
						</th>
						<th class="sortable" onclick={() => handleSort('current_elo')}>
							ELO Rating {sortField === 'current_elo' ? (sortDirection === 'desc' ? '▼' : '▲') : ''}
						</th>
						<th class="sortable" onclick={() => handleSort('games_played')}>
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
								<span class="player-name">{player.name}</span>
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
								<span class="status-badge" class:active={player.is_active}>
									{player.is_active ? 'Active' : 'Inactive'}
								</span>
							</td>
							<td class="actions-cell">
								<button
									class="btn-action"
									onclick={() => handleToggleActive(player.id, player.is_active)}
								>
									{player.is_active ? 'Deactivate' : 'Activate'}
								</button>
								<a href="/table-tennis/players/{player.id}" class="btn-view">
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
		max-width: 1200px;
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

	h1 {
		font-size: clamp(1.5rem, 4vw, 2.5rem);
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		margin: 0;
		color: var(--text-primary);
	}

	.nav-links {
		display: flex;
		gap: 1.5rem;
	}

	.nav-links a {
		font-size: 0.875rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		text-decoration: none;
		color: var(--text-primary);
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
		opacity: 0.7;
		font-weight: 300;
		letter-spacing: 0.05em;
	}

	.stats-summary {
		display: flex;
		gap: 2rem;
		margin-bottom: 2rem;
		font-size: 0.875rem;
		color: var(--text-primary);
		opacity: 0.7;
		font-weight: 300;
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

	.filter-buttons {
		display: flex;
		gap: 0.5rem;
	}

	.filter-btn {
		padding: 0.75rem 1.25rem;
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		border: 1px solid var(--border-subtle);
		background: transparent;
		color: var(--text-primary);
		cursor: pointer;
		transition: all 0.2s ease;
		font-family: inherit;
	}

	.filter-btn:hover {
		border-color: var(--border-active);
		background: var(--border-subtle);
	}

	.filter-btn.active {
		border-color: var(--border-active);
		background: rgba(255, 255, 255, 0.05);
	}

	.table-wrapper {
		overflow-x: auto;
		border: 1px solid var(--border-subtle);
		background: transparent;
		margin-bottom: 1rem;
	}

	.players-table {
		width: 100%;
		border-collapse: collapse;
	}

	.players-table thead {
		background: transparent;
		border-bottom: 1px solid var(--border-subtle);
	}

	.players-table th {
		padding: 1rem;
		text-align: left;
		font-weight: 300;
		color: var(--text-primary);
		font-size: 0.75rem;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		opacity: 0.8;
	}

	.players-table th.sortable {
		cursor: pointer;
		user-select: none;
		transition: opacity 0.2s;
	}

	.players-table th.sortable:hover {
		opacity: 1;
	}

	.players-table tbody tr {
		border-bottom: 1px solid var(--border-subtle);
		transition: opacity 0.15s;
	}

	.players-table tbody tr:hover {
		opacity: 0.8;
	}

	.players-table tbody tr.inactive-row {
		opacity: 0.5;
	}

	.players-table td {
		padding: 1rem;
		font-size: 0.875rem;
		font-weight: 300;
		color: var(--text-primary);
	}

	.elo-value {
		font-size: 1rem;
	}

	.separator {
		opacity: 0.4;
		margin: 0 0.25rem;
	}

	.status-badge {
		display: inline-block;
		padding: 0.25rem 0.75rem;
		font-size: 0.625rem;
		border: 1px solid var(--border-subtle);
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		opacity: 0.6;
	}

	.status-badge.active {
		opacity: 0.8;
	}

	.actions-cell {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.btn-action {
		padding: 0.5rem 0.875rem;
		font-size: 0.625rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		background: transparent;
		border: 1px solid var(--border-subtle);
		color: var(--text-primary);
		cursor: pointer;
		transition: all 0.2s ease;
		font-family: inherit;
	}

	.btn-action:hover {
		border-color: var(--border-active);
		background: var(--border-subtle);
	}

	.btn-view {
		display: inline-block;
		padding: 0.5rem 0.875rem;
		font-size: 0.625rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		color: var(--text-primary);
		text-decoration: underline;
		text-decoration-thickness: 0.5px;
		border: none;
		background: transparent;
		transition: opacity 0.3s ease;
	}

	.btn-view:hover {
		opacity: 0.6;
	}

	.no-results {
		text-align: center;
		padding: 3rem !important;
		color: var(--text-primary);
		opacity: 0.6;
		font-weight: 300;
	}

	.results-summary {
		text-align: center;
		padding: 1rem;
		font-size: 0.875rem;
		color: var(--text-primary);
		opacity: 0.7;
		font-weight: 300;
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

		.nav-links {
			flex-direction: column;
			gap: 0.75rem;
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
			font-size: 0.8rem;
		}

		.players-table th,
		.players-table td {
			padding: 0.75rem 0.5rem;
		}

		.actions-cell {
			flex-direction: column;
		}

		.btn-action,
		.btn-view {
			width: 100%;
			text-align: center;
		}
	}
</style>
