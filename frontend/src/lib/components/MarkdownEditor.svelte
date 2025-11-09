<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { EditorView, keymap, lineNumbers, placeholder as placeholderExt } from '@codemirror/view';
	import { EditorState, Compartment } from '@codemirror/state';
	import { markdown } from '@codemirror/lang-markdown';
	import { oneDark } from '@codemirror/theme-one-dark';
	import { defaultKeymap, indentWithTab } from '@codemirror/commands';
	import { syntaxHighlighting, LanguageDescription } from '@codemirror/language';
	import { classHighlighter } from '@lezer/highlight';

	// Language support for code blocks
	import { javascript } from '@codemirror/lang-javascript';
	import { python } from '@codemirror/lang-python';
	import { css } from '@codemirror/lang-css';
	import { html } from '@codemirror/lang-html';
	import { json } from '@codemirror/lang-json';
	import { sql } from '@codemirror/lang-sql';
	import { rust } from '@codemirror/lang-rust';

	interface Props {
		value: string;
		placeholder?: string;
		readonly?: boolean;
		minHeight?: string;
		showLineNumbers?: boolean;
	}

	let {
		value = $bindable(''),
		placeholder = '',
		readonly = false,
		minHeight = '400px',
		showLineNumbers = true
	}: Props = $props();

	let editorElement: HTMLDivElement;
	let editorView: EditorView | undefined;
	let isDarkMode = $state(false);
	let mounted = $state(false);

	// Compartments for reactive configuration
	const themeCompartment = new Compartment();
	const readonlyCompartment = new Compartment();

	// Configure code block language support
	const codeLanguages = [
		LanguageDescription.of({
			name: 'javascript',
			alias: ['js'],
			support: javascript()
		}),
		LanguageDescription.of({
			name: 'typescript',
			alias: ['ts'],
			support: javascript({ typescript: true })
		}),
		LanguageDescription.of({
			name: 'jsx',
			support: javascript({ jsx: true })
		}),
		LanguageDescription.of({
			name: 'tsx',
			support: javascript({ typescript: true, jsx: true })
		}),
		LanguageDescription.of({
			name: 'python',
			alias: ['py'],
			support: python()
		}),
		LanguageDescription.of({
			name: 'css',
			support: css()
		}),
		LanguageDescription.of({
			name: 'html',
			support: html()
		}),
		LanguageDescription.of({
			name: 'json',
			support: json()
		}),
		LanguageDescription.of({
			name: 'sql',
			support: sql()
		}),
		LanguageDescription.of({
			name: 'rust',
			alias: ['rs'],
			support: rust()
		})
	];

	// Create theme based on current mode and minHeight
	const getTheme = (dark: boolean) => {
		const baseTheme = EditorView.theme(
			{
				'&': {
					backgroundColor: 'transparent',
					color: 'var(--text-primary)',
					fontSize: '0.875rem',
					fontFamily: "'Monaco', 'Courier New', monospace",
					height: '100%'
				},
				'.cm-content': {
					caretColor: 'var(--text-primary)',
					padding: '0.75rem 1rem',
					minHeight: minHeight
				},
				'.cm-cursor': {
					borderLeftColor: 'var(--text-primary)'
				},
				'.cm-selectionBackground, ::selection': {
					backgroundColor: dark ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)'
				},
				'.cm-focused .cm-selectionBackground': {
					backgroundColor: dark ? 'rgba(255, 255, 255, 0.15)' : 'rgba(0, 0, 0, 0.15)'
				},
				'.cm-gutters': {
					backgroundColor: 'transparent',
					color: 'var(--text-primary)',
					opacity: dark ? '0.5' : '0.3',
					border: 'none',
					fontFamily: "'Monaco', 'Courier New', monospace",
					fontSize: '0.75rem'
				},
				'.cm-activeLineGutter': {
					backgroundColor: 'transparent',
					opacity: '0.7'
				},
				'.cm-line': {
					fontWeight: dark ? '400' : '300'
				}
			},
			{ dark }
		);

		// Dark mode uses oneDark which has built-in syntax highlighting
		// Light mode uses classHighlighter to generate .tok-* CSS classes
		if (dark) {
			return [oneDark, baseTheme];
		} else {
			// classHighlighter generates .tok-heading, .tok-strong, etc. classes
			// that our CSS can target for custom colors
			return [baseTheme, syntaxHighlighting(classHighlighter)];
		}
	};

	// Check for dark mode
	function checkDarkMode() {
		isDarkMode = document.documentElement.getAttribute('data-theme') === 'dark';
	}

	// Focus the editor (exposed for parent components)
	export function focus() {
		editorView?.focus();
	}

	onMount(() => {
		mounted = true;
		checkDarkMode();

		// Watch for theme changes
		const observer = new MutationObserver(() => {
			const wasDark = isDarkMode;
			checkDarkMode();

			if (editorView && wasDark !== isDarkMode) {
				editorView.dispatch({
					effects: themeCompartment.reconfigure(getTheme(isDarkMode))
				});
			}
		});

		observer.observe(document.documentElement, {
			attributes: true,
			attributeFilter: ['data-theme']
		});

		// Create editor with initial configuration
		try {
			const extensions = [
				markdown({ codeLanguages }),
				keymap.of([...defaultKeymap, indentWithTab]),
				EditorView.lineWrapping,
				readonlyCompartment.of(EditorView.editable.of(!readonly)),
				themeCompartment.of(getTheme(isDarkMode)),
				EditorView.updateListener.of((update) => {
					if (update.docChanged) {
						value = update.state.doc.toString();
					}
				})
			];

			// Add optional extensions
			if (showLineNumbers) {
				extensions.push(lineNumbers());
			}

			if (placeholder) {
				extensions.push(placeholderExt(placeholder));
			}

			const state = EditorState.create({
				doc: value,
				extensions
			});

			editorView = new EditorView({
				state,
				parent: editorElement
			});
		} catch (error) {
			console.error('Failed to initialize CodeMirror editor:', error);
		}

		return () => {
			observer.disconnect();
		};
	});

	// Watch for readonly changes
	$effect(() => {
		if (mounted && editorView) {
			editorView.dispatch({
				effects: readonlyCompartment.reconfigure(EditorView.editable.of(!readonly))
			});
		}
	});

	// Update editor content when value changes externally
	$effect(() => {
		if (mounted && editorView && value !== editorView.state.doc.toString()) {
			const currentPos = editorView.state.selection.main.head;
			const transaction = editorView.state.update({
				changes: {
					from: 0,
					to: editorView.state.doc.length,
					insert: value
				},
				// Try to preserve cursor position if possible
				selection: { anchor: Math.min(currentPos, value.length) }
			});
			editorView.dispatch(transaction);
		}
	});

	onDestroy(() => {
		if (editorView) {
			editorView.destroy();
		}
	});
