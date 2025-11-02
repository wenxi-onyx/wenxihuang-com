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
			<FeatureCard title="WIP" status="inactive" delay="0.7s" />
			<FeatureCard title="PING PONG" status="active" href="/ping-pong" delay="0.5s" />
			<FeatureCard title="MULTIPLAYER CHATGPT" status="inactive" delay="0.7s" />
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

	@media (max-width: 768px) {
		.landing-page {
			min-height: auto;
			padding: 1rem;
			gap: 2rem;
		}

		.cards-container {
			grid-template-columns: 1fr;
			gap: 2rem;
			width: 100%;
			max-width: 400px;
		}
	}
</style>
