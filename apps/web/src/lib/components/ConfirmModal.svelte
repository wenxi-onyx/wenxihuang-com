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

	// Lock body scroll when modal is open
	$effect(() => {
		if (modal) {
			document.body.style.overflow = 'hidden';
		} else {
			document.body.style.overflow = '';
		}
	});

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
	/* Using shared styles: modals.css (.modal-backdrop, .modal, .modal-header, .modal-body, .modal-actions), buttons.css (.btn, .btn-cancel), animations.css (fadeIn, slideUp) */

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
</style>
