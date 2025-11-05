<script lang="ts">
	import { onMount } from 'svelte';
	import { authStore } from '$lib/stores/auth';
	import { adminApi, seasonsApi, type Season, type CreateSeasonRequest, type SeasonPlayer } from '$lib/api/client';
	import { goto } from '$app/navigation';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import LoginButton from '$lib/components/LoginButton.svelte';
	import Toast, { showToast } from '$lib/components/Toast.svelte';

	const user = $derived($authStore.user);
	let seasons: Season[] = $state([]);
	let loading = $state(true);
	let authChecked = $state(false);
	let creating = $state(false);
	let showCreateForm = $state(false);
	let operatingSeasonId = $state<string | null>(null);
	let managingPlayersForSeasonId = $state<string | null>(null);
	let seasonPlayers = $state<SeasonPlayer[]>([]);
	let availablePlayers = $state<SeasonPlayer[]>([]);
	let loadingPlayers = $state(false);

	// Create form state
	let newSeasonName = $state('');
	let newSeasonDescription = $state('');
	let newSeasonStartDate = $state('');
	let newSeasonStartingElo = $state(1000);
	let newSeasonKFactor = $state(32);

	onMount(async () => {
		// Wait for auth to load first to avoid race condition
		await authStore.checkAuth();
		authChecked = true;

		// Check if user is admin
		if (!user || user.role !== 'admin') {
			showToast('Admin access required', 'error');
			goto('/table-tennis');
			return;
		}

		await loadSeasons();
	});

	async function loadSeasons() {
		try {
			loading = true;
			seasons = await seasonsApi.listSeasons();
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to load seasons', 'error');
		} finally {
			loading = false;
		}
	}

	async function handleCreateSeason(e: Event) {
		e.preventDefault();

		if (!newSeasonName.trim() || !newSeasonStartDate) {
			showToast('Please fill in all required fields', 'error');
			return;
		}

		creating = true;

		try {
			// Parse date and set to 12:01 AM Pacific time (08:01 UTC for PST)
			// This ensures seasons start at the beginning of the day in Pacific time
			const [year, month, day] = newSeasonStartDate.split('-');
			const utcDate = new Date(Date.UTC(parseInt(year), parseInt(month) - 1, parseInt(day), 8, 1, 0));

			const seasonData: CreateSeasonRequest = {
				name: newSeasonName.trim(),
				description: newSeasonDescription.trim() || undefined,
				start_date: utcDate.toISOString(),
				starting_elo: newSeasonStartingElo,
				k_factor: newSeasonKFactor,
			};

			await adminApi.createSeason(seasonData);
			showToast(`Season '${seasonData.name}' created successfully!`, 'success');

			// Reset form
			newSeasonName = '';
			newSeasonDescription = '';
			newSeasonStartDate = '';
			newSeasonStartingElo = 1000;
			newSeasonKFactor = 32;
			showCreateForm = false;

			// Reload seasons
			await loadSeasons();
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to create season', 'error');
		} finally {
			creating = false;
		}
	}

	async function handleActivateSeason(seasonId: string, seasonName: string) {
		if (operatingSeasonId) return; // Prevent multiple operations
		if (!confirm(`Activate season '${seasonName}'? This will deactivate all other seasons.`)) {
			return;
		}

		operatingSeasonId = seasonId;
		try {
			await adminApi.activateSeason(seasonId);
			showToast(`Season '${seasonName}' activated`, 'success');
			await loadSeasons();
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to activate season', 'error');
		} finally {
			operatingSeasonId = null;
		}
	}

	async function handleRecalculateSeason(seasonId: string, seasonName: string) {
		if (operatingSeasonId) return; // Prevent multiple operations
		if (!confirm(`Recalculate ELO for season '${seasonName}'? This will replay all games.`)) {
			return;
		}

		operatingSeasonId = seasonId;
		try {
			const response = await adminApi.recalculateSeason(seasonId);
			showToast(response.message, 'success');
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to recalculate season', 'error');
		} finally {
			operatingSeasonId = null;
		}
	}

	async function handleDeleteSeason(seasonId: string, seasonName: string) {
		if (operatingSeasonId) return; // Prevent multiple operations
		if (!confirm(`Delete season '${seasonName}'? This will delete all associated data and reassign games. This cannot be undone!`)) {
			return;
		}

		operatingSeasonId = seasonId;
		try {
			const response = await adminApi.deleteSeason(seasonId);
			showToast(response.message, 'success');
			await loadSeasons();
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to delete season', 'error');
		} finally {
			operatingSeasonId = null;
		}
	}

	function formatDate(dateString: string): string {
		return new Date(dateString).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric'
		});
	}

	async function loadSeasonPlayers(seasonId: string) {
		loadingPlayers = true;
		try {
			const [players, available] = await Promise.all([
				adminApi.getSeasonPlayers(seasonId),
				adminApi.getAvailablePlayers(seasonId)
			]);
			seasonPlayers = players;
			availablePlayers = available;
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to load players', 'error');
		} finally {
			loadingPlayers = false;
		}
	}

	async function toggleManagePlayers(seasonId: string, seasonName: string) {
		if (managingPlayersForSeasonId === seasonId) {
			managingPlayersForSeasonId = null;
			seasonPlayers = [];
			availablePlayers = [];
		} else {
			managingPlayersForSeasonId = seasonId;
			await loadSeasonPlayers(seasonId);
		}
	}

	async function handleAddPlayer(seasonId: string, playerId: string, playerName: string) {
		try {
			await adminApi.addPlayerToSeason(seasonId, playerId);
			showToast(`Added ${playerName} to season`, 'success');
			await loadSeasonPlayers(seasonId);
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to add player', 'error');
		}
	}

	async function handleRemovePlayer(seasonId: string, playerId: string, playerName: string) {
		if (!confirm(`Remove ${playerName} from this season?`)) {
			return;
		}
		try {
			await adminApi.removePlayerFromSeason(seasonId, playerId);
			showToast(`Removed ${playerName} from season`, 'success');
			await loadSeasonPlayers(seasonId);
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to remove player', 'error');
		}
	}
