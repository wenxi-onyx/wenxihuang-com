<script lang="ts">
	interface Props {
		title: string;
		status: 'active' | 'inactive';
		href?: string;
		delay?: string;
	}

	let { title, status, href, delay = '0s' }: Props = $props();
</script>

{#if status === 'active' && href}
	<a {href} class="card active" style="animation-delay: {delay}">
		<h3 class="card-title">{title}</h3>
		<button class="card-cta">ENTER</button>
	</a>
{:else}
	<div class="card inactive" style="animation-delay: {delay}">
		<h3 class="card-title">{title}</h3>
		<p class="card-status">coming soon</p>
	</div>
{/if}

<style>
	.card {
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		align-items: center;
		width: 100%;
		height: 420px;
		padding: 3rem 2rem;
		text-decoration: none;
		transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
		background: var(--bg-primary);
		opacity: 0;
		animation-name: fadeInCard;
		animation-duration: 0.4s;
		animation-timing-function: ease-out;
		animation-fill-mode: forwards;
	}

	/* Theme-specific borders */
	:global([data-theme='dark']) .card {
		background: #000000;
		color: #ffffff;
		border: 1px solid rgba(255, 255, 255, 0.3);
	}

	:global([data-theme='light']) .card {
		background: #ffffff;
		color: #000000;
		border: 1px solid rgba(0, 0, 0, 0.15);
	}

	:global([data-theme='dark']) .card.active {
		border-width: 2px;
		border-color: rgba(255, 255, 255, 0.35);
	}

	:global([data-theme='light']) .card.active {
		border-width: 1px;
		border-color: rgba(0, 0, 0, 0.2);
	}

	.card.active:hover {
		transform: translateY(-4px);
	}

	:global([data-theme='dark']) .card.active:hover {
		border-color: rgba(255, 255, 255, 0.4);
		box-shadow: 0 8px 24px rgba(255, 255, 255, 0.1);
	}

	:global([data-theme='light']) .card.active:hover {
		border-color: rgba(0, 0, 0, 0.2);
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.04);
	}

	.card.inactive {
		cursor: not-allowed;
	}

	.card.inactive:hover {
		border-color: var(--border-subtle);
	}

	.card-title {
		font-size: 1.5rem;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		text-align: center;
		margin: 0;
	}

	:global([data-theme='dark']) .card-title {
		font-weight: 400;
	}

	:global([data-theme='light']) .card-title {
		font-weight: 200;
	}

	.card-cta {
		padding: 0.75rem 2rem;
		font-size: 0.875rem;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		background: transparent;
		cursor: pointer;
		transition: all 0.3s ease;
		border: none;
	}

	:global([data-theme='dark']) .card-cta {
		color: #ffffff;
		border: 1px solid #ffffff;
	}

	:global([data-theme='dark']) .card-cta:hover {
		background: #ffffff;
		color: #000000;
	}

	:global([data-theme='light']) .card-cta {
		color: #000000;
		border: 0.5px solid #000000;
		font-weight: 200;
	}

	:global([data-theme='light']) .card-cta:hover {
		background: #000000;
		color: #ffffff;
	}

	.card-status {
		font-size: 0.75rem;
		font-family: 'Courier', monospace;
		text-transform: lowercase;
		text-align: center;
		margin: 0;
	}

	:global([data-theme='dark']) .card-status {
		color: #666666;
	}

	:global([data-theme='light']) .card-status {
		color: #999999;
	}

	@keyframes fadeInCard {
		from {
			opacity: 0;
			transform: translateY(20px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	@keyframes fadeInCardInactive {
		from {
			opacity: 0;
			transform: translateY(20px);
		}
		to {
			opacity: 0.6;
			transform: translateY(0);
		}
	}

	.card.inactive {
		animation-name: fadeInCardInactive;
	}

	@media (max-width: 768px) {
		.card {
			width: 100%;
			max-width: 400px;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.card {
			animation: none;
			opacity: 1;
		}

		.card.active:hover {
			transform: none;
		}
	}
</style>
