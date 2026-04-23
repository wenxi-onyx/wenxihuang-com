<script lang="ts">
	import { marked } from 'marked';

	interface Props {
		content: string;
	}

	let { content = '' }: Props = $props();

	// Configure marked options
	marked.setOptions({
		breaks: true, // Convert line breaks to <br>
		gfm: true // GitHub Flavored Markdown
	});

	// Render markdown to HTML
	const renderedHtml = $derived(marked.parse(content) as string);
</script>

<div class="markdown-preview">
	{@html renderedHtml}
</div>

<style>
	.markdown-preview {
		width: 100%;
		border: 1px solid var(--border-subtle);
		padding: 0.75rem 1rem;
		min-height: 400px;
		overflow-y: auto;
		font-size: 0.875rem;
		font-family: 'Monaco', 'Courier New', monospace;
		color: var(--text-primary);
		line-height: 1.6;
	}

	/* Typography */
	:global(.markdown-preview h1) {
		font-size: 2rem;
		font-weight: 600;
		margin: 1.5rem 0 1rem;
		color: var(--text-primary);
		border-bottom: 1px solid var(--border-subtle);
		padding-bottom: 0.5rem;
	}

	:global(.markdown-preview h2) {
		font-size: 1.5rem;
		font-weight: 600;
		margin: 1.25rem 0 0.75rem;
		color: var(--text-primary);
	}

	:global(.markdown-preview h3) {
		font-size: 1.25rem;
		font-weight: 600;
		margin: 1rem 0 0.5rem;
		color: var(--text-primary);
	}

	:global(.markdown-preview h4),
	:global(.markdown-preview h5),
	:global(.markdown-preview h6) {
		font-size: 1rem;
		font-weight: 600;
		margin: 1rem 0 0.5rem;
		color: var(--text-primary);
	}

	:global(.markdown-preview p) {
		margin: 0.75rem 0;
	}

	:global(.markdown-preview a) {
		color: #0ea5e9;
		text-decoration: underline;
	}

	:global([data-theme='dark'] .markdown-preview a) {
		color: #38bdf8;
	}

	:global(.markdown-preview strong) {
		font-weight: 700;
	}

	:global(.markdown-preview em) {
		font-style: italic;
	}

	/* Lists */
	:global(.markdown-preview ul),
	:global(.markdown-preview ol) {
		margin: 0.75rem 0;
		padding-left: 2rem;
	}

	:global(.markdown-preview li) {
		margin: 0.25rem 0;
	}

	/* Code */
	:global(.markdown-preview code) {
		font-family: 'Monaco', 'Courier New', monospace;
		font-size: 0.875rem;
		background-color: rgba(0, 0, 0, 0.05);
		padding: 0.125rem 0.25rem;
		border-radius: 0.25rem;
		color: #dc2626;
	}

	:global([data-theme='dark'] .markdown-preview code) {
		background-color: rgba(255, 255, 255, 0.05);
		color: #fca5a5;
	}

	:global(.markdown-preview pre) {
		background-color: rgba(0, 0, 0, 0.05);
		padding: 1rem;
		border-radius: 0.25rem;
		overflow-x: auto;
		margin: 1rem 0;
		border: 1px solid var(--border-subtle);
	}

	:global([data-theme='dark'] .markdown-preview pre) {
		background-color: rgba(255, 255, 255, 0.05);
	}

	:global(.markdown-preview pre code) {
		background-color: transparent;
		padding: 0;
		color: var(--text-primary);
	}

	/* Blockquotes */
	:global(.markdown-preview blockquote) {
		border-left: 3px solid var(--border-subtle);
		padding-left: 1rem;
		margin: 1rem 0;
		color: #64748b;
		font-style: italic;
	}

	:global([data-theme='dark'] .markdown-preview blockquote) {
		color: #94a3b8;
	}

	/* Tables */
	:global(.markdown-preview table) {
		border-collapse: collapse;
		width: 100%;
		margin: 1rem 0;
	}

	:global(.markdown-preview th),
	:global(.markdown-preview td) {
		border: 1px solid var(--border-subtle);
		padding: 0.5rem 0.75rem;
		text-align: left;
	}

	:global(.markdown-preview th) {
		background-color: rgba(0, 0, 0, 0.05);
		font-weight: 600;
	}

	:global([data-theme='dark'] .markdown-preview th) {
		background-color: rgba(255, 255, 255, 0.05);
	}

	/* Horizontal rule */
	:global(.markdown-preview hr) {
		border: none;
		border-top: 1px solid var(--border-subtle);
		margin: 1.5rem 0;
	}

	/* Images */
	:global(.markdown-preview img) {
		max-width: 100%;
		height: auto;
		display: block;
		margin: 1rem 0;
	}

	/* Light mode specific */
	:global(:not([data-theme='dark']) .markdown-preview h1),
	:global(:not([data-theme='dark']) .markdown-preview h2),
	:global(:not([data-theme='dark']) .markdown-preview h3) {
		color: #2563eb;
	}

	:global(:not([data-theme='dark']) .markdown-preview) {
		font-weight: 300;
	}

	/* Dark mode specific */
	:global([data-theme='dark'] .markdown-preview h1),
	:global([data-theme='dark'] .markdown-preview h2),
	:global([data-theme='dark'] .markdown-preview h3) {
		color: #60a5fa;
	}

	:global([data-theme='dark'] .markdown-preview) {
		font-weight: 400;
	}
</style>
