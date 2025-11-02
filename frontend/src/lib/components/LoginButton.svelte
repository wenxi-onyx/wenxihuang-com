<a href="/login" class="login-btn"> LOGIN </a>

<style>
	.login-btn {
		position: fixed;
		top: 2.5rem;
		right: 2rem;
		padding: 0.5rem 1.5rem;
		font-size: 0.875rem;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		background: transparent;
		text-decoration: none;
		cursor: pointer;
		transition: all 0.3s ease;
		z-index: 100;
		color: var(--text-primary);
	}

	/* Dark mode */
	:global([data-theme='dark']) .login-btn {
		color: #ffffff;
		border: 1px solid #ffffff;
	}

	:global([data-theme='dark']) .login-btn:hover {
		background: #ffffff;
		color: #000000;
	}

	/* Light mode */
	:global([data-theme='light']) .login-btn {
		color: #000000;
		border: 0.5px solid #000000;
		font-weight: 200;
	}

	:global([data-theme='light']) .login-btn:hover {
		background: #000000;
		color: #ffffff;
	}

	@media (max-width: 768px) {
		.login-btn {
			top: 1rem;
			right: 1rem;
			padding: 0.375rem 1rem;
			font-size: 0.75rem;
		}
	}
</style>
