<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { adminApi, type EloConfiguration, type Job } from '$lib/api/client';

	let configs: EloConfiguration[] = [];
	let loading = true;
	let error = '';
	let showCreateForm = false;
	let editingConfig: EloConfiguration | null = null;
	let jobStatus: Job | null = null;
	let jobInterval: number | null = null;

	// Form state
	let formData = {
		version_name: '',
		k_factor: 32,
		starting_elo: 1000,
		base_k_factor: null as number | null,
		new_player_k_bonus: null as number | null,
		new_player_bonus_period: null as number | null,
		description: ''
	};

	let useDynamicK = false;

	onMount(async () => {
		await loadConfigs();
	});

	onDestroy(() => {
		if (jobInterval) {
			clearInterval(jobInterval);
		}
	});

	async function loadConfigs() {
		try {
			loading = true;
			configs = await adminApi.listEloConfigurations();
			loading = false;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load configurations';
			loading = false;
		}
	}

	function resetForm() {
		formData = {
			version_name: '',
			k_factor: 32,
			starting_elo: 1000,
			base_k_factor: null,
			new_player_k_bonus: null,
			new_player_bonus_period: null,
			description: ''
		};
		useDynamicK = false;
		editingConfig = null;
		showCreateForm = false;
	}

	function startEdit(config: EloConfiguration) {
		editingConfig = config;
		formData = {
			version_name: config.version_name,
			k_factor: config.k_factor,
			starting_elo: config.starting_elo,
			base_k_factor: config.base_k_factor,
			new_player_k_bonus: config.new_player_k_bonus,
			new_player_bonus_period: config.new_player_bonus_period,
			description: config.description || ''
		};
		useDynamicK = config.base_k_factor !== null;
		showCreateForm = true;
	}

	async function handleSubmit() {
		try {
			const payload = {
				...formData,
				base_k_factor: useDynamicK ? (formData.base_k_factor ?? undefined) : undefined,
				new_player_k_bonus: useDynamicK ? (formData.new_player_k_bonus ?? undefined) : undefined,
				new_player_bonus_period: useDynamicK ? (formData.new_player_bonus_period ?? undefined) : undefined
			};

			if (editingConfig) {
				await adminApi.updateEloConfiguration(editingConfig.version_name, payload);
			} else {
				await adminApi.createEloConfiguration(payload);
			}

			await loadConfigs();
			resetForm();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to save configuration';
		}
	}

	async function handleDelete(versionName: string) {
		if (!confirm(`Are you sure you want to delete configuration "${versionName}"?`)) {
			return;
		}

		try {
			await adminApi.deleteEloConfiguration(versionName);
			await loadConfigs();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to delete configuration';
		}
	}

	async function handleActivate(versionName: string) {
		if (!confirm(`Activate configuration "${versionName}"? This will deactivate all other configurations.`)) {
			return;
		}

		try {
			await adminApi.activateEloConfiguration(versionName);
			await loadConfigs();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to activate configuration';
		}
	}

	async function handleRecalculate(versionName: string) {
		if (!confirm(`Recalculate all ELO ratings using "${versionName}"? This may take a while.`)) {
			return;
		}

		try {
			const response = await adminApi.recalculateElo(versionName);
			jobStatus = await adminApi.getJobStatus(response.job_id);

			// Poll job status
			if (jobInterval) clearInterval(jobInterval);
			jobInterval = window.setInterval(async () => {
				try {
					jobStatus = await adminApi.getJobStatus(response.job_id);
					if (jobStatus.status === 'completed' || jobStatus.status === 'failed') {
						if (jobInterval) clearInterval(jobInterval);
						await loadConfigs();
					}
				} catch (e) {
					console.error('Failed to fetch job status:', e);
				}
			}, 2000);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to start recalculation';
		}
	}

	$: if (useDynamicK && formData.base_k_factor === null) {
		formData.base_k_factor = formData.k_factor;
		formData.new_player_k_bonus = 48;
		formData.new_player_bonus_period = 10;
	}
</script>

<svelte:head>
	<title>ELO Algorithm Configurator</title>
</svelte:head>

