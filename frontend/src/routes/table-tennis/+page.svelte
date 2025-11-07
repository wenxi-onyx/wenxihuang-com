<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { authStore } from '$lib/stores/auth';
	import { playersApi, seasonsApi, type PlayerWithStats, type Season, type PlayerSeasonStats, type PlayerEloHistory } from '$lib/api/client';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import LoginButton from '$lib/components/LoginButton.svelte';
	import AddMatchModal, { openAddMatchModal } from '$lib/components/AddMatchModal.svelte';
	import Toast, { showToast } from '$lib/components/Toast.svelte';
	import Presence from '$lib/components/Presence.svelte';
	import {
		Chart,
		LineController,
		LineElement,
		PointElement,
		LinearScale,
		Title,
		Tooltip,
		Legend,
		CategoryScale
	} from 'chart.js';
	import 'chartjs-adapter-date-fns';

	// Register Chart.js components
	Chart.register(
		LineController,
		LineElement,
		PointElement,
		LinearScale,
		Title,
		Tooltip,
		Legend,
		CategoryScale
	);

	const user = $derived($authStore.user);

	let players = $state<PlayerWithStats[]>([]);
	let filteredPlayers = $state<PlayerWithStats[]>([]);
	let seasons = $state<Season[]>([]);
	let selectedSeasonId = $state<string | null>(null);
	let loading = $state(true);
	let error = $state('');
	let searchQuery = $state('');
	let sortField = $state<'rank' | 'name' | 'current_elo' | 'games_played' | 'wins' | 'losses' | 'win_rate'>('current_elo');
	let sortDirection = $state<'asc' | 'desc'>('desc');
	let abortController: AbortController | null = null; // For canceling in-flight requests
	let showManageDropdown = $state(false);

	// Derive selectedSeason from selectedSeasonId
	let selectedSeason = $derived(
		selectedSeasonId ? seasons.find(s => s.id === selectedSeasonId) || null : null
	);

	// Chart related state
	let allPlayersHistory = $state<PlayerEloHistory[]>([]);
	let chartCanvas = $state<HTMLCanvasElement | undefined>();
	let chart: any | null = null;
	let loadingChart = $state(false);

	async function loadSeasons() {
		try {
			seasons = await seasonsApi.listSeasons();
			// Try to get active season first
			const activeSeason = await seasonsApi.getActiveSeason();
			if (activeSeason) {
				selectedSeasonId = activeSeason.id;
			} else if (seasons.length > 0) {
				selectedSeasonId = seasons[0].id;
			}
		} catch (e) {
			console.error('Failed to load seasons:', e);
			// Continue without seasons - will fall back to all-time
		}
	}

	async function loadPlayers(seasonId: string | null) {
		// Cancel any in-flight request
		if (abortController) {
			abortController.abort();
		}
		abortController = new AbortController();
		const currentController = abortController;

		try {
			error = '';

			if (seasonId) {
				// Load season-specific leaderboard
				const seasonPlayers = await seasonsApi.getSeasonLeaderboard(seasonId);

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

			loading = false;
		} catch (e) {
			// Don't show error if request was aborted (user switched seasons)
			if (currentController.signal.aborted) {
				return;
			}
			error = e instanceof Error ? e.message : 'Failed to load players';
			loading = false;
		}
	}

	let previousSeasonId: string | null | undefined = undefined; // Track previous season for change detection

	onMount(async () => {
		await loadSeasons();
		await loadPlayers(selectedSeasonId);
		await loadAllPlayersHistory();
		previousSeasonId = selectedSeasonId;
	});

	onDestroy(() => {
		if (chart) {
			chart.destroy();
		}
	});

	// Reload players when season changes
	$effect(() => {
		// Track selectedSeasonId changes
		const currentSeasonId = selectedSeasonId;

		// Skip initial run (undefined means not yet initialized)
		if (previousSeasonId === undefined) return;

		// Only reload if season actually changed
		if (previousSeasonId !== currentSeasonId) {
			loadPlayers(currentSeasonId);
			previousSeasonId = currentSeasonId;
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

	async function loadAllPlayersHistory() {
		const CACHE_KEY = 'tt-players-history-matches-cache';
		const CACHE_TIMESTAMP_KEY = 'tt-players-history-matches-timestamp';
		const CACHE_DURATION = 60 * 1000; // 60 seconds

		try {
			// Try to load from cache first (synchronously)
			const cachedData = localStorage.getItem(CACHE_KEY);
			const cachedTimestamp = localStorage.getItem(CACHE_TIMESTAMP_KEY);

			let usedCache = false;
			if (cachedData && cachedTimestamp) {
				const age = Date.now() - parseInt(cachedTimestamp);
				// Use cache if it's less than 5 minutes old (stale-while-revalidate)
				if (age < 300000) {
					allPlayersHistory = JSON.parse(cachedData);
					usedCache = true;

					// If cache is very fresh (< 60s), we can skip the loading state
					if (age < CACHE_DURATION) {
						// Don't need to fetch, data is fresh
						return;
					}
				}
			}

			// Fetch fresh data (only show spinner if we didn't have cache)
			if (!usedCache) {
				loadingChart = true;
			}

			const freshData = await playersApi.getAllPlayersHistory();

			// Update state with fresh data
			allPlayersHistory = freshData;

			// Save to cache
			localStorage.setItem(CACHE_KEY, JSON.stringify(freshData));
			localStorage.setItem(CACHE_TIMESTAMP_KEY, Date.now().toString());

			loadingChart = false;
		} catch (e) {
			console.error('Failed to load players history:', e);
			loadingChart = false;
		}
	}

	// Create chart reactively when data and canvas are ready (also when season changes)
	$effect(() => {
		if (chartCanvas && allPlayersHistory.length > 0 && !loadingChart) {
			// Track selectedSeasonId to trigger re-render when it changes
			void selectedSeasonId;
			// Use requestAnimationFrame to ensure canvas is fully rendered
			requestAnimationFrame(() => {
				createChart();
			});
		}
	});

	function createChart() {
		if (!chartCanvas || !allPlayersHistory.length) return;

		// Filter history by selected season
		const filteredHistory = allPlayersHistory.map(player => ({
			...player,
			history: selectedSeasonId
				? player.history.filter(point => point.season_id === selectedSeasonId)
				: player.history
		}));

		// Filter out players with no history and check if we have any data to display
		const playersWithHistory = filteredHistory.filter(p => p.history.length > 0);
		if (playersWithHistory.length === 0) return;

		// Destroy existing chart if any
		if (chart) {
			chart.destroy();
		}

		const ctx = chartCanvas.getContext('2d');
		if (!ctx) return;

		// Get current theme
		const isDark = document.documentElement.getAttribute('data-theme') !== 'light';
		const textColor = isDark ? '#ffffff' : '#000000';
		const gridColor = isDark ? 'rgba(255, 255, 255, 0.08)' : 'rgba(0, 0, 0, 0.05)';
		const borderColor = isDark ? 'rgba(255, 255, 255, 0.15)' : 'rgba(0, 0, 0, 0.08)';

		// Define colors for players (more vibrant palette)
		const playerColors = [
			'rgba(59, 130, 246, 0.9)',   // blue
			'rgba(168, 85, 247, 0.9)',   // purple
			'rgba(34, 197, 94, 0.9)',    // green
			'rgba(234, 179, 8, 0.9)',    // yellow
			'rgba(239, 68, 68, 0.9)',    // red
			'rgba(236, 72, 153, 0.9)',   // pink
			'rgba(14, 165, 233, 0.9)',   // cyan
			'rgba(249, 115, 22, 0.9)',   // orange
			'rgba(139, 92, 246, 0.9)',   // violet
			'rgba(16, 185, 129, 0.9)',   // emerald
			'rgba(245, 158, 11, 0.9)',   // amber
			'rgba(99, 102, 241, 0.9)',   // indigo
		];

		// Find the player with the most data points (calculate once)
		let maxDataPoints = Math.max(...playersWithHistory.map(p => p.history.length + 1));

		// Create datasets for each player
		const datasets: any[] = [];
		let colorIndex = 0;

		playersWithHistory.forEach(playerData => {
			const playerColor = playerColors[colorIndex % playerColors.length];
			colorIndex++;

			const dataPoints = [];
			const playerTotalPoints = playerData.history.length + 1; // +1 for starting point

			// Add starting point (elo_before of first match)
			dataPoints.push({
				x: 0,
				y: playerData.history[0].elo_before
			});

			// Add all matches for this player, spacing them evenly across the chart width
			playerData.history.forEach((point, index) => {
				// Map this player's match index to the full chart width
				// Handle edge case where player has only 1 match (avoid division by zero)
				const normalizedX = playerTotalPoints > 1
					? ((index + 1) / (playerTotalPoints - 1)) * (maxDataPoints - 1)
					: maxDataPoints - 1;

				dataPoints.push({
					x: normalizedX,
					y: point.elo_after
				});
			});

			datasets.push({
				label: playerData.player_name,
				data: dataPoints,
				borderColor: playerColor,
				backgroundColor: 'transparent',
				borderWidth: 2,
				pointRadius: 1,
				pointHoverRadius: 4,
			pointHitRadius: 8,
				pointBackgroundColor: playerColor,
				pointBorderColor: playerColor,
				pointHoverBackgroundColor: textColor,
				pointHoverBorderColor: playerColor,
				pointBorderWidth: 1,
				pointHoverBorderWidth: 2,
				fill: false,
				tension: 0.2,
				spanGaps: false
			});
		});

		chart = new Chart(ctx, {
			type: 'line',
			data: { datasets },
			options: {
				responsive: true,
				maintainAspectRatio: false,
				plugins: {
					title: {
						display: true,
						text: selectedSeason
							? `${selectedSeason.name.toUpperCase()} - ALL PLAYERS ELO HISTORY`
							: 'ALL PLAYERS ELO HISTORY',
						color: textColor,
						font: {
							size: 11,
							weight: isDark ? 500 : 300,
							family: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif'
						},
						padding: {
							top: 0,
							bottom: 20
						}
					},
					legend: {
						display: true,
						position: 'bottom',
						labels: {
							color: textColor,
							font: {
								size: 9,
								weight: isDark ? 400 : 300,
								family: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif'
							},
							padding: 8,
							usePointStyle: true,
							pointStyle: 'rectRounded',
							boxWidth: 25,
							boxHeight: 4
						}
					},
					tooltip: {
						enabled: true,
						backgroundColor: isDark ? 'rgba(0, 0, 0, 0.95)' : 'rgba(255, 255, 255, 0.95)',
						titleColor: textColor,
						bodyColor: textColor,
						borderColor: borderColor,
						borderWidth: 1,
						padding: 12,
						displayColors: true,
						titleFont: {
							size: 10,
							weight: 400,
							family: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif'
						},
						bodyFont: {
							size: 11,
							weight: isDark ? 500 : 300,
							family: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif'
						},
					filter: (tooltipItem, index, tooltipItems) => {
						// Only show the first point for each dataset (player)
						const datasetIndex = tooltipItem.datasetIndex;
						const firstIndexForDataset = tooltipItems.findIndex(item => item.datasetIndex === datasetIndex);
						return index === firstIndexForDataset;
					},
						callbacks: {
							label: (context) => {
								const yValue = context.parsed?.y;
								const playerName = context.dataset.label || '';
								return yValue !== null && yValue !== undefined ? `${playerName}: ${yValue.toFixed(1)}` : '';
							}
						}
					}
				},
				scales: {
					x: {
						type: 'linear',
						grid: {
							color: gridColor,
							lineWidth: 1
						},
						border: {
							display: false
						},
						title: {
							display: true,
							text: 'RELATIVE PROGRESS',
							color: textColor,
							font: {
								size: 9,
								weight: isDark ? 400 : 300,
								family: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif'
							},
							padding: {
								top: 12
							}
						},
						ticks: {
							color: textColor,
							font: {
								size: 10,
								weight: 300,
								family: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif'
							},
							padding: 8,
							callback: function(value) {
								// Show fewer ticks for cleaner display
								return '';
							}
						}
					},
					y: {
						grid: {
							color: gridColor,
							lineWidth: 1
						},
						border: {
							display: false
						},
						title: {
							display: true,
							text: 'ELO RATING',
							color: textColor,
							font: {
								size: 9,
								weight: isDark ? 400 : 300,
								family: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif'
							},
							padding: {
								bottom: 12
							}
						},
						ticks: {
							color: textColor,
							font: {
								size: 10,
								weight: 300,
								family: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif'
							},
							padding: 8
						}
					}
				},
				interaction: {
					mode: 'point',
					intersect: false
				}
			}
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

	function handleAddMatch() {
		// Construct full name from first_name and last_name
		const userName = user
			? [user.first_name, user.last_name].filter(Boolean).join(' ')
			: undefined;

		openAddMatchModal(() => {
			// Show success toast
			showToast('Match recorded successfully!', 'success');
			// Reload players and chart history after match is added
			loadPlayers(selectedSeasonId);
			loadAllPlayersHistory();
		}, userName);
	}
</script>

<svelte:head>
	<title>ELO Leaderboard</title>
</svelte:head>

<ThemeToggle />
<LoginButton />
<AddMatchModal />
<Toast />
<Presence />

<div class="container">
	<header class="page-header">
		<h1>Table Tennis Leaderboard</h1>
		<nav class="nav-links">
			{#if user}
				<button class="nav-link-btn" onclick={handleAddMatch}>
					<span class="plus-icon">+</span> ADD MATCH
				</button>
			{/if}
			<a href="/table-tennis/matches">MATCH HISTORY</a>
			{#if user}
				<div class="dropdown">
					<button
						class="dropdown-toggle"
						onclick={() => showManageDropdown = !showManageDropdown}
						onblur={() => setTimeout(() => showManageDropdown = false, 200)}
					>
						MANAGE ▾
					</button>
					{#if showManageDropdown}
						<div class="dropdown-menu">
							<a href="/table-tennis/seasons" class="dropdown-item">Seasons</a>
							<a href="/table-tennis/admin" class="dropdown-item">Elo Algorithms</a>
							<a href="/table-tennis/admin/players" class="dropdown-item">Players</a>
						</div>
					{/if}
				</div>
			{/if}
			<button class="nav-link-btn" onclick={() => window.history.back()}>BACK</button>
		</nav>
	</header>

	{#if loading}
		<div class="loading">Loading players...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else}
		<div class="controls">
			{#if seasons.length > 0}
				<select
					bind:value={selectedSeasonId}
					class="season-selector"
				>
					{#each seasons as season}
						<option value={season.id}>
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
		</div>

		<div class="table-wrapper">
			<table class="leaderboard-table">
				<thead>
					<tr>
						<th class="sortable" onclick={() => handleSort('rank')}>
							Rank <span class="sort-arrow" class:visible={sortField === 'rank'}>{sortDirection === 'desc' ? '▼' : '▲'}</span>
						</th>
						<th class="sortable" onclick={() => handleSort('name')}>
							Player <span class="sort-arrow" class:visible={sortField === 'name'}>{sortDirection === 'desc' ? '▼' : '▲'}</span>
						</th>
						<th class="sortable" onclick={() => handleSort('current_elo')}>
							ELO Rating <span class="sort-arrow" class:visible={sortField === 'current_elo'}>{sortDirection === 'desc' ? '▼' : '▲'}</span>
						</th>
						<th class="sortable" onclick={() => handleSort('games_played')}>
							Games <span class="sort-arrow" class:visible={sortField === 'games_played'}>{sortDirection === 'desc' ? '▼' : '▲'}</span>
						</th>
						<th class="sortable" onclick={() => handleSort('wins')}>
							Wins <span class="sort-arrow" class:visible={sortField === 'wins'}>{sortDirection === 'desc' ? '▼' : '▲'}</span>
						</th>
						<th class="sortable" onclick={() => handleSort('losses')}>
							Losses <span class="sort-arrow" class:visible={sortField === 'losses'}>{sortDirection === 'desc' ? '▼' : '▲'}</span>
						</th>
						<th class="sortable" onclick={() => handleSort('win_rate')}>
							Win Rate <span class="sort-arrow" class:visible={sortField === 'win_rate'}>{sortDirection === 'desc' ? '▼' : '▲'}</span>
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
								<a href="/table-tennis/players/{player.id}" class="player-name-link">
									<span class="player-name">{player.name}</span>
								</a>
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

		<!-- All Players ELO History Chart -->
		{#if allPlayersHistory.some(p => p.history.length > 0) || loadingChart}
			<div class="chart-section">
				<div class="chart-container">
					{#if loadingChart}
						<div class="chart-loading">
							<div class="spinner"></div>
						</div>
					{/if}
					<canvas bind:this={chartCanvas}></canvas>
				</div>
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
		align-items: center;
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
		line-height: 1;
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
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.nav-link-btn:hover {
		opacity: 1;
	}

	.plus-icon {
		font-size: 1.2rem;
		line-height: 1;
		font-weight: 300;
	}

	.dropdown {
		position: relative;
	}

	.dropdown-toggle {
		font-size: 0.875rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		text-decoration: none;
		color: inherit;
		opacity: 0.7;
		transition: opacity 0.2s ease;
		background: none;
		border: none;
		cursor: pointer;
		padding: 0;
		margin: 0;
		font-family: inherit;
		appearance: none;
		line-height: 1;
	}

	.dropdown-toggle:hover {
		opacity: 1;
	}

	.dropdown-menu {
		position: absolute;
		top: calc(100% + 0.5rem);
		right: 0;
		background: var(--bg-primary);
		border: 1px solid var(--border-subtle);
		min-width: 200px;
		z-index: 1000;
		box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
	}

	.dropdown-item {
		display: block;
		padding: 0.75rem 1rem;
		font-size: 0.875rem;
		font-weight: 300;
		letter-spacing: 0.05em;
		text-decoration: none;
		color: var(--text-primary);
		opacity: 0.8;
		transition: all 0.2s ease;
		border-bottom: 1px solid var(--border-subtle);
	}

	.dropdown-item:last-child {
		border-bottom: none;
	}

	.dropdown-item:hover {
		opacity: 1;
		background: var(--border-subtle);
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
		width: 400px;
		padding: 0.75rem 1rem;
		font-size: 1rem;
		font-family: inherit;
		background: transparent;
		color: var(--text-primary);
		border: 1px solid var(--border-subtle);
		outline: none;
		transition: border-color 0.2s ease;
		margin-left: auto;
	}

	.search-input:focus {
		border-color: var(--border-active);
	}

	.search-input::placeholder {
		color: var(--text-primary);
		opacity: 0.3;
	}

	.season-selector {
		width: 400px;
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

	.sort-arrow {
		display: inline-block;
		width: 0.75em;
		opacity: 0;
		transition: opacity 0.2s;
	}

	.sort-arrow.visible {
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

	.player-name-link {
		text-decoration: none;
		color: inherit;
		display: inline-flex;
		align-items: center;
		transition: opacity 0.2s ease;
	}

	.player-name-link:hover {
		opacity: 0.6;
		text-decoration: underline;
		text-decoration-thickness: 0.5px;
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
		padding: 0.5rem 0;
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

	/* Chart section */
	.chart-section {
		margin-top: 4rem;
		padding-top: 3rem;
		border-top: 1px solid var(--border-subtle);
	}

	.chart-container {
		width: 100%;
		height: 500px;
		background: transparent;
		border: 1px solid var(--border-subtle);
		padding: 2rem;
		position: relative;
	}

	.chart-loading {
		position: absolute;
		top: 2rem;
		right: 2rem;
		z-index: 10;
	}

	.spinner {
		width: 24px;
		height: 24px;
		border: 2px solid var(--border-subtle);
		border-top-color: var(--text-primary);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	@media (max-width: 768px) {
		.chart-container {
			height: 400px;
			padding: 1rem;
		}

		.chart-loading {
			top: 1rem;
			right: 1rem;
		}

		.spinner {
			width: 20px;
			height: 20px;
		}
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
