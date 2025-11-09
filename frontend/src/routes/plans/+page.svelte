<script lang="ts">
	import { onMount } from 'svelte';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import LoginButton from '$lib/components/LoginButton.svelte';
	import UploadPlanModal, { openUploadPlanModal } from '$lib/components/UploadPlanModal.svelte';
	import Presence from '$lib/components/Presence.svelte';

	const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:8083';

	interface Plan {
		id: string;
		title: string;
		owner_id: string;
		owner_username: string;
		current_version: number;
		comment_count: number;
		is_public: boolean;
		created_at: string;
		updated_at: string;
	}

	let plans: Plan[] = $state([]);
	let loading = $state(true);
	let error = $state('');

	async function loadPlans() {
		try {
			loading = true;
			const response = await fetch(`${API_BASE}/api/plans`);
			if (response.ok) {
				plans = await response.json();
			} else {
				error = 'Failed to load plans';
			}
		} catch (err) {
			error = 'An error occurred while loading plans';
			console.error(err);
		} finally {
			loading = false;
		}
	}

	onMount(async () => {
		await loadPlans();
	});

	function handleUploadPlan() {
		openUploadPlanModal(async () => {
			await loadPlans();
		});
	}

	function formatDate(dateString: string): string {
		return new Date(dateString).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric'
		});
	}
</script>

<svelte:head>
	<title>Plans</title>
</svelte:head>

<ThemeToggle />
<LoginButton />
<UploadPlanModal />
<Presence />

<div class="container">
	<header class="page-header">
		<h1>Plans</h1>
		<nav class="nav-links">
			<button class="nav-link upload-btn" onclick={handleUploadPlan}>+ UPLOAD PLAN</button>
			<a href="/">BACK</a>
		</nav>
	</header>

	{#if loading}
		<div class="loading">Loading plans...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else if plans.length === 0}
		<div class="empty-state">
			<p class="empty-message">No plans uploaded yet</p>
			<button onclick={handleUploadPlan} class="btn">Upload your first plan</button>
		</div>
	{:else}
		<div class="plans-grid">
			{#each plans as plan}
				<a href="/plans/{plan.id}" class="plan-card">
					<div class="plan-header">
						<h2 class="plan-title">{plan.title}</h2>
						<p class="plan-author">by {plan.owner_username}</p>
					</div>
					<div class="plan-meta">
						<div class="plan-stats">
							<span class="stat">v{plan.current_version}</span>
							<span class="stat">{plan.comment_count} {plan.comment_count === 1 ? 'comment' : 'comments'}</span>
						</div>
						<span class="plan-date">{formatDate(plan.updated_at)}</span>
					</div>
				</a>
			{/each}
		</div>
	{/if}
</div>

<style>
	/* Using shared styles: layout.css (.container, .page-header, .nav-links), utilities.css (.loading, .error, .empty-state), buttons.css (.btn) */

	.upload-btn {
		background: none;
		border: none;
		padding: 0;
		cursor: pointer;
		font-family: inherit;
	}

	.empty-message {
		font-size: 1rem;
		color: var(--text-primary);
		opacity: 0.6;
		margin-bottom: 2rem;
	}

	.plans-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
		gap: 2rem;
	}

	.plan-card {
		display: block;
		padding: 2rem;
		border: 1px solid var(--border-subtle);
		text-decoration: none;
		color: inherit;
		transition: opacity 0.2s ease, border-color 0.2s ease;
	}

	.plan-card:hover {
		opacity: 0.8;
		border-color: var(--border-active);
	}

	.plan-header {
		margin-bottom: 1.5rem;
	}

	.plan-title {
		font-size: 1.25rem;
		font-weight: 300;
		letter-spacing: 0.05em;
		margin: 0 0 0.5rem 0;
		color: var(--text-primary);
	}

	:global([data-theme='dark']) .plan-title {
		font-weight: 400;
	}

	.plan-author {
		font-size: 0.875rem;
		font-weight: 300;
		color: var(--text-secondary);
		margin: 0;
	}

	.plan-meta {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding-top: 1rem;
		border-top: 1px solid var(--border-subtle);
	}

	.plan-stats {
		display: flex;
		gap: 1.5rem;
	}

	.stat {
		font-size: 0.75rem;
		font-weight: 300;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--text-secondary);
	}

	.plan-date {
		font-size: 0.75rem;
		font-weight: 300;
		color: var(--text-tertiary);
	}

	@media (max-width: 768px) {

		.plans-grid {
			grid-template-columns: 1fr;
			gap: 1.5rem;
		}
	}
</style>