<div class="container">
	<div class="header-section">
		<a href="/table-tennis" class="btn-back">← Back to Leaderboard</a>
		<h1>ELO Algorithm Configurator</h1>
		<p class="subtitle">Manage ELO rating system configurations and recalculations</p>
	</div>

	{#if error}
		<div class="alert alert-error">
			{error}
			<button class="btn-close" on:click={() => error = ''}>×</button>
		</div>
	{/if}

	{#if jobStatus}
		<div class="job-status" class:completed={jobStatus.status === 'completed'} class:failed={jobStatus.status === 'failed'}>
			<div class="job-header">
				<h3>Recalculation {jobStatus.status === 'completed' ? 'Completed' : jobStatus.status === 'failed' ? 'Failed' : 'In Progress'}</h3>
				<button class="btn-close" on:click={() => jobStatus = null}>×</button>
			</div>
			{#if jobStatus.status === 'running' || jobStatus.status === 'pending'}
				<div class="progress-bar">
					<div class="progress-fill" style="width: {jobStatus.progress}%"></div>
				</div>
				<p class="progress-text">{jobStatus.processed_items} / {jobStatus.total_items || 0} games processed</p>
			{:else if jobStatus.status === 'completed'}
				<p class="success">All ELO ratings have been successfully recalculated!</p>
			{:else}
				<p class="error-text">Recalculation failed. Please check the logs.</p>
			{/if}
		</div>
	{/if}

	<div class="action-bar">
		{#if !showCreateForm}
			<button class="btn btn-primary" on:click={() => showCreateForm = true}>
				+ New Configuration
			</button>
		{/if}
	</div>

	{#if showCreateForm}
		<div class="form-card">
			<h2>{editingConfig ? 'Edit' : 'Create'} Configuration</h2>
			<form on:submit|preventDefault={handleSubmit}>
				<div class="form-group">
					<label for="version_name">Version Name</label>
					<input
						type="text"
						id="version_name"
						bind:value={formData.version_name}
						disabled={!!editingConfig}
						required
						placeholder="e.g., v1, v2, experimental"
					/>
				</div>

				<div class="form-row">
					<div class="form-group">
						<label for="k_factor">K-Factor</label>
						<input
							type="number"
							id="k_factor"
							bind:value={formData.k_factor}
							min="1"
							max="200"
							step="0.1"
							required
						/>
						<p class="help-text">Rating volatility (higher = more volatile)</p>
					</div>

					<div class="form-group">
						<label for="starting_elo">Starting ELO</label>
						<input
							type="number"
							id="starting_elo"
							bind:value={formData.starting_elo}
							min="0"
							step="1"
							required
						/>
						<p class="help-text">Initial rating for new players</p>
					</div>
				</div>

				<div class="form-group">
					<label class="checkbox-label">
						<input type="checkbox" bind:checked={useDynamicK} />
						<span>Use Dynamic K-Factor</span>
					</label>
					<p class="help-text">Adjust K-factor based on player experience</p>
				</div>

				{#if useDynamicK}
					<div class="dynamic-k-section">
						<h3>Dynamic K-Factor Settings</h3>
						<p class="section-description">
							Formula: K = Base K + (Bonus × e^(-games / period))
						</p>

						<div class="form-row">
							<div class="form-group">
								<label for="base_k">Base K-Factor</label>
								<input
									type="number"
									id="base_k"
									bind:value={formData.base_k_factor}
									min="1"
									max="200"
									step="0.1"
									required
								/>
							</div>

							<div class="form-group">
								<label for="k_bonus">New Player K Bonus</label>
								<input
									type="number"
									id="k_bonus"
									bind:value={formData.new_player_k_bonus}
									min="0"
									max="200"
									step="0.1"
									required
								/>
							</div>

							<div class="form-group">
								<label for="bonus_period">Bonus Period (games)</label>
								<input
									type="number"
									id="bonus_period"
									bind:value={formData.new_player_bonus_period}
									min="1"
									step="1"
									required
								/>
							</div>
						</div>
					</div>
				{/if}

				<div class="form-group">
					<label for="description">Description (optional)</label>
					<textarea
						id="description"
						bind:value={formData.description}
						rows="3"
						placeholder="Describe this configuration..."
					></textarea>
				</div>

				<div class="form-actions">
					<button type="submit" class="btn btn-primary">
						{editingConfig ? 'Update' : 'Create'} Configuration
					</button>
					<button type="button" class="btn btn-secondary" on:click={resetForm}>
						Cancel
					</button>
				</div>
			</form>
		</div>
	{/if}

	{#if loading}
		<div class="loading">Loading configurations...</div>
	{:else if configs.length === 0}
		<div class="empty-state">
			<p>No configurations yet. Create your first one!</p>
		</div>
	{:else}
		<div class="configs-grid">
			{#each configs as config}
				<div class="config-card" class:active={config.is_active}>
					{#if config.is_active}
						<div class="active-badge">Active</div>
					{/if}

					<h3 class="config-title">{config.version_name}</h3>

					<div class="config-details">
						<div class="detail-row">
							<span class="label">K-Factor:</span>
							<span class="value">{config.k_factor}</span>
						</div>
						<div class="detail-row">
							<span class="label">Starting ELO:</span>
							<span class="value">{config.starting_elo}</span>
						</div>

						{#if config.base_k_factor !== null}
							<div class="detail-section">
								<h4>Dynamic K-Factor</h4>
								<div class="detail-row">
									<span class="label">Base K:</span>
									<span class="value">{config.base_k_factor}</span>
								</div>
								<div class="detail-row">
									<span class="label">Bonus:</span>
									<span class="value">{config.new_player_k_bonus}</span>
								</div>
								<div class="detail-row">
									<span class="label">Period:</span>
									<span class="value">{config.new_player_bonus_period} games</span>
								</div>
							</div>
						{/if}

						{#if config.description}
							<p class="config-description">{config.description}</p>
						{/if}

						<p class="config-date">
							Created: {new Date(config.created_at).toLocaleDateString()}
						</p>
					</div>

					<div class="config-actions">
						{#if !config.is_active}
							<button class="btn btn-sm btn-success" on:click={() => handleActivate(config.version_name)}>
								Activate
							</button>
						{/if}
						<button class="btn btn-sm btn-primary" on:click={() => handleRecalculate(config.version_name)}>
							Recalculate
						</button>
						<button class="btn btn-sm btn-secondary" on:click={() => startEdit(config)}>
							Edit
						</button>
						{#if !config.is_active}
							<button class="btn btn-sm btn-danger" on:click={() => handleDelete(config.version_name)}>
								Delete
							</button>
						{/if}
					</div>
				</div>
			{/each}
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

	.job-status {
		background: #dbeafe;
		border: 1px solid #93c5fd;
		padding: 1.5rem;
		border-radius: 12px;
		margin-bottom: 1.5rem;
	}

	.job-status.completed {
		background: #d1fae5;
		border-color: #6ee7b7;
	}

	.job-status.failed {
		background: #fef2f2;
		border-color: #fecaca;
	}

	.job-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.job-header h3 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 600;
	}

	.progress-bar {
		width: 100%;
		height: 8px;
		background: rgba(255, 255, 255, 0.5);
		border-radius: 4px;
		overflow: hidden;
		margin-bottom: 0.5rem;
	}

	.progress-fill {
		height: 100%;
		background: #3b82f6;
		transition: width 0.3s ease;
	}

	.progress-text {
		font-size: 0.9rem;
		margin: 0.5rem 0 0;
	}

	.success {
		color: #16a34a;
		font-weight: 600;
		margin: 0;
	}

	.error-text {
		color: #dc2626;
		font-weight: 600;
		margin: 0;
	}

	.action-bar {
		display: flex;
		justify-content: flex-end;
		margin-bottom: 2rem;
	}

	.btn {
		padding: 0.75rem 1.5rem;
		font-size: 1rem;
		font-weight: 500;
		border-radius: 8px;
		border: none;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-primary {
		background: var(--accent-color, #3b82f6);
		color: white;
	}

	.btn-primary:hover {
		background: #2563eb;
	}

	.btn-secondary {
		background: var(--bg-secondary, #f9fafb);
		color: var(--text-primary, #1a1a1a);
		border: 1px solid var(--border-color, #e5e7eb);
	}

	.btn-secondary:hover {
		background: #f3f4f6;
	}

	.btn-success {
		background: #16a34a;
		color: white;
	}

	.btn-success:hover {
		background: #15803d;
	}

	.btn-danger {
		background: #dc2626;
		color: white;
	}

	.btn-danger:hover {
		background: #b91c1c;
	}

	.btn-sm {
		padding: 0.5rem 1rem;
		font-size: 0.875rem;
	}

	.form-card {
		background: var(--bg-primary, white);
		border: 1px solid var(--border-color, #e5e7eb);
		border-radius: 12px;
		padding: 2rem;
		margin-bottom: 2rem;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}

	.form-card h2 {
		margin: 0 0 1.5rem;
		font-size: 1.5rem;
		font-weight: 600;
	}

	.form-group {
		margin-bottom: 1.5rem;
	}

	.form-row {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1.5rem;
	}

	label {
		display: block;
		margin-bottom: 0.5rem;
		font-weight: 500;
		color: var(--text-primary, #1a1a1a);
	}

	input[type="text"],
	input[type="number"],
	textarea {
		width: 100%;
		padding: 0.75rem;
		font-size: 1rem;
		border: 1px solid var(--border-color, #e5e7eb);
		border-radius: 6px;
		background: var(--bg-primary, white);
		color: var(--text-primary, #1a1a1a);
	}

	input:focus,
	textarea:focus {
		outline: none;
		border-color: var(--accent-color, #3b82f6);
	}

	input:disabled {
		background: var(--bg-secondary, #f9fafb);
		cursor: not-allowed;
	}

	.help-text {
		margin: 0.25rem 0 0;
		font-size: 0.875rem;
		color: var(--text-secondary, #666);
	}

	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
	}

	.checkbox-label input[type="checkbox"] {
		width: auto;
	}

	.dynamic-k-section {
		background: var(--bg-secondary, #f9fafb);
		padding: 1.5rem;
		border-radius: 8px;
		margin-bottom: 1.5rem;
	}

	.dynamic-k-section h3 {
		margin: 0 0 0.5rem;
		font-size: 1.25rem;
	}

	.section-description {
		margin: 0 0 1rem;
		font-size: 0.95rem;
		color: var(--text-secondary, #666);
		font-family: monospace;
	}

	.form-actions {
		display: flex;
		gap: 1rem;
		justify-content: flex-end;
		margin-top: 2rem;
	}

	.loading,
	.empty-state {
		text-align: center;
		padding: 3rem;
		font-size: 1.2rem;
		color: var(--text-secondary, #666);
	}

	.configs-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
		gap: 1.5rem;
	}

	.config-card {
		background: var(--bg-primary, white);
		border: 2px solid var(--border-color, #e5e7eb);
		border-radius: 12px;
		padding: 1.5rem;
		position: relative;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
		transition: all 0.2s;
	}

	.config-card:hover {
		box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
	}

	.config-card.active {
		border-color: #16a34a;
		background: linear-gradient(to bottom, #f0fdf4, white);
	}

	.active-badge {
		position: absolute;
		top: 1rem;
		right: 1rem;
		background: #16a34a;
		color: white;
		padding: 0.25rem 0.75rem;
		border-radius: 6px;
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
	}

	.config-title {
		font-size: 1.5rem;
		font-weight: 700;
		margin: 0 0 1rem;
		color: var(--text-primary, #1a1a1a);
	}

	.config-details {
		margin-bottom: 1.5rem;
	}

	.detail-row {
		display: flex;
		justify-content: space-between;
		padding: 0.5rem 0;
		border-bottom: 1px solid var(--border-color, #f3f4f6);
	}

	.detail-row:last-child {
		border-bottom: none;
	}

	.label {
		font-weight: 500;
		color: var(--text-secondary, #666);
	}

	.value {
		font-weight: 600;
		color: var(--text-primary, #1a1a1a);
	}

	.detail-section {
		margin-top: 1rem;
		padding-top: 1rem;
		border-top: 2px solid var(--border-color, #e5e7eb);
	}

	.detail-section h4 {
		font-size: 0.95rem;
		font-weight: 600;
		margin: 0 0 0.75rem;
		color: var(--accent-color, #3b82f6);
	}

	.config-description {
		margin: 1rem 0 0;
		padding: 0.75rem;
		background: var(--bg-secondary, #f9fafb);
		border-radius: 6px;
		font-size: 0.9rem;
		color: var(--text-secondary, #666);
	}

	.config-date {
		margin: 1rem 0 0;
		font-size: 0.85rem;
		color: var(--text-secondary, #999);
	}

	.config-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	@media (max-width: 768px) {
		.container {
			padding: 1rem;
		}

		h1 {
			font-size: 2rem;
		}

		.configs-grid {
			grid-template-columns: 1fr;
		}

		.form-row {
			grid-template-columns: 1fr;
		}
	}
</style>
