<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { playersApi, type PlayerWithStats, type EloHistoryPoint } from '$lib/api/client';
	import {
		Chart,
		LineController,
		LineElement,
		PointElement,
		LinearScale,
		TimeScale,
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
		TimeScale,
		Title,
		Tooltip,
		Legend,
		CategoryScale
	);

	let player: PlayerWithStats | null = null;
	let history: EloHistoryPoint[] = [];
	let loading = true;
	let error = '';
	let chartCanvas: HTMLCanvasElement;
	let chart: any | null = null;

	$: playerId = $page.params.id || '';

	onMount(async () => {
		if (!playerId) {
			error = 'Player ID not provided';
			loading = false;
			return;
		}

		try {
			// Fetch player data and history
			const [allPlayers, playerHistory] = await Promise.all([
				playersApi.listPlayers(),
				playersApi.getPlayerHistory(playerId)
			]);

			player = allPlayers.find(p => p.id === playerId) || null;
			history = playerHistory;

			if (!player) {
				error = 'Player not found';
			} else {
				// Create chart after data is loaded
				setTimeout(() => createChart(), 0);
			}

			loading = false;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load player data';
			loading = false;
		}
	});

	onDestroy(() => {
		if (chart) {
			chart.destroy();
		}
	});

	function createChart() {
		if (!chartCanvas || !history.length) return;

		// Destroy existing chart if any
		if (chart) {
			chart.destroy();
		}

		const ctx = chartCanvas.getContext('2d');
		if (!ctx) return;

		// Prepare data points
		const dataPoints = history.map(point => ({
			x: new Date(point.created_at),
			y: point.elo_after
		}));

		// Add starting point if we have history
		if (history.length > 0) {
			dataPoints.unshift({
				x: new Date(history[0].created_at),
				y: history[0].elo_before
			});
		}

		chart = new Chart(ctx, {
			type: 'line',
			data: {
				datasets: [{
					label: 'ELO Rating',
					data: dataPoints,
					borderColor: 'rgb(59, 130, 246)',
					backgroundColor: 'rgba(59, 130, 246, 0.1)',
					borderWidth: 2,
					pointRadius: 3,
					pointHoverRadius: 5,
					fill: true,
					tension: 0.1
				}]
			},
			options: {
				responsive: true,
				maintainAspectRatio: false,
				plugins: {
					title: {
						display: true,
						text: 'ELO Rating Over Time',
						font: {
							size: 16,
							weight: 'bold'
						}
					},
					legend: {
						display: false
					},
					tooltip: {
						mode: 'index',
						intersect: false,
						callbacks: {
							title: (context) => {
								const xValue = context[0]?.parsed?.x;
								if (xValue) {
									const date = new Date(xValue);
									return date.toLocaleDateString('en-US', {
										year: 'numeric',
										month: 'short',
										day: 'numeric'
									});
								}
								return '';
							},
							label: (context) => {
								const yValue = context.parsed?.y;
								return yValue !== null && yValue !== undefined ? `ELO: ${yValue.toFixed(1)}` : '';
							}
						}
					}
				},
				scales: {
					x: {
						type: 'time',
						time: {
							unit: 'day',
							displayFormats: {
								day: 'MMM d'
							}
						},
						title: {
							display: true,
							text: 'Date'
						}
					},
					y: {
						title: {
							display: true,
							text: 'ELO Rating'
						},
						beginAtZero: false
					}
				},
				interaction: {
					mode: 'nearest',
					axis: 'x',
					intersect: false
				}
			}
		});
	}

	function getWinRate(player: PlayerWithStats): string {
		if (player.games_played === 0) return '0.0';
		return ((player.wins / player.games_played) * 100).toFixed(1);
	}

	function getEloChange(): string {
		if (history.length === 0) return '0';
		const first = history[0].elo_before;
		const last = history[history.length - 1].elo_after;
		const change = last - first;
		return change > 0 ? `+${change.toFixed(1)}` : change.toFixed(1);
	}

	function getHighestElo(): number {
		if (history.length === 0) return player?.current_elo || 0;
		return Math.max(...history.map(h => h.elo_after), history[0].elo_before);
	}

	function getLowestElo(): number {
		if (history.length === 0) return player?.current_elo || 0;
		return Math.min(...history.map(h => h.elo_after), history[0].elo_before);
	}
</script>

<svelte:head>
	<title>{player?.name || 'Player'} - ELO History</title>
</svelte:head>

