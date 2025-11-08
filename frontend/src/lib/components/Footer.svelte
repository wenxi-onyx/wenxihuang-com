<footer class="footer">
	<p class="disclaimer">100% of this Website was AI Generated</p>
</footer>

<style>
	.footer {
		width: 100%;
		padding: 2rem 0.5rem 3rem;
		display: flex;
		justify-content: center;
		align-items: center;
		background: var(--bg-primary);
		transition: background-color 0.3s ease;
	}

	@media (max-width: 768px) {
		.footer {
			padding: 2rem 0.5rem 1.5rem;
		}
	}

	.disclaimer {
		font-size: 0.75rem;
		font-weight: 300;
		font-style: italic;
		color: var(--text-secondary);
		text-align: center;
		margin: 0;
		transition: color 0.3s ease;
	}

	:global([data-theme='light']) .disclaimer {
		font-weight: 200;
	}
</style>