</script>

<svelte:head>
	<title>Season Management</title>
</svelte:head>

<ThemeToggle />
<LoginButton />
<Toast />

<div class="container">
	<header class="page-header">
		<h1>Season Management</h1>
		<nav class="nav-links">
			<a href="/table-tennis">BACK TO LEADERBOARD</a>
		</nav>
	</header>

	{#if !authChecked}
		<div class="loading">Checking permissions...</div>
	{:else if loading}
		<div class="loading">Loading seasons...</div>
	{:else}
		<div class="content">
			<div class="section">
				<div class="section-header">
					<h2>CREATE NEW SEASON</h2>
					<button
						class="btn-toggle"
						onclick={() => showCreateForm = !showCreateForm}
					>
						{showCreateForm ? 'CANCEL' : 'NEW SEASON'}
					</button>
				</div>

				{#if showCreateForm}
					<form class="create-form" onsubmit={handleCreateSeason}>
						<div class="form-group">
							<label for="name">SEASON NAME *</label>
							<input
								type="text"
								id="name"
								bind:value={newSeasonName}
								placeholder="e.g., Spring 2025"
								required
								maxlength="100"
							/>
						</div>

						<div class="form-group">
							<label for="description">DESCRIPTION</label>
							<input
								type="text"
								id="description"
								bind:value={newSeasonDescription}
								placeholder="Optional description"
								maxlength="500"
							/>
						</div>

						<div class="form-row">
							<div class="form-group">
								<label for="start_date">START DATE *</label>
								<input
									type="date"
									id="start_date"
									bind:value={newSeasonStartDate}
									required
								/>
							</div>

							<div class="form-group">
								<label for="starting_elo">STARTING ELO</label>
								<input
									type="number"
									id="starting_elo"
									bind:value={newSeasonStartingElo}
									min="100"
									max="3000"
									step="50"
								/>
							</div>

							<div class="form-group">
								<label for="k_factor">K-FACTOR</label>
								<input
									type="number"
									id="k_factor"
									bind:value={newSeasonKFactor}
									min="1"
									max="100"
									step="1"
								/>
							</div>
						</div>

						<button type="submit" class="btn-submit" disabled={creating}>
							{creating ? 'CREATING...' : 'CREATE SEASON'}
						</button>
					</form>
				{/if}
			</div>

			<div class="section">
				<h2>EXISTING SEASONS ({seasons.length})</h2>

				{#if seasons.length === 0}
					<div class="empty-state">No seasons found. Create one to get started.</div>
				{:else}
					<div class="seasons-list">
						{#each seasons as season}
							<div class="season-card" class:active={season.is_active}>
								<div class="season-header">
									<div class="season-info">
										<h3>
											{season.name}
											{#if season.is_active}
												<span class="badge">ACTIVE</span>
											{/if}
										</h3>
										{#if season.description}
											<p class="description">{season.description}</p>
										{/if}
									</div>
									<div class="season-meta">
										<div class="meta-item">
											<span class="label">START DATE</span>
											<span class="value">{formatDate(season.start_date)}</span>
										</div>
										<div class="meta-item">
											<span class="label">STARTING ELO</span>
											<span class="value">{season.starting_elo}</span>
										</div>
										<div class="meta-item">
											<span class="label">K-FACTOR</span>
											<span class="value">{season.k_factor}</span>
										</div>
									</div>
								</div>

								<div class="season-actions">
									{#if !season.is_active}
										<button
											class="btn-action"
											onclick={() => handleActivateSeason(season.id, season.name)}
											disabled={operatingSeasonId === season.id}
										>
											{operatingSeasonId === season.id ? 'ACTIVATING...' : 'ACTIVATE'}
										</button>
									{/if}
									<button
										class="btn-action"
										onclick={() => toggleManagePlayers(season.id, season.name)}
									>
										{managingPlayersForSeasonId === season.id ? 'CLOSE PLAYERS' : 'MANAGE PLAYERS'}
									</button>
									<button
										class="btn-action"
										onclick={() => handleRecalculateSeason(season.id, season.name)}
										disabled={operatingSeasonId === season.id}
									>
										{operatingSeasonId === season.id ? 'RECALCULATING...' : 'RECALCULATE'}
									</button>
									<button
										class="btn-action btn-danger"
										onclick={() => handleDeleteSeason(season.id, season.name)}
										disabled={operatingSeasonId === season.id}
									>
										{operatingSeasonId === season.id ? 'DELETING...' : 'DELETE'}
									</button>
								</div>

								{#if managingPlayersForSeasonId === season.id}
									<div class="player-management">
										{#if loadingPlayers}
											<div class="loading-players">Loading players...</div>
										{:else}
											<div class="player-section">
												<h3>INCLUDED PLAYERS ({seasonPlayers.filter(p => p.is_included).length})</h3>
												<div class="player-list">
													{#each seasonPlayers.filter(p => p.is_included) as player}
														<div class="player-item" class:inactive={!player.is_active}>
															<span class="player-name">
																{player.player_name}
																{#if !player.is_active}
																	<span class="player-badge">INACTIVE</span>
																{/if}
															</span>
															<button
																class="btn-remove"
																onclick={() => handleRemovePlayer(season.id, player.player_id, player.player_name)}
															>
																REMOVE
															</button>
														</div>
													{:else}
														<div class="empty-message">No players included in this season</div>
													{/each}
												</div>
											</div>

											<div class="player-section">
												<h3>EXCLUDED PLAYERS ({seasonPlayers.filter(p => !p.is_included).length})</h3>
												<div class="player-list">
													{#each seasonPlayers.filter(p => !p.is_included) as player}
														<div class="player-item" class:inactive={!player.is_active}>
															<span class="player-name">
																{player.player_name}
																{#if !player.is_active}
																	<span class="player-badge">INACTIVE</span>
																{/if}
															</span>
															<button
																class="btn-add"
																onclick={() => handleAddPlayer(season.id, player.player_id, player.player_name)}
															>
																ADD
															</button>
														</div>
													{:else}
														<div class="empty-message">No excluded players</div>
													{/each}
												</div>
											</div>

											{#if availablePlayers.length > 0}
												<div class="player-section">
													<h3>NOT IN SEASON ({availablePlayers.length})</h3>
													<div class="player-list">
														{#each availablePlayers as player}
															<div class="player-item" class:inactive={!player.is_active}>
																<span class="player-name">
																	{player.player_name}
																	{#if !player.is_active}
																		<span class="player-badge">INACTIVE</span>
																	{/if}
																</span>
																<button
																	class="btn-add"
																	onclick={() => handleAddPlayer(season.id, player.player_id, player.player_name)}
																>
																	ADD
																</button>
															</div>
														{/each}
													</div>
												</div>
											{/if}
										{/if}
									</div>
								{/if}
							</div>
						{/each}
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>

<style>
	.container {
		max-width: 1200px;
		margin: 0 auto;
		padding: 6rem 1rem 2rem 1rem;
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 3rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.1);
	}

	h1 {
		font-size: clamp(1.5rem, 4vw, 2.5rem);
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		margin: 0;
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

	.content {
		display: flex;
		flex-direction: column;
		gap: 3rem;
	}

	.section {
		background: transparent;
		border: 1px solid rgba(255, 255, 255, 0.1);
		padding: 2rem;
	}

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 2rem;
	}

	h2 {
		font-size: 1.25rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		margin: 0 0 1.5rem 0;
	}

	.btn-toggle {
		padding: 0.5rem 1rem;
		background: transparent;
		border: 1px solid rgba(255, 255, 255, 0.2);
		color: inherit;
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.btn-toggle:hover {
		border-color: rgba(255, 255, 255, 0.4);
		background: rgba(255, 255, 255, 0.05);
	}

	.create-form {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
		padding-top: 1.5rem;
		border-top: 1px solid rgba(255, 255, 255, 0.1);
	}

	.form-row {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1.5rem;
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	label {
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		opacity: 0.7;
	}

	input {
		padding: 0.75rem;
		background: transparent;
		border: 1px solid rgba(255, 255, 255, 0.2);
		color: inherit;
		font-family: inherit;
		font-size: 0.875rem;
		font-weight: 300;
		transition: border-color 0.2s ease;
	}

	input:focus {
		outline: none;
		border-color: rgba(255, 255, 255, 0.4);
	}

	.btn-submit {
		padding: 1rem;
		background: transparent;
		border: 1px solid rgba(255, 255, 255, 0.3);
		color: inherit;
		font-size: 0.875rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.btn-submit:hover:not(:disabled) {
		border-color: rgba(255, 255, 255, 0.5);
		background: rgba(255, 255, 255, 0.05);
	}

	.btn-submit:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.loading, .empty-state {
		text-align: center;
		padding: 3rem;
		opacity: 0.7;
		font-weight: 300;
		letter-spacing: 0.05em;
	}

	.seasons-list {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.season-card {
		background: transparent;
		border: 1px solid rgba(255, 255, 255, 0.1);
		padding: 1.5rem;
		transition: border-color 0.2s ease;
	}

	.season-card.active {
		border-color: rgba(255, 255, 255, 0.3);
	}

	.season-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 2rem;
		margin-bottom: 1.5rem;
		flex-wrap: wrap;
	}

	.season-info {
		flex: 1;
		min-width: 200px;
	}

	.season-info h3 {
		font-size: 1.125rem;
		font-weight: 300;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		margin: 0 0 0.5rem 0;
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.badge {
		font-size: 0.625rem;
		padding: 0.25rem 0.5rem;
		border: 1px solid rgba(255, 255, 255, 0.3);
		letter-spacing: 0.1em;
	}

	.description {
		font-size: 0.875rem;
		font-weight: 300;
		opacity: 0.7;
		margin: 0;
	}

	.season-meta {
		display: flex;
		gap: 2rem;
		flex-wrap: wrap;
	}

	.meta-item {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.meta-item .label {
		font-size: 0.625rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		opacity: 0.5;
	}

	.meta-item .value {
		font-size: 0.875rem;
		font-weight: 300;
	}

	.season-actions {
		display: flex;
		gap: 1rem;
		flex-wrap: wrap;
	}

	.btn-action {
		padding: 0.5rem 1rem;
		background: transparent;
		border: 1px solid rgba(255, 255, 255, 0.2);
		color: inherit;
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.btn-action:hover:not(:disabled) {
		border-color: rgba(255, 255, 255, 0.4);
		background: rgba(255, 255, 255, 0.05);
	}

	.btn-action:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-danger:hover:not(:disabled) {
		border-color: rgba(255, 100, 100, 0.5);
		background: rgba(255, 100, 100, 0.05);
		color: rgb(255, 150, 150);
	}

	.player-management {
		margin-top: 1.5rem;
		padding-top: 1.5rem;
		border-top: 1px solid rgba(255, 255, 255, 0.1);
	}

	.loading-players {
		text-align: center;
		padding: 2rem;
		opacity: 0.7;
	}

	.player-section {
		margin-bottom: 2rem;
	}

	.player-section h3 {
		font-size: 0.875rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		margin: 0 0 1rem 0;
		opacity: 0.8;
	}

	.player-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.player-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem 1rem;
		background: transparent;
		border: 1px solid rgba(255, 255, 255, 0.1);
		transition: border-color 0.2s ease;
	}

	.player-item:hover {
		border-color: rgba(255, 255, 255, 0.2);
	}

	.player-item.inactive {
		opacity: 0.6;
	}

	.player-name {
		font-size: 0.875rem;
		font-weight: 300;
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.player-badge {
		font-size: 0.625rem;
		padding: 0.125rem 0.5rem;
		border: 1px solid rgba(255, 255, 255, 0.2);
		letter-spacing: 0.1em;
		opacity: 0.6;
	}

	.btn-add,
	.btn-remove {
		padding: 0.375rem 0.75rem;
		background: transparent;
		border: 1px solid rgba(255, 255, 255, 0.2);
		color: inherit;
		font-size: 0.625rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.btn-add:hover {
		border-color: rgba(100, 255, 100, 0.5);
		background: rgba(100, 255, 100, 0.05);
		color: rgb(150, 255, 150);
	}

	.btn-remove:hover {
		border-color: rgba(255, 100, 100, 0.5);
		background: rgba(255, 100, 100, 0.05);
		color: rgb(255, 150, 150);
	}

	.empty-message {
		text-align: center;
		padding: 2rem;
		opacity: 0.5;
		font-size: 0.875rem;
	}

	@media (max-width: 768px) {
		.page-header {
			flex-direction: column;
			gap: 1rem;
			align-items: flex-start;
		}

		.season-header {
			flex-direction: column;
		}

		.form-row {
			grid-template-columns: 1fr;
		}

		.player-item {
			flex-direction: column;
			align-items: flex-start;
			gap: 0.5rem;
		}
	}
</style>