</script>

<div
	bind:this={editorElement}
	class="markdown-editor"
	class:readonly
	style="--min-height: {minHeight}"
	role="textbox"
	aria-multiline="true"
	aria-label="Markdown editor"
	aria-readonly={readonly}
></div>

<style>
	.markdown-editor {
		width: 100%;
		border: 1px solid var(--border-subtle);
		transition: border-color 0.2s ease, opacity 0.2s ease;
		min-height: var(--min-height);
	}

	.markdown-editor:focus-within {
		border-color: var(--border-active);
	}

	.markdown-editor.readonly {
		opacity: 0.7;
		cursor: not-allowed;
		background: var(--border-subtle);
	}

	/* CodeMirror placeholder styling */
	:global(.markdown-editor .cm-placeholder) {
		color: var(--text-primary);
		opacity: 0.3;
		font-weight: 300;
	}

	/* Markdown syntax highlighting - Light mode */
	/* Using tok- prefix from classHighlighter */
	:global(:not([data-theme='dark']) .markdown-editor .tok-heading) {
		color: #2563eb;
		font-weight: 600;
	}

	:global(:not([data-theme='dark']) .markdown-editor .tok-strong) {
		font-weight: 700;
	}

	:global(:not([data-theme='dark']) .markdown-editor .tok-emphasis) {
		font-style: italic;
	}

	:global(:not([data-theme='dark']) .markdown-editor .tok-link) {
		color: #0ea5e9;
		text-decoration: underline;
	}

	:global(:not([data-theme='dark']) .markdown-editor .tok-url) {
		color: #06b6d4;
		opacity: 0.7;
	}

	:global(:not([data-theme='dark']) .markdown-editor .tok-quote) {
		color: #64748b;
		font-style: italic;
	}

	:global(:not([data-theme='dark']) .markdown-editor .tok-monospace) {
		color: #dc2626;
		background-color: rgba(0, 0, 0, 0.05);
		padding: 0.125rem 0.25rem;
		border-radius: 0.25rem;
	}

	:global(:not([data-theme='dark']) .markdown-editor .tok-meta) {
		color: #94a3b8;
	}

	:global(:not([data-theme='dark']) .markdown-editor .tok-list) {
		color: #7c3aed;
	}

	/* Code blocks - Light mode */
	:global(:not([data-theme='dark']) .markdown-editor .tok-keyword) {
		color: #d73a49;
		font-weight: 600;
	}

	:global(:not([data-theme='dark']) .markdown-editor .tok-variableName) {
		color: #24292e;
	}

	:global(:not([data-theme='dark']) .markdown-editor .tok-string) {
		color: #032f62;
	}

	:global(:not([data-theme='dark']) .markdown-editor .tok-comment) {
		color: #6a737d;
		font-style: italic;
	}

	:global(:not([data-theme='dark']) .markdown-editor .tok-number) {
		color: #005cc5;
	}

	:global(:not([data-theme='dark']) .markdown-editor .tok-operator) {
		color: #d73a49;
	}

	:global(:not([data-theme='dark']) .markdown-editor .tok-className) {
		color: #6f42c1;
	}

	:global(:not([data-theme='dark']) .markdown-editor .tok-function) {
		color: #6f42c1;
	}

	:global(:not([data-theme='dark']) .markdown-editor .tok-typeName) {
		color: #005cc5;
	}

	/* Markdown syntax highlighting - Dark mode */
	:global([data-theme='dark'] .markdown-editor .tok-heading) {
		color: #60a5fa;
		font-weight: 600;
	}

	:global([data-theme='dark'] .markdown-editor .tok-strong) {
		font-weight: 700;
	}

	:global([data-theme='dark'] .markdown-editor .tok-emphasis) {
		font-style: italic;
	}

	:global([data-theme='dark'] .markdown-editor .tok-link) {
		color: #38bdf8;
		text-decoration: underline;
	}

	:global([data-theme='dark'] .markdown-editor .tok-url) {
		color: #22d3ee;
		opacity: 0.7;
	}

	:global([data-theme='dark'] .markdown-editor .tok-quote) {
		color: #94a3b8;
		font-style: italic;
	}

	:global([data-theme='dark'] .markdown-editor .tok-monospace) {
		color: #fca5a5;
		background-color: rgba(255, 255, 255, 0.05);
		padding: 0.125rem 0.25rem;
		border-radius: 0.25rem;
	}

	:global([data-theme='dark'] .markdown-editor .tok-meta) {
		color: #64748b;
	}

	:global([data-theme='dark'] .markdown-editor .tok-list) {
		color: #a78bfa;
	}

	/* Code blocks - Dark mode */
	:global([data-theme='dark'] .markdown-editor .tok-keyword) {
		color: #ff7b72;
		font-weight: 600;
	}

	:global([data-theme='dark'] .markdown-editor .tok-variableName) {
		color: #e6edf3;
	}

	:global([data-theme='dark'] .markdown-editor .tok-string) {
		color: #a5d6ff;
	}

	:global([data-theme='dark'] .markdown-editor .tok-comment) {
		color: #8b949e;
		font-style: italic;
	}

	:global([data-theme='dark'] .markdown-editor .tok-number) {
		color: #79c0ff;
	}

	:global([data-theme='dark'] .markdown-editor .tok-operator) {
		color: #ff7b72;
	}

	:global([data-theme='dark'] .markdown-editor .tok-className) {
		color: #d2a8ff;
	}

	:global([data-theme='dark'] .markdown-editor .tok-function) {
		color: #d2a8ff;
	}

	:global([data-theme='dark'] .markdown-editor .tok-typeName) {
		color: #79c0ff;
	}
</style>
