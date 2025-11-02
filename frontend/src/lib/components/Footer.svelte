<footer class="footer">
	<p class="disclaimer">Everything on this Website was AI Generated</p>
</footer>

<style>
	.footer {
		position: fixed;
		bottom: 0;
		left: 0;
		right: 0;
		width: 100%;
		padding: 0.5rem;
		display: flex;
		justify-content: center;
		align-items: center;
		z-index: 100;
		background: var(--bg-primary);
		transition: background-color 0.3s ease;
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
