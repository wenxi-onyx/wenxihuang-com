<script lang="ts">
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import LoginButton from '$lib/components/LoginButton.svelte';
	import Hero from '$lib/components/Hero.svelte';
	import FeatureCard from '$lib/components/FeatureCard.svelte';
</script>

<ThemeToggle />
<LoginButton />

<main class="landing-page">
	<Hero animate={true} />

	<section class="features">
		<div class="cards-container">
			<div class="card-wrapper card-1">
				<FeatureCard title="WIP" status="inactive" delay="0.5s" />
			</div>
			<div class="card-wrapper card-2">
				<FeatureCard title="PING PONG" status="active" href="/ping-pong" delay="0.3s" />
			</div>
			<div class="card-wrapper card-3">
				<FeatureCard title="MULTIPLAYER CHATGPT" status="inactive" delay="0.7s" />
			</div>
		</div>
	</section>
</main>

<style>
	.landing-page {
		min-height: 100vh;
		width: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: flex-start;
		padding: 4rem 2rem 2rem;
		gap: 3.5rem;
	}

	.features {
		display: flex;
		justify-content: center;
		align-items: center;
	}

	.cards-container {
		display: grid;
		grid-template-columns: repeat(3, 320px);
		gap: 3rem;
	}

	@media (max-width: 1200px) {
		.cards-container {
			grid-template-columns: repeat(3, 240px);
			gap: 2rem;
		}
	}

	/* Desktop: wrappers are transparent, grid handles layout */
	@media (min-width: 769px) {
		.card-wrapper {
			display: contents;
		}
	}

	@media (max-width: 768px) {
		.landing-page {
			min-height: auto;
			padding: 3.5rem 1rem 2rem;
			gap: 2rem;
		}

		.cards-container {
			display: flex;
			flex-direction: column;
			gap: 2rem;
			width: 100%;
			max-width: 400px;
		}

		/* Mobile: wrappers become flex items for reordering */
		.card-wrapper {
			display: block;
			width: 100%;
		}

		/* Reorder cards on mobile: Ping Pong first */
		.card-2 {
			order: 1;
		}

		.card-1 {
			order: 2;
		}

		.card-3 {
			order: 3;
		}
	}
</style>
