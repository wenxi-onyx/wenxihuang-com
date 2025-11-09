<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { playersApi, type PlayerWithStats, type EloHistoryPoint, type PlayerMatch } from '$lib/api/client';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import LoginButton from '$lib/components/LoginButton.svelte';
	import Presence from '$lib/components/Presence.svelte';
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
	let matches: PlayerMatch[] = [];
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
			// Fetch player data, history, and matches
			const [allPlayers, playerHistory, playerMatches] = await Promise.all([
				playersApi.listPlayers(),
				playersApi.getPlayerHistory(playerId),
				playersApi.getPlayerMatches(playerId)
			]);

			player = allPlayers.find(p => p.id === playerId) || null;
			history = playerHistory;
			matches = playerMatches;

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

		// Get current theme
		const isDark = document.documentElement.getAttribute('data-theme') !== 'light';
		const textColor = isDark ? '#ffffff' : '#000000';
		const gridColor = isDark ? 'rgba(255, 255, 255, 0.08)' : 'rgba(0, 0, 0, 0.05)';
		const borderColor = isDark ? 'rgba(255, 255, 255, 0.15)' : 'rgba(0, 0, 0, 0.08)';

		// Define colors for seasons (cycle through these)
		const seasonColors = [
			{ border: 'rgba(59, 130, 246, 0.9)', bg: 'rgba(59, 130, 246, 0.05)' }, // blue
			{ border: 'rgba(168, 85, 247, 0.9)', bg: 'rgba(168, 85, 247, 0.05)' }, // purple
			{ border: 'rgba(34, 197, 94, 0.9)', bg: 'rgba(34, 197, 94, 0.05)' }, // green
			{ border: 'rgba(234, 179, 8, 0.9)', bg: 'rgba(234, 179, 8, 0.05)' }, // yellow
			{ border: 'rgba(239, 68, 68, 0.9)', bg: 'rgba(239, 68, 68, 0.05)' }, // red
			{ border: 'rgba(236, 72, 153, 0.9)', bg: 'rgba(236, 72, 153, 0.05)' }, // pink
			{ border: 'rgba(14, 165, 233, 0.9)', bg: 'rgba(14, 165, 233, 0.05)' }, // cyan
			{ border: 'rgba(249, 115, 22, 0.9)', bg: 'rgba(249, 115, 22, 0.05)' }, // orange
		];

		// Group history by season
		const seasonMap = new Map<string, { name: string; points: typeof history }>();
		history.forEach(point => {
			if (!seasonMap.has(point.season_id)) {
				seasonMap.set(point.season_id, { name: point.season_name, points: [] });
			}
			seasonMap.get(point.season_id)!.points.push(point);
		});

		// Create datasets for each season
		const datasets = [];
		const dates: Date[] = [];
		let globalIndex = 0;
		let seasonColorIndex = 0;

		for (const [seasonId, { name: seasonName, points }] of seasonMap) {
			const seasonColor = seasonColors[seasonColorIndex % seasonColors.length];
			seasonColorIndex++;

			const dataPoints = [];

			// Add starting point for this season (elo_before of first game)
			if (points.length > 0) {
				dates.push(new Date(points[0].created_at));
				dataPoints.push({
					x: globalIndex,
					y: points[0].elo_before
				});
				globalIndex++;
			}

			// Add all games in this season
			points.forEach(point => {
				dates.push(new Date(point.created_at));
				dataPoints.push({
					x: globalIndex,
					y: point.elo_after
				});
				globalIndex++;
			});

			datasets.push({
				label: seasonName,
				data: dataPoints,
				borderColor: seasonColor.border,
				backgroundColor: seasonColor.bg,
				borderWidth: 2,
				pointRadius: 2,
				pointHoverRadius: 5,
				pointHitRadius: 10,
				pointBackgroundColor: seasonColor.border,
				pointBorderColor: seasonColor.border,
				pointHoverBackgroundColor: textColor,
				pointHoverBorderColor: seasonColor.border,
				pointBorderWidth: 1,
				pointHoverBorderWidth: 2,
				fill: true,
				tension: 0.15,
				spanGaps: false
			});
		}

		chart = new Chart(ctx, {
			type: 'line',
			data: { datasets },
			options: {
				responsive: true,
				maintainAspectRatio: false,
				plugins: {
					title: {
						display: true,
						text: 'ELO RATING OVER TIME',
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
						display: seasonMap.size > 1,
						position: 'bottom',
						labels: {
							color: textColor,
							font: {
								size: 10,
								weight: isDark ? 400 : 300,
								family: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif'
							},
							padding: 12,
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
						callbacks: {
							title: (context) => {
								const xValue = context[0]?.parsed?.x;
								if (xValue !== null && xValue !== undefined && dates[xValue]) {
									return dates[xValue].toLocaleDateString('en-US', {
										year: 'numeric',
										month: 'short',
										day: 'numeric'
									}).toUpperCase();
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
							text: 'MATCH NUMBER',
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
							stepSize: 1,
							color: textColor,
							font: {
								size: 10,
								weight: 300,
								family: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif'
							},
							padding: 8,
							callback: function(value) {
								// Show tick labels for whole numbers only
								return Number.isInteger(value) ? value : '';
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
						},
						beginAtZero: false
					}
				},
				interaction: {
					mode: 'nearest',
					intersect: false,
					axis: 'x'
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

	function formatMatchDate(dateString: string): string {
		const date = new Date(dateString);
		const dateStr = date.toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric'
		}).toUpperCase();
		const timeStr = date.toLocaleTimeString('en-US', {
			hour: 'numeric',
			minute: '2-digit',
			hour12: true,
			timeZoneName: 'short'
		}).toUpperCase();
		return `${dateStr} ${timeStr}`;
	}

	function getMatchEloChange(matchId: string): number {
		const matchHistory = history.find(h => h.match_id === matchId);
		if (!matchHistory) return 0;
		return matchHistory.elo_after - matchHistory.elo_before;
	}

	function formatEloChange(change: number): string {
		if (change > 0) return `+${change.toFixed(1)}`;
		return change.toFixed(1);
	}
</script>

<svelte:head>
	<title>{player?.name || 'Player'} - ELO History</title>
</svelte:head>

<ThemeToggle />
<LoginButton />
<Presence />

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
				<button class="nav-link-btn" onclick={() => window.history.back()}>BACK</button>
			</nav>
		</header>

		<div class="stats-grid">
			<div class="stat-card">
				<div class="stat-label">Current ELO</div>
				<div class="stat-value primary">{player.current_elo.toFixed(1)}</div>
			</div>

			<div class="stat-card">
				<div class="stat-label">Peak ELO</div>
				<div class="stat-value">{getHighestElo().toFixed(1)}</div>
			</div>

			<div class="stat-card">
				<div class="stat-label">Lowest ELO</div>
				<div class="stat-value">{getLowestElo().toFixed(1)}</div>
			</div>

			<div class="stat-card">
				<div class="stat-label">Games Played</div>
				<div class="stat-value">{player.games_played}</div>
			</div>

			<div class="stat-card">
				<div class="stat-label">Win Rate</div>
				<div class="stat-value" class:positive={parseFloat(getWinRate(player)) > 50} class:negative={parseFloat(getWinRate(player)) <= 50 && player.games_played > 0}>{getWinRate(player)}%</div>
			</div>

			<div class="stat-card">
				<div class="stat-label">Wins / Losses</div>
				<div class="stat-value">
					<span class="wins">{player.wins}</span> / <span class="losses">{player.losses}</span>
				</div>
			</div>
		</div>

		{#if history.length > 0}
			<div class="chart-container">
				<canvas bind:this={chartCanvas}></canvas>
			</div>
		{/if}

		{#if matches.length > 0}
			<div class="history-section">
				<h2 class="section-title">Recent Match History</h2>
				<div class="table-wrapper">
					<table class="history-table">
						<thead>
							<tr>
								<th>Date</th>
								<th>Opponent</th>
								<th>Score</th>
								<th>ELO Change</th>
								<th>Season</th>
							</tr>
						</thead>
						<tbody>
							{#each matches.slice(0, 20) as match}
								{@const eloChange = getMatchEloChange(match.match_id)}
								<tr>
									<td>{formatMatchDate(match.submitted_at)}</td>
									<td>{match.opponent_name}</td>
									<td>{match.player_games_won} - {match.opponent_games_won}</td>
									<td class:positive={eloChange > 0} class:negative={eloChange < 0}>
										{formatEloChange(eloChange)}
									</td>
									<td><span class="version-badge">{match.season_name}</span></td>
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

	/* nav-link-btn styles now in shared buttons.css */

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
		grid-template-columns: repeat(3, 1fr);
		gap: 1rem;
		margin-bottom: 3rem;
	}

	.stat-card {
		background: transparent;
		border: 1px solid var(--border-subtle);
		padding: 1.5rem;
		text-align: center;
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
		color: rgba(34, 197, 94, 0.65);
	}

	:global([data-theme='light']) .stat-value.positive {
		color: rgba(22, 163, 74, 0.7);
	}

	.stat-value.negative {
		color: rgba(239, 68, 68, 0.8);
	}

	:global([data-theme='light']) .stat-value.negative {
		color: rgba(220, 38, 38, 0.8);
	}

	.wins {
		color: rgba(34, 197, 94, 0.65);
	}

	:global([data-theme='light']) .wins {
		color: rgba(22, 163, 74, 0.7);
	}

	.losses {
		color: rgba(239, 68, 68, 0.75);
	}

	:global([data-theme='light']) .losses {
		color: rgba(220, 38, 38, 0.75);
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
		text-align: center;
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
		text-align: center;
	}

	.history-table td.positive {
		font-weight: 300;
		color: rgba(34, 197, 94, 0.65);
	}

	:global([data-theme='light']) .history-table td.positive {
		color: rgba(22, 163, 74, 0.7);
	}

	.history-table td.negative {
		font-weight: 300;
		color: rgba(239, 68, 68, 0.8);
	}

	:global([data-theme='light']) .history-table td.negative {
		color: rgba(220, 38, 38, 0.8);
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
