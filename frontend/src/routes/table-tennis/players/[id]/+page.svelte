<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { playersApi, type PlayerWithStats, type EloHistoryPoint } from '$lib/api/client';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import LoginButton from '$lib/components/LoginButton.svelte';
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

<ThemeToggle />
<LoginButton />

<div class="container">
	{#if loading}
		<div class="loading">Loading player data...</div>
	{:else if error}
		<div class="error">{error}</div>
		<a href="/table-tennis" class="btn-back">Back to Leaderboard</a>
	{:else if player}
		<header class="page-header">
			<div class="title-wrapper">
				<h1>{player.name}</h1>
				{#if !player.is_active}
					<span class="inactive-badge">Inactive</span>
				{/if}
			</div>
			<nav class="nav-links">
				<a href="/table-tennis">BACK TO LEADERBOARD</a>
			</nav>
		</header>

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
				<h2 class="section-title">Recent Match History</h2>
				<div class="table-wrapper">
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
		max-width: 1400px;
		margin: 0 auto;
		padding: 6rem 2rem 4rem 2rem;
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

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 3rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.1);
	}

	.title-wrapper {
		display: flex;
		align-items: center;
		gap: 1rem;
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

	.btn-back {
		display: inline-block;
		margin-bottom: 1rem;
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

	:global([data-theme='light']) .btn-back {
		font-weight: 200;
	}

	.btn-back:hover {
		opacity: 0.6;
	}

	.inactive-badge {
		display: inline-block;
		padding: 0.25rem 0.75rem;
		font-size: 0.625rem;
		border: 1px solid rgba(255, 255, 255, 0.3);
		text-transform: uppercase;
		letter-spacing: 0.1em;
		font-weight: 300;
		opacity: 0.8;
		color: var(--text-primary);
		background: transparent;
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1rem;
		margin-bottom: 3rem;
	}

	.stat-card {
		background: transparent;
		border: 1px solid var(--border-subtle);
		padding: 1.5rem;
		text-align: center;
		transition: opacity 0.15s;
	}

	.stat-card:hover {
		opacity: 0.8;
	}

	.stat-label {
		font-size: 0.75rem;
		color: var(--text-primary);
		text-transform: uppercase;
		letter-spacing: 0.1em;
		margin-bottom: 0.75rem;
		font-weight: 300;
		opacity: 0.8;
	}

	:global([data-theme='dark']) .stat-label {
		font-weight: 500;
	}

	:global([data-theme='light']) .stat-label {
		font-weight: 200;
	}

	.stat-value {
		font-size: 1.5rem;
		font-weight: 300;
		color: var(--text-primary);
	}

	:global([data-theme='dark']) .stat-value {
		font-weight: 500;
	}

	.stat-value.primary {
		font-weight: 300;
	}

	:global([data-theme='dark']) .stat-value.primary {
		font-weight: 700;
	}

	.stat-value.positive {
		opacity: 0.9;
	}

	.stat-value.negative {
		opacity: 0.7;
	}

	.wins {
		opacity: 0.9;
	}

	.losses {
		opacity: 0.7;
	}

	.chart-container {
		background: transparent;
		border: 1px solid var(--border-subtle);
		padding: 2rem;
		margin-bottom: 3rem;
		height: 400px;
	}

	.history-section {
		margin-top: 3rem;
	}

	.section-title {
		font-size: 1rem;
		font-weight: 300;
		letter-spacing: 0.15em;
		margin-bottom: 2rem;
		color: var(--text-primary);
		text-align: center;
		text-transform: uppercase;
	}

	:global([data-theme='dark']) .section-title {
		font-weight: 700;
	}

	:global([data-theme='light']) .section-title {
		font-weight: 100;
	}

	.table-wrapper {
		overflow-x: auto;
		border: 1px solid var(--border-subtle);
		background: transparent;
	}

	.history-table {
		width: 100%;
		border-collapse: collapse;
	}

	.history-table thead {
		background: transparent;
		border-bottom: 1px solid var(--border-subtle);
	}

	.history-table th {
		padding: 1rem;
		text-align: left;
		font-weight: 300;
		color: var(--text-primary);
		font-size: 0.75rem;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		opacity: 0.8;
	}

	:global([data-theme='dark']) .history-table th {
		font-weight: 500;
	}

	:global([data-theme='light']) .history-table th {
		font-weight: 200;
	}

	.history-table tbody tr {
		border-bottom: 1px solid var(--border-subtle);
		transition: opacity 0.15s;
	}

	.history-table tbody tr:hover {
		opacity: 0.8;
	}

	.history-table td {
		padding: 1rem;
		font-size: 0.875rem;
		color: var(--text-primary);
		font-weight: 300;
	}

	.history-table td.positive {
		font-weight: 300;
		opacity: 0.9;
	}

	.history-table td.negative {
		font-weight: 300;
		opacity: 0.7;
	}

	.version-badge {
		display: inline-block;
		padding: 0.125rem 0.5rem;
		font-size: 0.625rem;
		background: transparent;
		color: var(--text-primary);
		border: 1px solid var(--border-subtle);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-weight: 300;
		opacity: 0.6;
	}

	:global([data-theme='light']) .version-badge {
		font-weight: 200;
	}

	.no-data {
		text-align: center;
		padding: 3rem;
		color: var(--text-primary);
		opacity: 0.6;
		font-size: 1rem;
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

		.stats-grid {
			grid-template-columns: repeat(2, 1fr);
			gap: 0.75rem;
		}

		.stat-card {
			padding: 1rem;
		}

		.stat-value {
			font-size: 1.25rem;
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
