<script lang="ts">
	import { onMount } from 'svelte';
	import { authStore } from '$lib/stores/auth';
	import { adminApi, seasonsApi, playersApi, type Season, type CreateSeasonRequest, type SeasonPlayer, type PlayerWithStats, type EloConfiguration } from '$lib/api/client';
	import { goto } from '$app/navigation';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import LoginButton from '$lib/components/LoginButton.svelte';
	import { showToast } from '$lib/components/Toast.svelte';
	import ConfirmModal, { confirm } from '$lib/components/ConfirmModal.svelte';

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
	let eloConfigurations = $state<EloConfiguration[]>([]);
	let selectedEloVersions = $state<Map<string, string | null>>(new Map()); // Track selected ELO version per season

	// Track pending player changes for each season
	let pendingChanges = $state<Map<string, { toAdd: Set<string>, toRemove: Set<string> }>>(new Map());

	// Create form state
	let newSeasonName = $state('');
	let newSeasonDescription = $state('');
	let newSeasonStartDate = $state('');
	let newSeasonStartingElo = $state(1000);
	let newSeasonKFactor = $state(32);
	let newSeasonEloVersion = $state<string | null>(null);
	let allPlayersForCreate = $state<PlayerWithStats[]>([]);
	let selectedPlayerIds = $state<Set<string>>(new Set());
	let loadingCreatePlayers = $state(false);

	onMount(async () => {
		// Wait for auth to load first to avoid race condition
		const currentUser = await authStore.checkAuth();
		authChecked = true;

		// Check if user is authenticated
		if (!currentUser) {
			showToast('You must be logged in to access this page', 'error');
			goto('/login');
			return;
		}

		await Promise.all([loadSeasons(), loadEloConfigurations()]);
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

	async function loadPlayersForCreate() {
		if (allPlayersForCreate.length > 0) return; // Already loaded

		loadingCreatePlayers = true;
		try {
			const players = await playersApi.listPlayers();
			allPlayersForCreate = players.filter(p => p.is_active);
			// By default, select all active players
			selectedPlayerIds = new Set(allPlayersForCreate.map(p => p.id));
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to load players', 'error');
		} finally {
			loadingCreatePlayers = false;
		}
	}

	function handleEloConfigChange(versionName: string) {
		newSeasonEloVersion = versionName;

		// Auto-populate starting ELO and K-factor from the selected configuration
		const config = eloConfigurations.find(c => c.version_name === versionName);
		if (config) {
			newSeasonStartingElo = config.starting_elo;
			newSeasonKFactor = config.k_factor;
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
				elo_version: newSeasonEloVersion || undefined,
				player_ids: selectedPlayerIds.size > 0 ? Array.from(selectedPlayerIds) : undefined,
			};

			await adminApi.createSeason(seasonData);
			showToast(`Season '${seasonData.name}' created successfully!`, 'success');

			// Reset form
			newSeasonName = '';
			newSeasonDescription = '';
			newSeasonStartDate = '';
			newSeasonStartingElo = 1000;
			newSeasonKFactor = 32;
			newSeasonEloVersion = null;
			allPlayersForCreate = [];
			selectedPlayerIds = new Set();
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

		const confirmed = await confirm({
			title: 'Activate Season',
			message: `Activate season '${seasonName}'?\n\nThis will deactivate all other seasons.`,
			confirmText: 'ACTIVATE',
			confirmStyle: 'primary'
		});

		if (!confirmed) return;

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

		const confirmed = await confirm({
			title: 'Recalculate Season',
			message: `Recalculate ELO for season '${seasonName}'?\n\nThis will replay all games with the current ELO algorithm.`,
			confirmText: 'RECALCULATE',
			confirmStyle: 'warning'
		});

		if (!confirmed) return;

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

		const confirmed = await confirm({
			title: 'Delete Season',
			message: `Delete season '${seasonName}'?\n\nThis will delete all associated data and reassign games to other seasons.\n\nThis action cannot be undone!`,
			confirmText: 'DELETE',
			confirmStyle: 'danger'
		});

		if (!confirmed) return;

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

	// Helper to get or initialize pending changes for a season
	function getPendingChanges(seasonId: string) {
		if (!pendingChanges.has(seasonId)) {
			pendingChanges.set(seasonId, { toAdd: new Set(), toRemove: new Set() });
		}
		return pendingChanges.get(seasonId)!;
	}

	// Check if a season has pending changes
	function hasPendingChanges(seasonId: string): boolean {
		const changes = pendingChanges.get(seasonId);
		return changes ? (changes.toAdd.size > 0 || changes.toRemove.size > 0) : false;
	}

	// Check if a player has a pending change
	function hasPendingChange(seasonId: string, playerId: string): 'add' | 'remove' | null {
		const changes = pendingChanges.get(seasonId);
		if (!changes) return null;
		if (changes.toAdd.has(playerId)) return 'add';
		if (changes.toRemove.has(playerId)) return 'remove';
		return null;
	}

	async function loadEloConfigurations() {
		try {
			eloConfigurations = await adminApi.listEloConfigurations();
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to load ELO configurations', 'error');
		}
	}

	async function toggleManagePlayers(seasonId: string, seasonName: string) {
		if (managingPlayersForSeasonId === seasonId) {
			managingPlayersForSeasonId = null;
			seasonPlayers = [];
			availablePlayers = [];
			// Clear pending changes when closing
			pendingChanges.delete(seasonId);
			selectedEloVersions.delete(seasonId);
			// Trigger reactivity
			selectedEloVersions = new Map(selectedEloVersions);
		} else {
			managingPlayersForSeasonId = seasonId;
			await loadSeasonPlayers(seasonId);
			// Initialize pending changes
			getPendingChanges(seasonId);
			// Initialize selected ELO version with current value
			const season = seasons.find(s => s.id === seasonId);
			if (season) {
				selectedEloVersions.set(seasonId, season.elo_version);
				// Trigger reactivity
				selectedEloVersions = new Map(selectedEloVersions);
			}
		}
	}

	// Check if ELO version has changed for a season
	function hasEloVersionChanged(seasonId: string): boolean {
		const season = seasons.find(s => s.id === seasonId);
		if (!season) return false;
		const selected = selectedEloVersions.get(seasonId);
		// Check if we have a selection and if it's different from the current value
		return selected !== undefined && selected !== season.elo_version;
	}

	function handleEloVersionChange(seasonId: string, newVersion: string) {
		selectedEloVersions.set(seasonId, newVersion || null);
		// Trigger reactivity
		selectedEloVersions = new Map(selectedEloVersions);
	}

	function handleAddPlayer(seasonId: string, playerId: string, playerName: string) {
		const changes = getPendingChanges(seasonId);

		// If this player was marked for removal, just unmark it
		if (changes.toRemove.has(playerId)) {
			changes.toRemove.delete(playerId);
		} else {
			// Otherwise, mark it for addition
			changes.toAdd.add(playerId);
		}

		// Trigger reactivity
		pendingChanges = new Map(pendingChanges);
	}

	function handleRemovePlayer(seasonId: string, playerId: string, playerName: string) {
		const changes = getPendingChanges(seasonId);

		// If this player was marked for addition, just unmark it
		if (changes.toAdd.has(playerId)) {
			changes.toAdd.delete(playerId);
		} else {
			// Otherwise, mark it for removal
			changes.toRemove.add(playerId);
		}

		// Trigger reactivity
		pendingChanges = new Map(pendingChanges);
	}

	async function handleSaveAndRecalculate(seasonId: string, seasonName: string) {
		if (operatingSeasonId) return; // Prevent multiple operations

		const changes = getPendingChanges(seasonId);
		const hasPlayerChanges = changes.toAdd.size > 0 || changes.toRemove.size > 0;
		const hasEloChange = hasEloVersionChanged(seasonId);

		if (!hasPlayerChanges && !hasEloChange) {
			showToast('No changes to save', 'error');
			return;
		}

		// Build detailed message about what will change
		let changesSummary = '';
		if (hasPlayerChanges) {
			if (changes.toAdd.size > 0) {
				changesSummary += `\n• Adding ${changes.toAdd.size} player${changes.toAdd.size > 1 ? 's' : ''}`;
			}
			if (changes.toRemove.size > 0) {
				changesSummary += `\n• Removing ${changes.toRemove.size} player${changes.toRemove.size > 1 ? 's' : ''}`;
			}
		}
		if (hasEloChange) {
			const newVersion = selectedEloVersions.get(seasonId);
			changesSummary += `\n• Changing ELO algorithm to: ${newVersion || 'Season defaults'}`;
		}

		const confirmed = await confirm({
			title: 'Save and Recalculate',
			message: `Save changes and recalculate ELO for season '${seasonName}'?${changesSummary}\n\nThis will replay all games with the updated configuration.`,
			confirmText: 'SAVE & RECALCULATE',
			confirmStyle: 'warning'
		});

		if (!confirmed) return;

		operatingSeasonId = seasonId;
		try {
			// Apply all player additions
			if (hasPlayerChanges) {
				for (const playerId of changes.toAdd) {
					await adminApi.addPlayerToSeason(seasonId, playerId);
				}

				// Apply all player removals
				for (const playerId of changes.toRemove) {
					await adminApi.removePlayerFromSeason(seasonId, playerId);
				}

				// Clear pending player changes
				pendingChanges.delete(seasonId);
				pendingChanges = new Map(pendingChanges);

				// Reload player list
				await loadSeasonPlayers(seasonId);
			}

			// Update ELO version if changed
			if (hasEloChange) {
				const newVersion = selectedEloVersions.get(seasonId);
				await adminApi.updateSeasonEloVersion(seasonId, newVersion || null);
			}

			// Now recalculate
			const response = await adminApi.recalculateSeason(seasonId);
			showToast(response.message, 'success');
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Failed to save and recalculate', 'error');
		} finally {
			operatingSeasonId = null;
			await loadSeasons(); // Reload to get updated data
		}
	}
</script>

<svelte:head>
	<title>Season Management</title>
</svelte:head>

<ThemeToggle />
<LoginButton />
<ConfirmModal />

<div class="container">
	<header class="page-header">
		<h1>Season Management</h1>
		<nav class="nav-links">
			<a href="/table-tennis">LEADERBOARD</a>
			<a href="/table-tennis/algorithms">ELO ALGORITHMS</a>
			<a href="/table-tennis/players">PLAYERS</a>
			<button class="nav-link-btn" onclick={() => window.history.back()}>BACK</button>
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
						onclick={() => {
							showCreateForm = !showCreateForm;
							if (showCreateForm) {
								loadPlayersForCreate();
								// Auto-select first available ELO configuration
								if (eloConfigurations.length > 0 && !newSeasonEloVersion) {
									// Prefer active configuration, otherwise use first one
									const activeConfig = eloConfigurations.find(c => c.is_active);
									const defaultConfig = activeConfig || eloConfigurations[0];
									handleEloConfigChange(defaultConfig.version_name);
								}
							}
						}}
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

						<div class="form-group">
							<label for="elo_config">
								ELO ALGORITHM
								<span class="label-hint">(optional - auto-fills starting ELO & K-factor)</span>
							</label>
							<select
								id="elo_config"
								bind:value={newSeasonEloVersion}
								onchange={(e) => {
									const value = e.currentTarget.value;
									if (value) {
										handleEloConfigChange(value);
									}
								}}
							>
								<option value={null}>None (Custom)</option>
								{#each eloConfigurations as config (config.version_name)}
									<option value={config.version_name}>
										{config.version_name}
										{#if config.description}
											- {config.description}
										{/if}
									</option>
								{/each}
							</select>
						</div>

						<div class="form-group player-selection">
							<label>SELECT PLAYERS ({selectedPlayerIds.size} selected)</label>
							{#if loadingCreatePlayers}
								<div class="loading-players-create">Loading players...</div>
							{:else if allPlayersForCreate.length === 0}
								<div class="no-players">No active players found</div>
							{:else}
								<!-- Debug: {JSON.stringify(Array.from(selectedPlayerIds))} -->
								<div class="player-select-controls">
									<button
										type="button"
										class="btn-select-all"
										onclick={() => selectedPlayerIds = new Set(allPlayersForCreate.map(p => p.id))}
									>
										SELECT ALL
									</button>
									<button
										type="button"
										class="btn-select-none"
										onclick={() => selectedPlayerIds = new Set()}
									>
										CLEAR ALL
									</button>
								</div>
								<div class="player-selection-list">
									{#each allPlayersForCreate as player (player.id)}
										{@const isSelected = selectedPlayerIds.has(player.id)}
										<label class="player-checkbox">
											<input
												type="checkbox"
												checked={isSelected}
												onclick={(e) => {
													e.stopPropagation();
													const newSet = new Set(selectedPlayerIds);
													if (e.currentTarget.checked) {
														newSet.add(player.id);
													} else {
														newSet.delete(player.id);
													}
													selectedPlayerIds = newSet;
													console.log('Selected players:', Array.from(newSet));
												}}
											/>
											<span class="player-checkbox-label">
												{player.name}
												<span class="player-stats">
													(ELO: {player.current_elo.toFixed(0)},
													{player.games_played} games)
												</span>
											</span>
										</label>
									{/each}
								</div>
							{/if}
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
									<div class="actions-left">
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
											{managingPlayersForSeasonId === season.id ? 'CLOSE' : 'DETAILS'}
										</button>
									</div>
									<div class="actions-right">
										<button
											class="btn-action btn-primary"
											onclick={() => handleSaveAndRecalculate(season.id, season.name)}
											disabled={operatingSeasonId === season.id || (!hasPendingChanges(season.id) && !hasEloVersionChanged(season.id))}
										>
											{operatingSeasonId === season.id ? 'SAVING...' : 'SAVE AND RECALCULATE'}
										</button>
										<button
											class="btn-action btn-danger"
											onclick={() => handleDeleteSeason(season.id, season.name)}
											disabled={operatingSeasonId === season.id}
										>
											{operatingSeasonId === season.id ? 'DELETING...' : 'DELETE'}
										</button>
									</div>
								</div>

								{#if managingPlayersForSeasonId === season.id}
									<div class="player-management">
										{#if loadingPlayers}
											<div class="loading-players">Loading players...</div>
										{:else}
											<!-- ELO Algorithm Selector -->
											<div class="elo-selector-section">
												<h3>ELO ALGORITHM</h3>
												<div class="elo-selector-container">
													<select
														class="elo-selector"
														value={selectedEloVersions.get(season.id) || ''}
														onchange={(e) => handleEloVersionChange(season.id, e.currentTarget.value)}
													>
														<option value="">No specific algorithm (use season values)</option>
														{#each eloConfigurations as config}
															<option value={config.version_name}>
																{config.version_name} - {config.description || 'K=' + config.k_factor}
															</option>
														{/each}
													</select>
													{#if hasEloVersionChanged(season.id)}
														<span class="elo-changed-indicator">Changed (requires recalculation)</span>
													{/if}
												</div>
											</div>

											<div class="player-section">
												<h3>INCLUDED PLAYERS ({seasonPlayers.filter(p => p.is_included).length})</h3>
												<div class="player-list">
													{#each seasonPlayers.filter(p => p.is_included) as player}
														{@const pendingChange = hasPendingChange(season.id, player.player_id)}
														<div
															class="player-item"
															class:inactive={!player.is_active}
															class:pending-remove={pendingChange === 'remove'}
														>
															<span class="player-name">
																{player.player_name}
																{#if !player.is_active}
																	<span class="player-badge">INACTIVE</span>
																{/if}
																{#if pendingChange === 'remove'}
																	<span class="player-badge pending">WILL BE REMOVED</span>
																{/if}
															</span>
															<button
																class="btn-remove"
																onclick={() => handleRemovePlayer(season.id, player.player_id, player.player_name)}
															>
																{pendingChange === 'remove' ? 'UNDO' : 'REMOVE'}
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
														{@const pendingChange = hasPendingChange(season.id, player.player_id)}
														<div
															class="player-item"
															class:inactive={!player.is_active}
															class:pending-add={pendingChange === 'add'}
														>
															<span class="player-name">
																{player.player_name}
																{#if !player.is_active}
																	<span class="player-badge">INACTIVE</span>
																{/if}
																{#if pendingChange === 'add'}
																	<span class="player-badge pending">WILL BE ADDED</span>
																{/if}
															</span>
															<button
																class="btn-add"
																onclick={() => handleAddPlayer(season.id, player.player_id, player.player_name)}
															>
																{pendingChange === 'add' ? 'UNDO' : 'ADD'}
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
															{@const pendingChange = hasPendingChange(season.id, player.player_id)}
															<div
																class="player-item"
																class:inactive={!player.is_active}
																class:pending-add={pendingChange === 'add'}
															>
																<span class="player-name">
																	{player.player_name}
																	{#if !player.is_active}
																		<span class="player-badge">INACTIVE</span>
																	{/if}
																	{#if pendingChange === 'add'}
																		<span class="player-badge pending">WILL BE ADDED</span>
																	{/if}
																</span>
																<button
																	class="btn-add"
																	onclick={() => handleAddPlayer(season.id, player.player_id, player.player_name)}
																>
																	{pendingChange === 'add' ? 'UNDO' : 'ADD'}
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
		color: var(--text-primary);
		opacity: 0.7;
		transition: opacity 0.2s ease;
	}

	.nav-links a:hover {
		opacity: 1;
	}

	/* nav-link-btn styles now in shared buttons.css */

	.content {
		display: flex;
		flex-direction: column;
		gap: 3rem;
	}

	.section {
		background: transparent;
		border: 1px solid var(--border-subtle);
		padding: 2rem;
	}

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 2rem;
	}

	.section-header h2 {
		margin: 0;
	}

	h2 {
		font-size: 1.25rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		margin: 0 0 1.5rem 0;
		color: var(--text-primary);
	}

	.btn-toggle {
		padding: 0.5rem 1rem;
		background: transparent;
		border: 1px solid var(--border-subtle);
		color: var(--text-primary);
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.btn-toggle:hover {
		border-color: var(--border-active);
		background: var(--border-subtle);
	}

	.create-form {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
		padding-top: 1.5rem;
		border-top: 1px solid var(--border-subtle);
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
		color: var(--text-primary);
		opacity: 0.7;
	}

	.label-hint {
		text-transform: none;
		font-size: 0.65rem;
		opacity: 0.5;
		font-style: italic;
		letter-spacing: 0.05em;
	}

	input,
	select {
		padding: 0.75rem;
		background: transparent;
		border: 1px solid var(--border-subtle);
		color: var(--text-primary);
		font-family: inherit;
		font-size: 0.875rem;
		font-weight: 300;
		transition: border-color 0.2s ease;
	}

	input:focus,
	select:focus {
		outline: none;
		border-color: var(--border-active);
	}

	select {
		cursor: pointer;
	}

	select option {
		background: var(--bg-primary);
		color: var(--text-primary);
	}

	.btn-submit {
		padding: 1rem;
		background: transparent;
		border: 1px solid var(--border-subtle);
		color: var(--text-primary);
		font-size: 0.875rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.btn-submit:hover:not(:disabled) {
		border-color: var(--border-active);
		background: var(--border-subtle);
	}

	.btn-submit:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.loading, .empty-state {
		text-align: center;
		padding: 3rem;
		color: var(--text-primary);
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
		border: 1px solid var(--border-subtle);
		padding: 1.5rem;
		transition: border-color 0.2s ease;
	}

	.season-card.active {
		border-color: var(--border-active);
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
		color: var(--text-primary);
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.badge {
		font-size: 0.625rem;
		padding: 0.25rem 0.5rem;
		border: 1px solid var(--border-subtle);
		letter-spacing: 0.1em;
		color: var(--text-primary);
	}

	.description {
		font-size: 0.875rem;
		font-weight: 300;
		color: var(--text-primary);
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
		color: var(--text-primary);
		opacity: 0.5;
	}

	.meta-item .value {
		font-size: 0.875rem;
		font-weight: 300;
		color: var(--text-primary);
	}

	.season-actions {
		display: flex;
		justify-content: space-between;
		gap: 1rem;
		flex-wrap: wrap;
		align-items: center;
	}

	.actions-left,
	.actions-right {
		display: flex;
		gap: 1rem;
		flex-wrap: wrap;
	}

	.btn-action {
		padding: 0.5rem 1rem;
		background: transparent;
		border: 1px solid var(--border-subtle);
		color: var(--text-primary);
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.btn-action:hover:not(:disabled) {
		border-color: var(--border-active);
		background: var(--border-subtle);
	}

	.btn-action:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.btn-primary:not(:disabled) {
		border-color: var(--border-active);
	}

	.btn-primary:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.1);
	}

	.btn-danger:hover:not(:disabled) {
		border-color: rgba(255, 100, 100, 0.5);
		background: rgba(255, 100, 100, 0.05);
		color: rgb(255, 150, 150);
	}

	.player-management {
		margin-top: 1.5rem;
		padding-top: 1.5rem;
		border-top: 1px solid var(--border-subtle);
	}

	.loading-players {
		text-align: center;
		padding: 2rem;
		color: var(--text-primary);
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
		color: var(--text-primary);
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
		border: 1px solid var(--border-subtle);
		transition: border-color 0.2s ease;
	}

	.player-item:hover {
		border-color: var(--border-active);
	}

	.player-item.inactive {
		opacity: 0.6;
	}

	.player-item.pending-add {
		border-color: rgba(100, 255, 100, 0.5);
		background: rgba(100, 255, 100, 0.02);
	}

	.player-item.pending-remove {
		border-color: rgba(255, 100, 100, 0.5);
		background: rgba(255, 100, 100, 0.02);
	}

	.player-name {
		font-size: 0.875rem;
		font-weight: 300;
		color: var(--text-primary);
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.player-badge {
		font-size: 0.625rem;
		padding: 0.125rem 0.5rem;
		border: 1px solid var(--border-subtle);
		letter-spacing: 0.1em;
		color: var(--text-primary);
		opacity: 0.6;
	}

	.player-badge.pending {
		border-color: rgba(100, 200, 255, 0.6);
		color: rgb(100, 200, 255);
		opacity: 1;
	}

	.btn-add,
	.btn-remove {
		padding: 0.375rem 0.75rem;
		background: transparent;
		border: 1px solid var(--border-subtle);
		color: var(--text-primary);
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
		color: var(--text-primary);
		opacity: 0.5;
		font-size: 0.875rem;
	}

	.player-selection {
		grid-column: 1 / -1;
	}

	.loading-players-create,
	.no-players {
		text-align: center;
		padding: 2rem;
		opacity: 0.5;
		font-size: 0.875rem;
	}

	.player-select-controls {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}

	.btn-select-all,
	.btn-select-none {
		padding: 0.5rem 1rem;
		background: transparent;
		border: 1px solid var(--border-subtle);
		color: var(--text-primary);
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.btn-select-all:hover {
		border-color: rgba(100, 255, 100, 0.5);
		background: rgba(100, 255, 100, 0.05);
	}

	.btn-select-none:hover {
		border-color: rgba(255, 100, 100, 0.5);
		background: rgba(255, 100, 100, 0.05);
	}

	.player-selection-list {
		max-height: 300px;
		overflow-y: auto;
		border: 1px solid var(--border-subtle);
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.player-checkbox {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		cursor: pointer;
		padding: 0.5rem;
		border: 1px solid transparent;
		transition: border-color 0.2s ease;
	}

	.player-checkbox:hover {
		border-color: var(--border-subtle);
	}

	.player-checkbox input[type="checkbox"] {
		width: 1.25rem;
		height: 1.25rem;
		cursor: pointer;
		margin: 0;
		padding: 0;
		-webkit-appearance: none;
		appearance: none;
		flex-shrink: 0;
		border: 1px solid var(--border-subtle);
		position: relative;
		transition: all 0.2s ease;
	}

	/* Dark mode: black background */
	:global([data-theme='dark']) .player-checkbox input[type="checkbox"] {
		background: #000;
		border-color: rgba(255, 255, 255, 0.2);
	}

	/* Light mode: white background */
	:global([data-theme='light']) .player-checkbox input[type="checkbox"] {
		background: #fff;
		border-color: rgba(0, 0, 0, 0.15);
	}

	/* Hover states */
	.player-checkbox input[type="checkbox"]:hover {
		border-color: var(--border-active);
	}

	/* Checkmark - dark mode: thin white check */
	:global([data-theme='dark']) .player-checkbox input[type="checkbox"]:checked::after {
		content: '';
		position: absolute;
		left: 0.35rem;
		top: 0.15rem;
		width: 0.35rem;
		height: 0.65rem;
		border: solid #fff;
		border-width: 0 1.5px 1.5px 0;
		transform: rotate(45deg);
	}

	/* Checkmark - light mode: thin black check */
	:global([data-theme='light']) .player-checkbox input[type="checkbox"]:checked::after {
		content: '';
		position: absolute;
		left: 0.35rem;
		top: 0.15rem;
		width: 0.35rem;
		height: 0.65rem;
		border: solid #000;
		border-width: 0 1.5px 1.5px 0;
		transform: rotate(45deg);
	}

	.player-checkbox-label {
		font-size: 0.875rem;
		font-weight: 300;
		flex: 1;
	}

	.player-stats {
		font-size: 0.75rem;
		opacity: 0.6;
		margin-left: 0.5rem;
	}

	.elo-selector-section {
		margin-bottom: 2rem;
		padding-bottom: 1.5rem;
		border-bottom: 1px solid var(--border-subtle);
	}

	.elo-selector-section h3 {
		font-size: 0.875rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		margin: 0 0 1rem 0;
		color: var(--text-primary);
		opacity: 0.8;
	}

	.elo-selector-container {
		display: flex;
		align-items: center;
		gap: 1rem;
		flex-wrap: wrap;
	}

	.elo-selector {
		flex: 1;
		min-width: 250px;
		padding: 0.75rem;
		background: transparent;
		border: 1px solid var(--border-subtle);
		color: var(--text-primary);
		font-family: inherit;
		font-size: 0.875rem;
		font-weight: 300;
		transition: border-color 0.2s ease;
		cursor: pointer;
	}

	.elo-selector:focus {
		outline: none;
		border-color: var(--border-active);
	}

	.elo-selector option {
		background: var(--bg-primary, #fff);
		color: var(--text-primary);
		padding: 0.5rem;
	}

	.elo-changed-indicator {
		font-size: 0.75rem;
		color: rgb(100, 200, 255);
		letter-spacing: 0.05em;
		padding: 0.375rem 0.75rem;
		border: 1px solid rgba(100, 200, 255, 0.6);
		background: rgba(100, 200, 255, 0.05);
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

		.season-actions {
			flex-direction: column;
			gap: 1rem;
			width: 100%;
		}

		.actions-left,
		.actions-right {
			width: 100%;
			flex-direction: column;
		}

		.btn-action {
			width: 100%;
		}

		.form-row {
			grid-template-columns: 1fr;
		}

		.player-item {
			flex-direction: column;
			align-items: flex-start;
			gap: 0.5rem;
		}

		.elo-selector-container {
			flex-direction: column;
			align-items: stretch;
		}

		.elo-selector {
			width: 100%;
		}
	}
</style>
