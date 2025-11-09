<script lang="ts" module>
	import { writable } from 'svelte/store';

	type UploadPlanModalState = {
		isOpen: boolean;
		onSuccess?: () => void | Promise<void>;
	};

	const modalStore = writable<UploadPlanModalState>({ isOpen: false });

	export function openUploadPlanModal(onSuccess?: () => void | Promise<void>) {
		modalStore.set({ isOpen: true, onSuccess });
	}

	export function closeUploadPlanModal() {
		modalStore.set({ isOpen: false });
	}
</script>

<script lang="ts">
	import { goto } from '$app/navigation';
	import { showToast } from '$lib/components/Toast.svelte';
	import MarkdownEditor from '$lib/components/MarkdownEditor.svelte';

	const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:8083';

	let modalState = $derived($modalStore);

	let title = $state('');
	let content = $state('');
	let isPublic = $state(true);
	let file: File | null = $state(null);
	let loading = $state(false);
	let loadingFile = $state(false);
	let fileInputElement = $state<HTMLInputElement>();
	let contentSource = $state<'manual' | 'file'>('manual');
	let fileName = $state('');
	let fileSize = $state(0);

	const MAX_FILE_SIZE = 1_048_576; // 1MB

	const hasContent = $derived(content.trim().length > 0);

	function handleFileSelect(event: Event) {
		const target = event.target as HTMLInputElement;
		const selectedFile = target.files?.[0];

		if (!selectedFile) return;

		if (selectedFile.size > MAX_FILE_SIZE) {
			showToast('File size must be less than 1MB', 'error');
			return;
		}

		if (!selectedFile.name.endsWith('.md')) {
			showToast('Please upload a markdown (.md) file', 'error');
			return;
		}

		file = selectedFile;
		fileName = selectedFile.name;
		fileSize = selectedFile.size;
		loadingFile = true;

		const reader = new FileReader();

		reader.onload = (e) => {
			const fileContent = e.target?.result as string;

			if (!fileContent || fileContent.trim().length === 0) {
				showToast('File is empty. Please choose a file with content.', 'error');
				loadingFile = false;
				file = null;
				fileName = '';
				fileSize = 0;
				if (fileInputElement) {
					fileInputElement.value = '';
				}
				return;
			}

			content = fileContent;
			contentSource = 'file';
			loadingFile = false;

			if (!title) {
				title = selectedFile.name.replace('.md', '');
			}
		};

		reader.onerror = () => {
			showToast('Failed to read file. Please try again.', 'error');
			loadingFile = false;
			contentSource = 'manual';
			file = null;
			fileName = '';
			fileSize = 0;
			if (fileInputElement) {
				fileInputElement.value = '';
			}
		};

		reader.readAsText(selectedFile);
	}

	function clearFileAndSwitchToManual() {
		file = null;
		content = '';
		contentSource = 'manual';
		fileName = '';
		fileSize = 0;
		if (fileInputElement) {
			fileInputElement.value = '';
		}
	}

	function formatFileSize(bytes: number): string {
		if (bytes < 1024) return bytes + ' B';
		if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
		return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
	}

	function resetForm() {
		title = '';
		content = '';
		isPublic = true;
		file = null;
		fileName = '';
		fileSize = 0;
		contentSource = 'manual';
		if (fileInputElement) {
			fileInputElement.value = '';
		}
	}

	function handleClose() {
		resetForm();
		closeUploadPlanModal();
	}

	async function handleSubmit() {
		if (!title.trim() || !content.trim()) {
			showToast('Please provide both title and content', 'error');
			return;
		}

		loading = true;

		try {
			const response = await fetch(`${API_BASE}/api/plans`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				credentials: 'include',
				body: JSON.stringify({
					title: title.trim(),
					content: content.trim(),
					is_public: isPublic
				})
			});

			if (response.ok) {
				const plan = await response.json();
				showToast('Plan uploaded successfully!', 'success');
				resetForm();
				closeUploadPlanModal();

				// Call onSuccess callback if provided, otherwise navigate
				if (modalState.onSuccess) {
					await modalState.onSuccess();
				}

				goto(`/plans/${plan.id}`);
			} else {
				const data = await response.json();
				showToast(data.error || 'Failed to upload plan', 'error');
			}
		} catch (err) {
			showToast('An error occurred while uploading the plan', 'error');
			console.error(err);
		} finally {
			loading = false;
		}
	}
</script>

