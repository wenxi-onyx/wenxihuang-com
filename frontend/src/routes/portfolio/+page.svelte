<script lang="ts">
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import LoginButton from '$lib/components/LoginButton.svelte';
</script>

<ThemeToggle />
<LoginButton />

<main class="portfolio-page">
	<section class="hero">
		<div class="hero-content">
			<h1 class="name">WENXI HUANG</h1>
			<nav class="nav-links">
				<a href="/">BACK</a>
			</nav>
		</div>
	</section>

	<section class="content">
		<p class="coming-soon">Coming Soon</p>
	</section>
</main>

<style>
	.portfolio-page {
		min-height: 100vh;
		width: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: flex-start;
		padding: 4rem 2rem 2rem;
		gap: 15.5rem;
	}

	.hero {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: flex-start;
		padding: 0;
	}

	.hero-content {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1rem;
	}

	.name {
		font-size: clamp(2.5rem, 6vw, 5rem);
		font-weight: 300;
		letter-spacing: 0.15em;
		text-align: center;
		margin: 0;
	}

	:global([data-theme='dark']) .name {
		font-family: -apple-system, BlinkMacSystemFont, 'Helvetica Neue', Arial, sans-serif;
		font-weight: 700;
	}

	:global([data-theme='light']) .name {
		font-family: 'Noto Sans JP', sans-serif;
		font-weight: 100;
		letter-spacing: 0.2em;
	}

	.nav-links {
		display: flex;
		gap: 2rem;
	}

	.nav-links a {
		font-size: 0.75rem;
		font-weight: 300;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		text-decoration: underline;
		text-decoration-thickness: 0.5px;
		color: var(--text-primary);
		transition: opacity 0.3s ease;
	}

	:global([data-theme='light']) .nav-links a {
		font-weight: 200;
		text-decoration-thickness: 0.5px;
	}

	.nav-links a:hover {
		opacity: 0.6;
	}

	.content {
		display: flex;
		align-items: center;
		justify-content: center;
		margin-top: 4rem;
	}

	.coming-soon {
		font-size: 1rem;
		font-weight: 300;
		color: var(--text-secondary);
		text-align: center;
		margin: 0;
	}

	:global([data-theme='light']) .coming-soon {
		font-weight: 200;
	}

	@media (prefers-reduced-motion: reduce) {
		.name,
		.nav-links {
			animation: none;
			opacity: 1;
		}
	}
</style>
