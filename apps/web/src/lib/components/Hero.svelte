<script lang="ts">
	interface Props {
		animate?: boolean;
	}

	let { animate = false }: Props = $props();
</script>

<section class="hero">
	<div class="hero-content">
		<h1 class="name" class:animate>WENXI HUANG</h1>
		<nav class="social-links" class:animate>
			<a href="https://github.com/wenxi-onyx" target="_blank" rel="noopener noreferrer">GITHUB</a>
			<a href="https://linkedin.com/in/wenxihuang" target="_blank" rel="noopener noreferrer"
				>LINKEDIN</a
			>
			<a href="/portfolio">PORTFOLIO</a>
			<a href="mailto:mail@wenxihuang.com">CONTACT</a>
		</nav>
	</div>
</section>

<style>
	/* Using shared styles: layout.css (.social-links), animations.css (fadeIn) */

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
		opacity: 1;
	}

	.name.animate {
		opacity: 0;
		animation: fadeIn 2s ease-out forwards;
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

	.social-links {
		opacity: 1;
	}

	.social-links.animate {
		opacity: 0;
		animation: fadeIn 1s ease-out 0.5s forwards;
	}


	@media (prefers-reduced-motion: reduce) {
		.name,
		.social-links {
			animation: none;
			opacity: 1;
		}
	}
</style>