{#if modalState.isOpen}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div class="modal-backdrop" onclick={handleClose} role="presentation">
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<div class="modal modal-xl" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-labelledby="modal-title" tabindex="-1">
			<div class="modal-header">
				<h2 id="modal-title">Upload Plan</h2>
				<button onclick={handleClose} class="modal-close">×</button>
			</div>

			<form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }} class="upload-form">
				<div class="form-body">
					<div class="form-section">
						<div class="label-with-spinner">
							<label for="file-upload">Upload Markdown File (Optional)</label>
							{#if loadingFile}
								<span class="file-spinner"></span>
							{/if}
						</div>
						<input
							bind:this={fileInputElement}
							id="file-upload"
							type="file"
							accept=".md"
							onchange={handleFileSelect}
							class="file-input"
							disabled={hasContent}
						/>
						{#if fileName && !loadingFile}
							<p class="file-info">Loaded: {fileName} ({formatFileSize(fileSize)})</p>
						{/if}
						<p class="form-help">
							{#if hasContent}
								Clear the content below to enable file upload
							{:else}
								Upload a file OR paste content directly below (Max size: 1MB, .md files only)
							{/if}
						</p>
					</div>

					<div class="form-section">
						<label for="title">Title *</label>
						<input
							id="title"
							type="text"
							bind:value={title}
							required
							maxlength="500"
							placeholder="Enter plan title"
						/>
					</div>

					<div class="form-section">
						<div class="label-with-action">
							<label for="content">
								Content * {#if contentSource === 'file'}<span class="mode-indicator">(File Preview - Read Only)</span>{/if}
							</label>
							{#if contentSource === 'file' && content}
								<button type="button" onclick={clearFileAndSwitchToManual} class="clear-file-btn">
									Edit Manually
								</button>
							{/if}
						</div>
						<div class="editor-wrapper">
							<MarkdownEditor
								bind:value={content}
								placeholder={contentSource === 'manual' ? "Paste or type your markdown content here..." : ""}
								readonly={contentSource === 'file'}
								minHeight="400px"
							/>
						</div>
						<p class="form-help">{content.length} characters</p>
					</div>

					<div class="form-section checkbox-section">
						<label class="checkbox-label">
							<span class="checkbox" class:checked={isPublic}>
								{#if isPublic}✓{/if}
							</span>
							<input
								id="is-public"
								type="checkbox"
								bind:checked={isPublic}
								class="checkbox-input-hidden"
							/>
							<span>Make this plan public (visible to everyone)</span>
						</label>
					</div>
				</div>

				<div class="form-actions">
					<button type="button" onclick={handleClose} class="btn-cancel">Cancel</button>
					<button
						type="submit"
						disabled={loading || !title.trim() || !content.trim()}
						class="btn-primary"
					>
						{#if loading}
							Uploading...
						{:else}
							Upload Plan
						{/if}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}

<style>
	/* Using shared styles: modals.css (.modal-backdrop, .modal, .modal-lg, .modal-header, .modal-close), forms.css (label, input[type="text"]), buttons.css (.btn-primary, .btn-cancel) */

	/* Wider modal variant for upload form */
	.modal-xl {
		max-width: 1000px;
		display: flex;
		flex-direction: column;
		max-height: 90vh;
	}

	.upload-form {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0; /* Important for flexbox overflow */
	}

	.form-body {
		flex: 1;
		overflow-y: auto;
		padding: 2rem;
		padding-bottom: 1rem;
	}

	.form-section {
		margin-bottom: 2rem;
	}

	.form-section:last-of-type {
		margin-bottom: 0;
	}

	.editor-wrapper {
		max-height: 300px;
		overflow: auto;
	}

	.label-with-spinner {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}

	.label-with-action {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.75rem;
	}

	.file-spinner {
		display: inline-block;
		width: 14px;
		height: 14px;
		border: 2px solid var(--border-subtle);
		border-top-color: var(--text-primary);
		border-radius: 50%;
		animation: spin 0.6s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.file-info {
		margin-top: 0.5rem;
		font-size: 0.75rem;
		color: var(--text-primary);
		opacity: 0.7;
		font-weight: 300;
	}

	.mode-indicator {
		font-size: 0.7rem;
		font-weight: 300;
		text-transform: none;
		opacity: 0.6;
		margin-left: 0.5rem;
	}

	.clear-file-btn {
		padding: 0.5rem 1rem;
		font-size: 0.7rem;
		font-weight: 300;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		color: var(--text-primary);
		background: transparent;
		border: 1px solid var(--border-subtle);
		cursor: pointer;
		transition: opacity 0.2s ease, border-color 0.2s ease;
		font-family: inherit;
		white-space: nowrap;
	}

	.clear-file-btn:hover {
		opacity: 0.7;
		border-color: var(--border-active);
	}

	.file-input {
		width: 100%;
		padding: 0.75rem;
		font-size: 0.875rem;
		font-family: inherit;
		color: var(--text-primary);
		background: transparent;
		border: 1px solid var(--border-subtle);
		outline: none;
		transition: border-color 0.2s ease, opacity 0.2s ease;
	}

	.file-input:focus {
		border-color: var(--border-active);
	}

	.file-input:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.form-help {
		margin-top: 0.5rem;
		font-size: 0.75rem;
		color: var(--text-secondary);
		opacity: 0.6;
	}

	.checkbox-section {
		padding: 1rem 0;
		border-top: 1px solid var(--border-subtle);
	}

	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		font-size: 0.875rem;
		font-weight: 300;
		color: var(--text-primary);
		cursor: pointer;
	}

	.checkbox {
		width: 1.25rem;
		height: 1.25rem;
		border: 1px solid var(--border-subtle);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.875rem;
		transition: all 0.2s ease;
		flex-shrink: 0;
	}

	.checkbox.checked {
		border-color: var(--border-active);
		background: var(--text-primary);
		color: var(--bg-primary);
	}

	.checkbox-input-hidden {
		position: absolute;
		opacity: 0;
		pointer-events: none;
	}

	.form-actions {
		display: flex;
		gap: 1rem;
		padding: 1.5rem 2rem;
		border-top: 1px solid var(--border-subtle);
		background: var(--bg-primary);
		flex-shrink: 0;
	}

	/* Using shared button styles: buttons.css (.btn-primary, .btn-cancel) */

	.form-actions .btn-primary,
	.form-actions .btn-cancel {
		flex: 1;
	}

	@media (max-width: 768px) {
		.modal-xl {
			max-width: 100%;
			max-height: 100vh;
		}

		.form-body {
			padding: 1.5rem;
			padding-bottom: 1rem;
		}

		.editor-wrapper {
			max-height: 200px;
		}

		.label-with-spinner {
			flex-wrap: wrap;
		}

		.label-with-action {
			flex-direction: column;
			align-items: flex-start;
			gap: 0.5rem;
		}

		.clear-file-btn {
			font-size: 0.65rem;
			padding: 0.4rem 0.75rem;
		}

		.file-info {
			font-size: 0.7rem;
		}

		.form-actions {
			flex-direction: column;
			padding: 1rem 1.5rem;
		}
	}
</style>
