<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { EditorView, keymap, lineNumbers, Decoration } from '@codemirror/view';
	import type { DecorationSet } from '@codemirror/view';
	import { EditorState, Compartment, StateEffect, StateField } from '@codemirror/state';
	import { markdown } from '@codemirror/lang-markdown';
	import { oneDark } from '@codemirror/theme-one-dark';
	import { defaultKeymap, selectAll } from '@codemirror/commands';
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
		content: string;
		highlightedLineStart?: number | null;
		highlightedLineEnd?: number | null;
		commentThreadLines?: Array<{ start: number; end: number }>;
		onselectionchange?: (detail: { lineStart: number; lineEnd: number; x: number; y: number }) => void;
		onclearselection?: () => void;
	}

	let {
		content = '',
		highlightedLineStart = null,
		highlightedLineEnd = null,
		commentThreadLines = [],
		onselectionchange,
		onclearselection
	}: Props = $props();

	let editorElement: HTMLDivElement;
	let editorView: EditorView | undefined;
	let isDarkMode = $state(false);
	let mounted = $state(false);
	let selectionTimeout: ReturnType<typeof setTimeout> | null = null;

	// Expose method to get line position relative to the editor wrapper
	// Returns { top: number, isVisible: boolean } or null if editor not ready
	export function getLinePosition(lineNumber: number): { top: number; isVisible: boolean } | null {
		if (!editorView || !editorElement) {
			return null;
		}

		try {
			const line = editorView.state.doc.line(lineNumber);
			const coords = editorView.coordsAtPos(line.from);

			if (coords) {
				// Line is visible - get exact position
				const editorRect = editorElement.getBoundingClientRect();
				const position = coords.top - editorRect.top + editorView.scrollDOM.scrollTop;

				return { top: position, isVisible: true };
			} else {
				// Line is not currently rendered (off-screen due to virtual scrolling)
				return { top: 0, isVisible: false };
			}
		} catch (e) {
			console.error('Error getting line position:', e);
			return null;
		}
	}

	// Expose method to get the offset from container top to where content actually starts
	// This accounts for panel headers, gutters, padding, etc.
	export function getEditorTopOffset(): number {
		if (!editorElement) return 0;

		// Get the editor element and its parent container
		const container = editorElement.closest('.plan-content-container') as HTMLElement;
		if (!container) return 0;

		const containerRect = container.getBoundingClientRect();
		const editorRect = editorElement.getBoundingClientRect();

		// This gives us the offset from container top to editor top
		// (accounts for panel header, padding, etc.)
		return editorRect.top - containerRect.top;
	}

	// Expose method to check if editor is ready
	export function isEditorReady(): boolean {
		return !!editorView && mounted;
	}

	// Expose method to subscribe to scroll events
	export function onScroll(callback: () => void): () => void {
		if (!editorView) return () => {};

		const scrollDOM = editorView.scrollDOM;
		scrollDOM.addEventListener('scroll', callback);

		return () => scrollDOM.removeEventListener('scroll', callback);
	}

	// Expose method to get total line count
	export function getTotalLines(): number {
		if (!editorView) return 0;
		return editorView.state.doc.lines;
	}

	// Expose method to get default line height
	export function getLineHeight(): number {
		if (!editorView) return 20; // fallback
		return editorView.defaultLineHeight;
	}

	// Compartments for reactive configuration
	const themeCompartment = new Compartment();

	// State effect for setting highlighted lines (can handle multiple ranges)
	const setHighlightEffect = StateEffect.define<Array<{ start: number; end: number }> | null>();

	// State field for tracking highlighted lines
	const highlightField = StateField.define<DecorationSet>({
		create() {
			return Decoration.none;
		},
		update(decorations, tr) {
			decorations = decorations.map(tr.changes);
			for (let effect of tr.effects) {
				if (effect.is(setHighlightEffect)) {
					if (effect.value === null || effect.value.length === 0) {
						decorations = Decoration.none;
					} else {
						const ranges: ReturnType<Decoration['range']>[] = [];
						const doc = tr.state.doc;

						// Process all ranges
						effect.value.forEach(({ start, end }) => {
							for (let lineNum = start; lineNum <= end; lineNum++) {
								if (lineNum <= doc.lines) {
									const line = doc.line(lineNum);
									ranges.push(
										Decoration.line({
											class: 'highlighted-commenting-line'
										}).range(line.from)
									);
								}
							}
						});

						decorations = Decoration.set(ranges);
					}
				}
			}
			return decorations;
		},
		provide: f => EditorView.decorations.from(f)
	});

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

	// Create theme based on current mode
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
					padding: '0.75rem 1rem',
					caretColor: 'var(--text-primary)',
					cursor: 'text'
				},
				'.cm-selectionBackground, ::selection': {
					backgroundColor: dark ? 'rgba(255, 255, 255, 0.15)' : 'rgba(0, 0, 0, 0.1)'
				},
				'.cm-focused .cm-selectionBackground': {
					backgroundColor: dark ? 'rgba(255, 255, 255, 0.2)' : 'rgba(0, 0, 0, 0.15)'
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
				},
				// Style for lines with comments
				'.cm-line.has-comment': {
					backgroundColor: 'var(--border-subtle)',
					opacity: '0.9'
				},
				// Style for selected lines
				'.cm-line.selected-line': {
					backgroundColor: 'var(--border-subtle)'
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

	// Handle selection changes
	function handleSelectionChange(view: EditorView) {
		// Clear any pending timeout
		if (selectionTimeout) {
			clearTimeout(selectionTimeout);
		}

		const selection = view.state.selection.main;

		// If there's no selection or it's just a cursor, clear the selection
		if (selection.empty) {
			onclearselection?.();
			return;
		}

		// Debounce selection events to avoid too many updates
		selectionTimeout = setTimeout(() => {
			const { from, to } = selection;
			const doc = view.state.doc;

			// Get line numbers (1-based)
			const lineStart = doc.lineAt(from).number;
			const lineEnd = doc.lineAt(to).number;

			// Get the position of the selection start and end for button placement
			const startCoords = view.coordsAtPos(from);
			const endCoords = view.coordsAtPos(to);
			if (startCoords && endCoords) {
				const editorRect = editorElement.getBoundingClientRect();
				const x = endCoords.right - editorRect.left;
				// Calculate the vertical midpoint of the selection
				const y = ((startCoords.top + endCoords.bottom) / 2) - editorRect.top;

				onselectionchange?.({ lineStart, lineEnd, x, y });
			}
		}, 200);
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
			const themeExtensions = getTheme(isDarkMode);

			const extensions = [
				markdown({ codeLanguages }),
				// Add Ctrl/Cmd+A for select all
				keymap.of([
					{ key: 'Mod-a', run: selectAll },
					...defaultKeymap
				]),
				EditorView.lineWrapping,
				// Use readOnly instead of editable to allow selections
				EditorState.readOnly.of(true),
				lineNumbers(),
				themeCompartment.of(themeExtensions),
				highlightField,
				// Listen for selection changes
				EditorView.updateListener.of((update) => {
					if (update.selectionSet) {
						handleSelectionChange(update.view);
					}
				})
			];

			const state = EditorState.create({
				doc: content,
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
			if (selectionTimeout) {
				clearTimeout(selectionTimeout);
			}
		};
	});

	// Update editor content when content changes externally
	$effect(() => {
		if (mounted && editorView && content !== editorView.state.doc.toString()) {
			const transaction = editorView.state.update({
				changes: {
					from: 0,
					to: editorView.state.doc.length,
					insert: content
				}
			});
			editorView.dispatch(transaction);
		}
	});

	// Update highlighting when highlightedLineStart/End or commentThreadLines change
	$effect(() => {
		if (mounted && editorView) {
			const ranges = [];

			// Add selected lines highlight (takes priority)
			if (highlightedLineStart !== null && highlightedLineEnd !== null) {
				ranges.push({ start: highlightedLineStart, end: highlightedLineEnd });
			}

			// Add comment thread lines highlights
			if (commentThreadLines && commentThreadLines.length > 0) {
				ranges.push(...commentThreadLines);
			}

			if (ranges.length > 0) {
				editorView.dispatch({
					effects: setHighlightEffect.of(ranges)
				});
			} else {
				editorView.dispatch({
					effects: setHighlightEffect.of(null)
				});
			}
		}
	});

	onDestroy(() => {
		if (selectionTimeout) {
			clearTimeout(selectionTimeout);
		}
		if (editorView) {
			editorView.destroy();
		}
	});
</script>

<div
	bind:this={editorElement}
	class="commentable-markdown-viewer"
	role="textbox"
	aria-multiline="true"
	aria-label="Plan content viewer"
	aria-readonly="true"
></div>

<style>
	.commentable-markdown-viewer {
		width: 100%;
		border: 1px solid var(--border-subtle);
		transition: border-color 0.2s ease;
		min-height: 400px;
	}

	.commentable-markdown-viewer:focus-within {
		border-color: var(--border-active);
	}

	/* Markdown syntax highlighting - Light mode */
	:global(:not([data-theme='dark']) .commentable-markdown-viewer .tok-heading) {
		color: #2563eb;
		font-weight: 600;
	}

	:global(:not([data-theme='dark']) .commentable-markdown-viewer .tok-strong) {
		font-weight: 700;
	}

	:global(:not([data-theme='dark']) .commentable-markdown-viewer .tok-emphasis) {
		font-style: italic;
	}

	:global(:not([data-theme='dark']) .commentable-markdown-viewer .tok-link) {
		color: #0ea5e9;
		text-decoration: underline;
	}

	:global(:not([data-theme='dark']) .commentable-markdown-viewer .tok-url) {
		color: #06b6d4;
		opacity: 0.7;
	}

	:global(:not([data-theme='dark']) .commentable-markdown-viewer .tok-quote) {
		color: #64748b;
		font-style: italic;
	}

	:global(:not([data-theme='dark']) .commentable-markdown-viewer .tok-monospace) {
		color: #dc2626;
		background-color: rgba(0, 0, 0, 0.05);
		padding: 0.125rem 0.25rem;
		border-radius: 0.25rem;
	}

	:global(:not([data-theme='dark']) .commentable-markdown-viewer .tok-meta) {
		color: #94a3b8;
	}

	:global(:not([data-theme='dark']) .commentable-markdown-viewer .tok-list) {
		color: #7c3aed;
	}

	/* Code blocks - Light mode */
	:global(:not([data-theme='dark']) .commentable-markdown-viewer .tok-keyword) {
		color: #d73a49;
		font-weight: 600;
	}

	:global(:not([data-theme='dark']) .commentable-markdown-viewer .tok-variableName) {
		color: #24292e;
	}

	:global(:not([data-theme='dark']) .commentable-markdown-viewer .tok-string) {
		color: #032f62;
	}

	:global(:not([data-theme='dark']) .commentable-markdown-viewer .tok-comment) {
		color: #6a737d;
		font-style: italic;
	}

	:global(:not([data-theme='dark']) .commentable-markdown-viewer .tok-number) {
		color: #005cc5;
	}

	:global(:not([data-theme='dark']) .commentable-markdown-viewer .tok-operator) {
		color: #d73a49;
	}

	:global(:not([data-theme='dark']) .commentable-markdown-viewer .tok-className) {
		color: #6f42c1;
	}

	:global(:not([data-theme='dark']) .commentable-markdown-viewer .tok-function) {
		color: #6f42c1;
	}

	:global(:not([data-theme='dark']) .commentable-markdown-viewer .tok-typeName) {
		color: #005cc5;
	}

	/* Markdown syntax highlighting - Dark mode */
	:global([data-theme='dark'] .commentable-markdown-viewer .tok-heading) {
		color: #60a5fa;
		font-weight: 600;
	}

	:global([data-theme='dark'] .commentable-markdown-viewer .tok-strong) {
		font-weight: 700;
	}

	:global([data-theme='dark'] .commentable-markdown-viewer .tok-emphasis) {
		font-style: italic;
	}

	:global([data-theme='dark'] .commentable-markdown-viewer .tok-link) {
		color: #38bdf8;
		text-decoration: underline;
	}

	:global([data-theme='dark'] .commentable-markdown-viewer .tok-url) {
		color: #22d3ee;
		opacity: 0.7;
	}

	:global([data-theme='dark'] .commentable-markdown-viewer .tok-quote) {
		color: #94a3b8;
		font-style: italic;
	}

	:global([data-theme='dark'] .commentable-markdown-viewer .tok-monospace) {
		color: #fca5a5;
		background-color: rgba(255, 255, 255, 0.05);
		padding: 0.125rem 0.25rem;
		border-radius: 0.25rem;
	}

	:global([data-theme='dark'] .commentable-markdown-viewer .tok-meta) {
		color: #64748b;
	}

	:global([data-theme='dark'] .commentable-markdown-viewer .tok-list) {
		color: #a78bfa;
	}

	/* Code blocks - Dark mode */
	:global([data-theme='dark'] .commentable-markdown-viewer .tok-keyword) {
		color: #ff7b72;
		font-weight: 600;
	}

	:global([data-theme='dark'] .commentable-markdown-viewer .tok-variableName) {
		color: #e6edf3;
	}

	:global([data-theme='dark'] .commentable-markdown-viewer .tok-string) {
		color: #a5d6ff;
	}

	:global([data-theme='dark'] .commentable-markdown-viewer .tok-comment) {
		color: #8b949e;
		font-style: italic;
	}

	:global([data-theme='dark'] .commentable-markdown-viewer .tok-number) {
		color: #79c0ff;
	}

	:global([data-theme='dark'] .commentable-markdown-viewer .tok-operator) {
		color: #ff7b72;
	}

	:global([data-theme='dark'] .commentable-markdown-viewer .tok-className) {
		color: #d2a8ff;
	}

	:global([data-theme='dark'] .commentable-markdown-viewer .tok-function) {
		color: #d2a8ff;
	}

	:global([data-theme='dark'] .commentable-markdown-viewer .tok-typeName) {
		color: #79c0ff;
	}

	/* Highlighted lines for commenting */
	:global(.commentable-markdown-viewer .highlighted-commenting-line) {
		background-color: rgba(255, 200, 0, 0.15);
		border-left: 3px solid rgba(255, 200, 0, 0.5);
	}

	:global([data-theme='dark'] .commentable-markdown-viewer .highlighted-commenting-line) {
		background-color: rgba(255, 200, 0, 0.1);
		border-left: 3px solid rgba(255, 200, 0, 0.4);
	}
</style>
