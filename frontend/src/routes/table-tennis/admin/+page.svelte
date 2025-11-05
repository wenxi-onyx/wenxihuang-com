<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/stores/auth';
	import { adminApi, type EloConfiguration, type Job } from '$lib/api/client';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import LoginButton from '$lib/components/LoginButton.svelte';

	let configs: EloConfiguration[] = [];
	let loading = true;
	let error = '';
	let showEditModal = false;
	let editingConfig: EloConfiguration | null = null;
	let jobStatus: Job | null = null;
	let jobInterval: number | null = null;
	let shouldRecalculate = false;

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
		const currentUser = await authStore.checkAuth();
		if (!currentUser) {
			error = 'You must be logged in to access this page';
			goto('/login');
			return;
		}
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
		showEditModal = false;
		shouldRecalculate = false;
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
		shouldRecalculate = false;
		showEditModal = true;
	}

	async function handleSubmit(e: Event) {
		e.preventDefault();
		try {
			const payload = {
				...formData,
				base_k_factor: useDynamicK ? (formData.base_k_factor ?? undefined) : undefined,
				new_player_k_bonus: useDynamicK ? (formData.new_player_k_bonus ?? undefined) : undefined,
				new_player_bonus_period: useDynamicK ? (formData.new_player_bonus_period ?? undefined) : undefined
			};

			// Save the configuration
			await adminApi.updateEloConfiguration(editingConfig!.version_name, payload);

			// If shouldRecalculate is true, trigger recalculation
			if (shouldRecalculate) {
				const response = await adminApi.recalculateElo(editingConfig!.version_name);
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


	$: if (useDynamicK && formData.base_k_factor === null) {
		formData.base_k_factor = formData.k_factor;
		formData.new_player_k_bonus = 48;
		formData.new_player_bonus_period = 10;
	}
</script>

<svelte:head>
	<title>ELO Algorithm Configurator</title>
</svelte:head>

<ThemeToggle />
<LoginButton />

<div class="container">
	<header class="page-header">
		<h1>ELO Algorithm Configurator</h1>
		<nav class="nav-links">
			<a href="/table-tennis">BACK</a>
		</nav>
	</header>

	{#if error}
		<div class="alert alert-error">
			{error}
			<button class="btn-close" onclick={() => error = ''}>×</button>
		</div>
	{/if}

	{#if jobStatus}
		<div class="job-status" class:completed={jobStatus.status === 'completed'} class:failed={jobStatus.status === 'failed'}>
			<div class="job-header">
				<h3>Recalculation {jobStatus.status === 'completed' ? 'Completed' : jobStatus.status === 'failed' ? 'Failed' : 'In Progress'}</h3>
				<button class="btn-close" onclick={() => jobStatus = null}>×</button>
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
						<button class="btn btn-sm btn-primary" onclick={() => startEdit(config)}>
							Edit
						</button>
						{#if !config.is_active}
							<button class="btn btn-sm btn-success" onclick={() => handleActivate(config.version_name)}>
								Activate
							</button>
						{/if}
						{#if !config.is_active}
							<button class="btn btn-sm btn-danger" onclick={() => handleDelete(config.version_name)}>
								Delete
							</button>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<!-- Edit Modal -->
{#if showEditModal && editingConfig}
	<div class="modal-backdrop" onclick={resetForm}>
		<div class="modal" onclick={(e) => e.stopPropagation()}>
			<div class="modal-header">
				<h2>Edit Configuration: {editingConfig.version_name}</h2>
				<button class="btn-close" onclick={resetForm}>×</button>
			</div>

			<form onsubmit={handleSubmit}>
				<div class="modal-body">
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

					<div class="form-group">
						<label class="checkbox-label recalculate-checkbox">
							<input type="checkbox" bind:checked={shouldRecalculate} />
							<span>Recalculate all ELO ratings after saving</span>
						</label>
						<p class="help-text">This will recalculate ELO ratings for ALL matches across ALL time using this configuration. Every game will be reprocessed in chronological order. This may take a few moments.</p>
					</div>
				</div>

				<div class="modal-actions">
					<button type="button" class="btn btn-secondary" onclick={resetForm}>
						Cancel
					</button>
					<button type="submit" class="btn btn-primary">
						{shouldRecalculate ? 'Save & Recalculate' : 'Save Changes'}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}

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
		color: inherit;
		opacity: 0.7;
		transition: opacity 0.2s ease;
	}

	.nav-links a:hover {
		opacity: 1;
	}

	.alert {
		padding: 1rem 1.5rem;
		margin-bottom: 2rem;
		display: flex;
		justify-content: space-between;
		align-items: center;
		border: 1px solid var(--border-subtle);
		background: transparent;
	}

	.alert-error {
		border-color: rgba(255, 100, 100, 0.3);
		color: var(--text-primary);
		opacity: 0.9;
	}

	.btn-close {
		background: none;
		border: none;
		font-size: 1.5rem;
		font-weight: 300;
		cursor: pointer;
		padding: 0;
		width: 2rem;
		height: 2rem;
		display: flex;
		align-items: center;
		justify-content: center;
		color: inherit;
		opacity: 0.6;
		transition: opacity 0.2s ease;
	}

	.btn-close:hover {
		opacity: 1;
	}

	.job-status {
		background: transparent;
		border: 1px solid var(--border-subtle);
		padding: 1.5rem;
		margin-bottom: 2rem;
	}

	.job-status.completed {
		border-color: rgba(100, 255, 100, 0.3);
	}

	.job-status.failed {
		border-color: rgba(255, 100, 100, 0.3);
	}

	.job-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.job-header h3 {
		margin: 0;
		font-size: 0.875rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--text-primary);
	}

	.progress-bar {
		width: 100%;
		height: 1px;
		background: var(--border-subtle);
		overflow: hidden;
		margin-bottom: 1rem;
	}

	.progress-fill {
		height: 100%;
		background: var(--text-primary);
		opacity: 0.5;
		transition: width 0.3s ease;
	}

	.progress-text {
		font-size: 0.875rem;
		font-weight: 300;
		color: var(--text-primary);
		opacity: 0.7;
		margin: 0.5rem 0 0;
	}

	.success {
		color: rgba(100, 255, 100, 0.8);
		font-weight: 300;
		margin: 0;
		font-size: 0.875rem;
	}

	.error-text {
		color: rgba(255, 100, 100, 0.8);
		font-weight: 300;
		margin: 0;
		font-size: 0.875rem;
	}

	.action-bar {
		display: flex;
		justify-content: flex-end;
		margin-bottom: 2rem;
	}

	.btn {
		padding: 0.75rem 1.5rem;
		font-size: 0.75rem;
		font-weight: 300;
		font-family: inherit;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		border: 1px solid var(--border-subtle);
		background: transparent;
		color: var(--text-primary);
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.btn:hover {
		border-color: var(--border-active);
		opacity: 0.8;
	}

	.btn-primary {
		border-color: var(--border-subtle);
		background: transparent;
		color: var(--text-primary);
	}

	.btn-primary:hover {
		border-color: var(--border-active);
		opacity: 0.8;
	}

	.btn-secondary {
		border-color: var(--border-subtle);
		background: transparent;
		color: var(--text-primary);
		opacity: 0.6;
	}

	.btn-secondary:hover {
		border-color: var(--border-active);
		opacity: 0.8;
	}

	.btn-success {
		border-color: rgba(100, 255, 100, 0.3);
		background: transparent;
		color: rgba(100, 255, 100, 0.8);
	}

	.btn-success:hover {
		border-color: rgba(100, 255, 100, 0.5);
		opacity: 0.9;
	}

	.btn-danger {
		border-color: rgba(255, 100, 100, 0.3);
		background: transparent;
		color: rgba(255, 100, 100, 0.8);
	}

	.btn-danger:hover {
		border-color: rgba(255, 100, 100, 0.5);
		opacity: 0.9;
	}

	.btn-sm {
		padding: 0.5rem 1rem;
		font-size: 0.75rem;
	}

	.form-card {
		background: transparent;
		border: 1px solid var(--border-subtle);
		padding: 2rem;
		margin-bottom: 2rem;
	}

	.form-card h2 {
		margin: 0 0 2rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid var(--border-subtle);
		font-size: 0.875rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--text-primary);
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
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		color: var(--text-primary);
		opacity: 0.7;
	}

	input[type="text"],
	input[type="number"],
	textarea {
		width: 100%;
		padding: 0.75rem;
		font-size: 0.875rem;
		font-family: inherit;
		font-weight: 300;
		border: 1px solid var(--border-subtle);
		background: transparent;
		color: var(--text-primary);
		transition: border-color 0.2s ease;
	}

	input:focus,
	textarea:focus {
		outline: none;
		border-color: var(--border-active);
	}

	input:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	textarea {
		resize: vertical;
		min-height: 80px;
	}

	.help-text {
		margin: 0.5rem 0 0;
		font-size: 0.75rem;
		font-weight: 300;
		color: var(--text-primary);
		opacity: 0.5;
		font-style: italic;
	}

	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		cursor: pointer;
		font-size: 0.875rem;
		font-weight: 300;
	}

	.checkbox-label input[type="checkbox"] {
		width: auto;
	}

	.dynamic-k-section {
		background: transparent;
		border: 1px solid var(--border-subtle);
		padding: 1.5rem;
		margin-bottom: 1.5rem;
	}

	.dynamic-k-section h3 {
		margin: 0 0 1rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid var(--border-subtle);
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--text-primary);
	}

	.section-description {
		margin: 0 0 1.5rem;
		font-size: 0.75rem;
		font-weight: 300;
		color: var(--text-primary);
		opacity: 0.6;
		font-family: monospace;
	}

	.form-actions {
		display: flex;
		gap: 1rem;
		justify-content: flex-end;
		margin-top: 2rem;
		padding-top: 2rem;
		border-top: 1px solid var(--border-subtle);
	}

	.loading,
	.empty-state {
		text-align: center;
		padding: 3rem;
		font-size: 0.875rem;
		font-weight: 300;
		color: var(--text-primary);
		opacity: 0.7;
	}

	.configs-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
		gap: 1.5rem;
	}

	.config-card {
		background: transparent;
		border: 1px solid var(--border-subtle);
		padding: 1.5rem;
		position: relative;
		transition: border-color 0.2s ease;
	}

	.config-card:hover {
		border-color: var(--border-active);
	}

	.config-card.active {
		border-color: rgba(100, 255, 100, 0.3);
	}

	.active-badge {
		position: absolute;
		top: 1rem;
		right: 1rem;
		border: 1px solid rgba(100, 255, 100, 0.3);
		background: transparent;
		color: rgba(100, 255, 100, 0.8);
		padding: 0.25rem 0.75rem;
		font-size: 0.625rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
	}

	.config-title {
		font-size: 1.125rem;
		font-weight: 300;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		margin: 0 0 1.5rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid var(--border-subtle);
		color: var(--text-primary);
	}

	.config-details {
		margin-bottom: 1.5rem;
	}

	.detail-row {
		display: flex;
		justify-content: space-between;
		padding: 0.75rem 0;
		border-bottom: 1px solid var(--border-subtle);
	}

	.detail-row:last-child {
		border-bottom: none;
	}

	.label {
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		color: var(--text-primary);
		opacity: 0.6;
	}

	.value {
		font-size: 0.875rem;
		font-weight: 300;
		color: var(--text-primary);
	}

	.detail-section {
		margin-top: 1rem;
		padding-top: 1rem;
		border-top: 1px solid var(--border-subtle);
	}

	.detail-section h4 {
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		margin: 0 0 1rem;
		padding-bottom: 0.75rem;
		border-bottom: 1px solid var(--border-subtle);
		color: var(--text-primary);
		opacity: 0.8;
	}

	.config-description {
		margin: 1rem 0 0;
		padding: 1rem;
		border: 1px solid var(--border-subtle);
		background: transparent;
		font-size: 0.75rem;
		font-weight: 300;
		color: var(--text-primary);
		opacity: 0.7;
		font-style: italic;
	}

	.config-date {
		margin: 1rem 0 0;
		font-size: 0.75rem;
		font-weight: 300;
		color: var(--text-primary);
		opacity: 0.5;
	}

	.config-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		padding-top: 1rem;
		border-top: 1px solid var(--border-subtle);
	}

	/* Modal Styles */
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		backdrop-filter: blur(4px);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 10000;
		animation: fadeIn 0.2s ease-out;
	}

	@keyframes fadeIn {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	.modal {
		background: var(--bg-primary);
		border: 1px solid var(--border-subtle);
		max-width: 700px;
		width: calc(100% - 2rem);
		margin: 1rem;
		max-height: 90vh;
		overflow-y: auto;
		animation: slideUp 0.3s ease-out;
	}

	@keyframes slideUp {
		from {
			transform: translateY(20px);
			opacity: 0;
		}
		to {
			transform: translateY(0);
			opacity: 1;
		}
	}

	.modal-header {
		padding: 1.5rem;
		border-bottom: 1px solid var(--border-subtle);
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.modal-header h2 {
		font-size: 0.875rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		margin: 0;
		color: var(--text-primary);
	}

	.modal-body {
		padding: 2rem;
	}

	.modal-actions {
		padding: 1.5rem;
		border-top: 1px solid var(--border-subtle);
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
	}

	.recalculate-checkbox {
		padding: 1rem;
		border: 1px solid var(--border-subtle);
		margin-top: 1rem;
	}

	.recalculate-checkbox span {
		font-weight: 300;
		color: var(--text-primary);
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
			width: 100%;
		}

		.configs-grid {
			grid-template-columns: 1fr;
		}

		.form-row {
			grid-template-columns: 1fr;
		}

		.form-actions,
		.modal-actions {
			flex-direction: column;
		}

		.config-actions {
			flex-direction: column;
		}

		.btn {
			width: 100%;
		}

		.modal {
			max-width: none;
			max-height: 100vh;
			margin: 0;
			width: 100%;
		}
	}
</style>
