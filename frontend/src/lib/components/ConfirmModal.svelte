<script lang="ts" module>
	import { writable, get } from 'svelte/store';

	export type ConfirmOptions = {
		title: string;
		message: string;
		confirmText?: string;
		cancelText?: string;
		confirmStyle?: 'danger' | 'primary' | 'warning';
	};

	type ConfirmState = ConfirmOptions & {
		id: number;
		resolve: (value: boolean) => void;
	};

	let modalId = 0;
	const modalStore = writable<ConfirmState | null>(null);

	export function confirm(options: ConfirmOptions): Promise<boolean> {
		return new Promise((resolve) => {
			// Cancel any pending modal to prevent race conditions
			const current = get(modalStore);
			if (current) {
				current.resolve(false);
			}

			const id = modalId++;
			modalStore.set({
				...options,
				id,
				resolve,
			});
		});
	}

	function handleConfirm(modal: ConfirmState) {
		modal.resolve(true);
		modalStore.set(null);
	}

	function handleCancel(modal: ConfirmState) {
		modal.resolve(false);
		modalStore.set(null);
	}
</script>

<script lang="ts">
	import { onMount } from 'svelte';

	let modal = $derived($modalStore);

	onMount(() => {
		function handleEscape(e: KeyboardEvent) {
			// Fix: Read current store value directly instead of closure capture
			const currentModal = get(modalStore);
			if (e.key === 'Escape' && currentModal) {
				handleCancel(currentModal);
			}
		}

		document.addEventListener('keydown', handleEscape);
		return () => document.removeEventListener('keydown', handleEscape);
	});
</script>

{#if modal}
	<div
		class="modal-backdrop"
		onclick={() => handleCancel(modal)}
		onkeydown={(e) => e.key === 'Enter' && handleCancel(modal)}
		role="button"
		tabindex="0"
		aria-label="Close modal"
	>
		<div
			class="modal"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="dialog"
			tabindex="-1"
			aria-modal="true"
			aria-labelledby="modal-title"
		>
			<div class="modal-header">
				<h2 id="modal-title">{modal.title}</h2>
			</div>

			<div class="modal-body">
				<p>{modal.message}</p>
			</div>

			<div class="modal-actions">
				<button class="btn btn-cancel" onclick={() => handleCancel(modal)}>
					{modal.cancelText || 'CANCEL'}
				</button>
				<button
					class="btn btn-confirm btn-{modal.confirmStyle || 'primary'}"
					onclick={() => handleConfirm(modal)}
				>
					{modal.confirmText || 'CONFIRM'}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
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
		background: var(--bg-primary, #000);
		border: 1px solid var(--border-subtle);
		max-width: 500px;
		width: calc(100% - 2rem);
		margin: 1rem;
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
	}

	.modal-header h2 {
		font-size: 1rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		margin: 0;
		color: var(--text-primary);
	}

	.modal-body {
		padding: 1.5rem;
	}

	.modal-body p {
		font-size: 0.875rem;
		font-weight: 300;
		line-height: 1.6;
		color: var(--text-primary);
		opacity: 0.8;
		margin: 0;
		white-space: pre-line;
	}

	.modal-actions {
		padding: 1.5rem;
		border-top: 1px solid var(--border-subtle);
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
	}

	.btn {
		padding: 0.75rem 1.5rem;
		font-size: 0.75rem;
		font-weight: 300;
		font-family: inherit;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		border: 1px solid var(--border-subtle);
		background: transparent;
		color: var(--text-primary);
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.btn:hover {
		border-color: var(--border-active);
	}

	.btn-cancel:hover {
		opacity: 0.8;
	}

	.btn-confirm.btn-danger:hover {
		border-color: rgba(255, 100, 100, 0.5);
		background: rgba(255, 100, 100, 0.1);
		color: rgb(255, 150, 150);
	}

	.btn-confirm.btn-warning:hover {
		border-color: rgba(255, 200, 100, 0.5);
		background: rgba(255, 200, 100, 0.1);
		color: rgb(255, 220, 150);
	}

	.btn-confirm.btn-primary:hover {
		border-color: rgba(100, 200, 255, 0.5);
		background: rgba(100, 200, 255, 0.1);
		color: rgb(150, 220, 255);
	}

	@media (max-width: 768px) {
		.modal {
			max-width: none;
		}

		.modal-actions {
			flex-direction: column-reverse;
		}

		.btn {
			width: 100%;
		}
	}
</style>