<div class="container">
	{#if loading}
		<div class="loading">Loading player data...</div>
	{:else if error}
		<div class="error">{error}</div>
		<a href="/table-tennis" class="btn-back">Back to Leaderboard</a>
	{:else if player}
		<div class="header-section">
			<a href="/table-tennis" class="btn-back">← Back to Leaderboard</a>
			<h1 class="player-name">{player.name}</h1>
			{#if !player.is_active}
				<span class="inactive-badge">Inactive</span>
			{/if}
		</div>

		<div class="stats-grid">
			<div class="stat-card">
				<div class="stat-label">Current ELO</div>
				<div class="stat-value primary">{player.current_elo.toFixed(1)}</div>
			</div>

			<div class="stat-card">
				<div class="stat-label">Total Change</div>
				<div class="stat-value" class:positive={getEloChange().startsWith('+')} class:negative={getEloChange().startsWith('-')}>
					{getEloChange()}
				</div>
			</div>

			<div class="stat-card">
				<div class="stat-label">Games Played</div>
				<div class="stat-value">{player.games_played}</div>
			</div>

			<div class="stat-card">
				<div class="stat-label">Win Rate</div>
				<div class="stat-value">{getWinRate(player)}%</div>
			</div>

			<div class="stat-card">
				<div class="stat-label">Wins / Losses</div>
				<div class="stat-value">
					<span class="wins">{player.wins}</span> / <span class="losses">{player.losses}</span>
				</div>
			</div>

			<div class="stat-card">
				<div class="stat-label">Peak ELO</div>
				<div class="stat-value">{getHighestElo().toFixed(1)}</div>
			</div>

			<div class="stat-card">
				<div class="stat-label">Lowest ELO</div>
				<div class="stat-value">{getLowestElo().toFixed(1)}</div>
			</div>
		</div>

		{#if history.length > 0}
			<div class="chart-container">
				<canvas bind:this={chartCanvas}></canvas>
			</div>

			<div class="history-section">
				<h2>Recent Match History</h2>
				<div class="history-table-wrapper">
					<table class="history-table">
						<thead>
							<tr>
								<th>Date</th>
								<th>ELO Before</th>
								<th>ELO After</th>
								<th>Change</th>
								<th>Version</th>
							</tr>
						</thead>
						<tbody>
							{#each history.slice(-20).reverse() as point}
								<tr>
									<td>{new Date(point.created_at).toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric' })}</td>
									<td>{point.elo_before.toFixed(1)}</td>
									<td>{point.elo_after.toFixed(1)}</td>
									<td class:positive={point.elo_after > point.elo_before} class:negative={point.elo_after < point.elo_before}>
										{point.elo_after > point.elo_before ? '+' : ''}{(point.elo_after - point.elo_before).toFixed(1)}
									</td>
									<td><span class="version-badge">{point.elo_version}</span></td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</div>
		{:else}
			<div class="no-data">No match history available for this player.</div>
		{/if}
	{/if}
</div>

<style>
	.container {
		max-width: 1200px;
		margin: 0 auto;
		padding: 2rem;
	}

	.loading, .error {
		text-align: center;
		padding: 3rem;
		font-size: 1.2rem;
	}

	.error {
		color: var(--error-color, #dc2626);
	}

	.header-section {
		margin-bottom: 2rem;
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

	.player-name {
		font-size: 2.5rem;
		font-weight: 700;
		margin: 0.5rem 0;
		color: var(--text-primary, #1a1a1a);
	}

	.inactive-badge {
		display: inline-block;
		padding: 0.25rem 0.75rem;
		font-size: 0.875rem;
		background: var(--warning-light, #fef3c7);
		color: var(--warning-color, #d97706);
		border-radius: 6px;
		font-weight: 600;
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1.5rem;
		margin-bottom: 3rem;
	}

	.stat-card {
		background: var(--bg-primary, white);
		border: 1px solid var(--border-color, #e5e7eb);
		border-radius: 12px;
		padding: 1.5rem;
		text-align: center;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}

	.stat-label {
		font-size: 0.875rem;
		color: var(--text-secondary, #666);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-bottom: 0.5rem;
	}

	.stat-value {
		font-size: 2rem;
		font-weight: 700;
		color: var(--text-primary, #1a1a1a);
	}

	.stat-value.primary {
		color: var(--accent-color, #3b82f6);
	}

	.stat-value.positive {
		color: var(--success-color, #16a34a);
	}

	.stat-value.negative {
		color: var(--error-color, #dc2626);
	}

	.wins {
		color: var(--success-color, #16a34a);
	}

	.losses {
		color: var(--error-color, #dc2626);
	}

	.chart-container {
		background: var(--bg-primary, white);
		border: 1px solid var(--border-color, #e5e7eb);
		border-radius: 12px;
		padding: 2rem;
		margin-bottom: 3rem;
		height: 400px;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}

	.history-section {
		margin-top: 3rem;
	}

	.history-section h2 {
		font-size: 1.75rem;
		font-weight: 600;
		margin-bottom: 1.5rem;
		color: var(--text-primary, #1a1a1a);
	}

	.history-table-wrapper {
		overflow-x: auto;
		background: var(--bg-primary, white);
		border-radius: 12px;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}

	.history-table {
		width: 100%;
		border-collapse: collapse;
	}

	.history-table thead {
		background: var(--bg-secondary, #f9fafb);
		border-bottom: 2px solid var(--border-color, #e5e7eb);
	}

	.history-table th {
		padding: 1rem;
		text-align: left;
		font-weight: 600;
		color: var(--text-primary, #1a1a1a);
		font-size: 0.875rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.history-table tbody tr {
		border-bottom: 1px solid var(--border-color, #e5e7eb);
	}

	.history-table tbody tr:hover {
		background: var(--hover-bg, #f9fafb);
	}

	.history-table td {
		padding: 1rem;
		font-size: 0.95rem;
		color: var(--text-primary, #1a1a1a);
	}

	.history-table td.positive {
		color: var(--success-color, #16a34a);
		font-weight: 600;
	}

	.history-table td.negative {
		color: var(--error-color, #dc2626);
		font-weight: 600;
	}

	.version-badge {
		display: inline-block;
		padding: 0.25rem 0.75rem;
		font-size: 0.75rem;
		background: var(--accent-light, #dbeafe);
		color: var(--accent-color, #3b82f6);
		border-radius: 4px;
		font-weight: 600;
	}

	.no-data {
		text-align: center;
		padding: 3rem;
		color: var(--text-secondary, #666);
		font-size: 1.1rem;
	}

	@media (max-width: 768px) {
		.container {
			padding: 1rem;
		}

		.player-name {
			font-size: 2rem;
		}

		.stats-grid {
			grid-template-columns: repeat(2, 1fr);
			gap: 1rem;
		}

		.chart-container {
			padding: 1rem;
			height: 300px;
		}

		.history-table th,
		.history-table td {
			padding: 0.75rem 0.5rem;
			font-size: 0.875rem;
		}
	}
</style>
