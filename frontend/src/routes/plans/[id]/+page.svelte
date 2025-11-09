<script lang="ts">
	import { onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import LoginButton from '$lib/components/LoginButton.svelte';
	import CommentableMarkdownViewer from '$lib/components/CommentableMarkdownViewer.svelte';
	import Presence from '$lib/components/Presence.svelte';
	import ToggleSwitch from '$lib/components/ToggleSwitch.svelte';
	import { planCommentsStore } from '$lib/stores/planComments';
	import { authStore } from '$lib/stores/auth';

	const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:8083';
	const JOB_POLL_INTERVAL_MS = 2000;
	const MAX_POLL_RETRIES = 30; // 60 seconds total

	interface Plan {
		id: string;
		title: string;
		content: string;
		content_hash: string;
		owner_id: string;
		is_public: boolean;
		current_version: number;
		file_size_bytes: number;
		created_at: string;
		updated_at: string;
	}

	interface CommentWithAuthor {
		id: string;
		plan_id: string;
		plan_version: number;
		author_id: string;
		line_start: number;
		line_end: number;
		comment_text: string;
		is_resolved: boolean;
		resolved_at: string | null;
		resolved_by: string | null;
		resolution_action: 'accepted' | 'rejected' | null;
		created_at: string;
		updated_at: string;
		author_username: string;
		author_first_name: string | null;
		author_last_name: string | null;
	}

	interface PlanWithComments {
		plan: Plan;
		comments: CommentWithAuthor[];
		owner_username: string;
	}

	const planId = $derived($page.params.id);

	let planData: PlanWithComments | null = $state(null);
	let loading = $state(true);
	let error = $state('');
	let showResolvedColumn = $state(false);
	let activeCommentId: string | null = $state(null);
	let selectedLineStart: number | null = $state(null);
	let selectedLineEnd: number | null = $state(null);
	let selectionY: number = $state(0);
	let commentText = $state('');
	let showCommentButton = $state(false);
	let showCommentForm = $state(false);
	let submittingComment = $state(false);
	let processingAction: string | null = $state(null);
	let jobPolling: { [key: string]: { interval: NodeJS.Timeout; retries: number } } = {};
	let wsError = $state('');
	let viewerRef: any = $state(null);

	// Get active (non-resolved) comment thread line ranges for highlighting
	const activeCommentThreadLines = $derived.by(() => {
		if (!planData) return [];
		return planData.comments
			.filter(c => !c.is_resolved)
			.map(c => ({ start: c.line_start, end: c.line_end }));
	});

	// Check if the current user is the plan owner
	const isOwner = $derived.by(() => {
		const currentUser = $authStore.user;
		if (!currentUser || !planData) return false;
		return currentUser.id === planData.plan.owner_id;
	});

	$effect(() => {
		if (planId) {
			loadPlan();

			// Capture planId to avoid stale reference in cleanup
			const currentPlanId = planId;

			planCommentsStore.subscribeToPlan(
				currentPlanId,
				(message) => {
					if (planId !== currentPlanId || !planData) return;

					if (message.type === 'comment_added') {
						const newComment = message.comment;
						if (!newComment) return; // Guard against missing comment data

						// Check if this comment already exists to prevent duplicates
						const exists = planData.comments.some(c => c.id === newComment.id);
						if (exists) {
							// Comment already added (HTTP response completed first), just remove temp comments
							planData = {
								...planData,
								comments: planData.comments.filter(c => !c.id.startsWith('temp-'))
							};
							return;
						}

						// Remove optimistic comment that matches this real comment by line range and text
						// This prevents removing unrelated temp comments from other pending submissions
						const filteredComments = planData.comments.filter(c => {
							if (!c.id.startsWith('temp-')) return true;
							// Remove temp comment if it matches the new comment's line range and text
							return !(
								c.line_start === newComment.line_start &&
								c.line_end === newComment.line_end &&
								c.comment_text === newComment.comment_text
							);
						});

						planData = {
							...planData,
							comments: [...filteredComments, newComment]
						};
					} else if (message.type === 'comment_updated') {
						// Update existing comment
						const updatedComment = message.comment;
						if (!updatedComment) return; // Guard against missing comment data

						planData = {
							...planData,
							comments: planData.comments.map(c =>
								c.id === updatedComment.id ? updatedComment : c
							)
						};
					} else if (message.type === 'comment_deleted') {
						// Remove comment
						planData = {
							...planData,
							comments: planData.comments.filter(c => c.id !== (message.comment_id ?? ''))
						};
					}
				},
				(errorMsg: string) => {
					wsError = errorMsg;
					console.error('WebSocket error:', errorMsg);
				}
			);

			return () => {
				planCommentsStore.unsubscribe();
			};
		}
	});

	onDestroy(() => {
		Object.values(jobPolling).forEach(({ interval }) => clearInterval(interval));
		planCommentsStore.cleanup();
	});

	async function loadPlan() {
		try {
			loading = true;
			const response = await fetch(`${API_BASE}/api/plans/${planId}`);

			if (response.ok) {
				const data = await response.json();
				planData = data;
			} else {
				const errorData = await response.text();
				console.error('Failed to load plan:', response.status, errorData);
				error = 'Failed to load plan';
			}
		} catch (err) {
			error = 'An error occurred while loading the plan';
			console.error('Error loading plan:', err);
		} finally {
			loading = false;
		}
	}

	function handleSelectionChange(detail: { lineStart: number; lineEnd: number; x: number; y: number }) {
		const { lineStart, lineEnd, y } = detail;
		selectedLineStart = lineStart;
		selectedLineEnd = lineEnd;
		selectionY = y;
		showCommentButton = true;
		showCommentForm = false;
	}

	function handleClearSelection() {
		showCommentButton = false;
		showCommentForm = false;
		selectedLineStart = null;
		selectedLineEnd = null;
		commentText = '';
	}

	function openCommentForm() {
		showCommentButton = false;
		showCommentForm = true;
	}

	function cancelComment() {
		showCommentForm = false;
		showCommentButton = false;
		selectedLineStart = null;
		selectedLineEnd = null;
		commentText = '';
	}

	async function submitComment() {
		if (!selectedLineStart || !selectedLineEnd || !commentText.trim() || !planId) return;

		submittingComment = true;
		error = '';

		// Create optimistic comment with robust temporary ID
		const optimisticComment: CommentWithAuthor = {
			id: `temp-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`, // Unique temporary ID
			plan_id: planId,
			plan_version: planData?.plan.current_version || 1,
			author_id: $authStore.user?.id || '',
			line_start: selectedLineStart,
			line_end: selectedLineEnd,
			comment_text: commentText.trim(),
			is_resolved: false,
			resolved_at: null,
			resolved_by: null,
			resolution_action: null,
			created_at: new Date().toISOString(),
			updated_at: new Date().toISOString(),
			author_username: $authStore.user?.username || '',
			author_first_name: $authStore.user?.first_name || null,
			author_last_name: $authStore.user?.last_name || null
		};

		// Optimistically add comment to UI
		if (planData) {
			planData = {
				...planData,
				comments: [...planData.comments, optimisticComment]
			};
		}

		try {
			const response = await fetch(`${API_BASE}/api/plans/${planId}/comments`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				credentials: 'include',
				body: JSON.stringify({
					line_start: selectedLineStart,
					line_end: selectedLineEnd,
					comment_text: commentText.trim()
				})
			});

			if (response.ok) {
				// Success - the WebSocket will handle updating with real data
				showCommentForm = false;
				showCommentButton = false;
				commentText = '';
			} else {
				// Remove optimistic comment on error
				if (planData) {
					planData = {
						...planData,
						comments: planData.comments.filter(c => c.id !== optimisticComment.id)
					};
				}
				const data = await response.json();
				error = data.error || 'Failed to submit comment';
			}
		} catch (err) {
			// Remove optimistic comment on error
			if (planData) {
				planData = {
					...planData,
					comments: planData.comments.filter(c => c.id !== optimisticComment.id)
				};
			}
			error = 'An error occurred while submitting the comment';
			console.error(err);
		} finally {
			submittingComment = false;
		}
	}

	async function acceptComment(commentId: string) {
		processingAction = commentId;
		error = '';

		try {
			const response = await fetch(`${API_BASE}/api/comments/${commentId}/accept`, {
				method: 'POST',
				credentials: 'include'
			});

			if (response.ok) {
				const result = await response.json();
				// Start polling for job status
				startJobPolling(result.job_id, commentId);
			} else {
				const data = await response.json();
				error = data.error || 'Failed to accept comment';
				processingAction = null;
			}
		} catch (err) {
			error = 'An error occurred while accepting the comment';
			console.error(err);
			processingAction = null;
		}
	}

	async function rejectComment(commentId: string) {
		processingAction = commentId;
		error = '';

		try {
			const response = await fetch(`${API_BASE}/api/comments/${commentId}/reject`, {
				method: 'POST',
				credentials: 'include'
			});

			if (response.ok) {
				await loadPlan();
			} else {
				const data = await response.json();
				error = data.error || 'Failed to reject comment';
			}
		} catch (err) {
			error = 'An error occurred while rejecting the comment';
			console.error(err);
		} finally {
			processingAction = null;
		}
	}

	function startJobPolling(jobId: string, commentId: string) {
		let retries = 0;

		const interval = setInterval(async () => {
			retries++;

			// Stop polling after max retries
			if (retries > MAX_POLL_RETRIES) {
				clearInterval(interval);
				delete jobPolling[commentId];
				error = 'Job status check timed out. The AI integration may still be processing.';
				processingAction = null;
				return;
			}

			try {
				// Use the new user-accessible endpoint
				const response = await fetch(`${API_BASE}/api/jobs/${jobId}`, {
					credentials: 'include'
				});

				if (response.ok) {
					const job = await response.json();

					if (job.status === 'completed') {
						clearInterval(interval);
						delete jobPolling[commentId];
						processingAction = null;
						await loadPlan();
					} else if (job.status === 'failed') {
						clearInterval(interval);
						delete jobPolling[commentId];
						error = 'AI integration failed: ' + (job.result_data?.error || 'Unknown error');
						processingAction = null;
					}
				} else if (response.status === 404) {
					// Job not found, might not exist yet
					console.warn('Job not found yet, will retry...');
				} else if (response.status === 403) {
					clearInterval(interval);
					delete jobPolling[commentId];
					error = 'Permission denied to check job status';
					processingAction = null;
				} else {
					// Other errors - log but keep trying
					console.error('Error checking job status:', response.status);
				}
			} catch (err) {
				console.error('Error polling job status:', err);
				// Network errors - keep trying until max retries
			}
		}, JOB_POLL_INTERVAL_MS);

		jobPolling[commentId] = { interval, retries: 0 };
	}

	function formatDate(dateString: string): string {
		return new Date(dateString).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function downloadPlan() {
		if (!planData) return;
		const blob = new Blob([planData.plan.content], { type: 'text/markdown' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `${planData.plan.title}.md`;
		a.click();
		URL.revokeObjectURL(url);
	}

	// Comment positioning system
	type CommentGroup = {
		id: string; // Unique ID for this group
		lineStart: number;
		lineEnd: number;
		comments: CommentWithAuthor[];
		element?: HTMLElement; // Ref to the DOM element
		measuredHeight: number; // Actual measured height
		collapsedHeight: number; // Fixed collapsed height
		calculatedTop: number; // Final calculated position
		originalTop: number; // Original position before being pushed
		isVisible: boolean; // Whether the line is currently visible in editor
		isExpanded: boolean; // Whether this comment is currently expanded
	};

	let commentGroups: CommentGroup[] = $state([]);
	let commentThreadRefs = new Map<string, HTMLElement>();
	let resizeObserver: ResizeObserver | null = null;
	let updateScheduled = false;
	let scrollUnsubscribe: (() => void) | null = null;

	// Action to register comment thread elements
	function registerCommentThread(node: HTMLElement, groupId: string) {
		commentThreadRefs.set(groupId, node);

		// Observe this element
		if (resizeObserver) {
			resizeObserver.observe(node);
		}

		return {
			destroy() {
				commentThreadRefs.delete(groupId);
				if (resizeObserver) {
					resizeObserver.unobserve(node);
				}
			}
		};
	}

	const COLLAPSED_HEIGHT = 160; // Fixed height: header + 2 lines of text + buttons + padding

	// Each comment is its own independent group - no grouping
	const groupedComments = $derived.by(() => {
		if (!planData) return [];

		const groups: Omit<CommentGroup, 'element' | 'measuredHeight' | 'calculatedTop' | 'originalTop' | 'isVisible' | 'isExpanded'>[] = [];

		planData.comments.forEach(comment => {
			// Each comment gets its own unique group
			const groupId = `comment-${comment.id}`;

			groups.push({
				id: groupId,
				lineStart: comment.line_start,
				lineEnd: comment.line_end,
				comments: [comment],
				collapsedHeight: COLLAPSED_HEIGHT
			});
		});

		return groups.sort((a, b) => a.lineStart - b.lineStart);
	});

	// Calculate positions for all comment threads
	function calculatePositions() {
		if (!viewerRef || !viewerRef.isEditorReady()) {
			console.log('calculatePositions: editor not ready');
			return;
		}

		const editorTopOffset = viewerRef.getEditorTopOffset() || 0;
		const lineHeight = viewerRef.getLineHeight() || 20;
		console.log('Editor top offset from container:', editorTopOffset);
		console.log('Line height:', lineHeight);

		const GAP = 16; // Gap between non-overlapping threads

		// First pass: get line positions from editor
		const updatedGroups = groupedComments.map(group => {
			const element = commentThreadRefs.get(group.id);
			const lineStartPos = viewerRef.getLinePosition(group.lineStart);
			const lineEndPos = viewerRef.getLinePosition(group.lineEnd);

			console.log(`Group ${group.id} (lines ${group.lineStart}-${group.lineEnd}):`, lineStartPos, lineEndPos);

			// Get measured height from existing group or default
			const existingGroup = commentGroups.find(g => g.id === group.id);
			const isExpanded = group.comments[0].id === activeCommentId;
			const measuredHeight = existingGroup?.measuredHeight || COLLAPSED_HEIGHT;

			// Use collapsed height unless this comment is expanded
			const effectiveHeight = isExpanded ? measuredHeight : group.collapsedHeight;

			let calculatedTop = 0;
			let isVisible = false;

			if (lineStartPos && lineEndPos && lineStartPos.isVisible && lineEndPos.isVisible) {
				// Both lines visible - center the comment on the highlighted range
				const highlightTop = lineStartPos.top;
				const highlightBottom = lineEndPos.top + lineHeight;
				const highlightMiddle = (highlightTop + highlightBottom) / 2;

				// Position comment so its middle aligns with highlight middle
				calculatedTop = highlightMiddle - (effectiveHeight / 2) + editorTopOffset;
				isVisible = true;

				console.log(`  -> Centering: highlightTop=${highlightTop}, highlightBottom=${highlightBottom}, middle=${highlightMiddle}`);
				console.log(`  -> Comment: height=${effectiveHeight}, calculatedTop=${calculatedTop}`);
			} else if (lineStartPos && lineStartPos.isVisible) {
				// Only start visible - position at start
				calculatedTop = lineStartPos.top + editorTopOffset;
				isVisible = true;
			} else {
				// Off-screen - estimate position based on line number
				const estimatedTop = (group.lineStart - 1) * lineHeight;
				calculatedTop = estimatedTop + editorTopOffset;
				isVisible = false;
				console.log(`  -> Off-screen, estimated position: ${calculatedTop}`);
			}

			return {
				...group,
				element,
				measuredHeight,
				collapsedHeight: group.collapsedHeight,
				calculatedTop,
				originalTop: calculatedTop,
				isVisible,
				isExpanded
			};
		});

		// Second pass: prevent overlaps
		// Separate active and resolved comments - they occupy different spaces
		const activeGroups = updatedGroups.filter(g => !g.comments.every(c => c.is_resolved));
		const resolvedGroups = updatedGroups.filter(g => g.comments.every(c => c.is_resolved));

		// Sort each group separately
		const sortedActive = [...activeGroups].sort((a, b) => a.calculatedTop - b.calculatedTop);
		const sortedResolved = [...resolvedGroups].sort((a, b) => a.calculatedTop - b.calculatedTop);

		// Handle active comments overlap prevention
		let activePreviousBottom = 0;
		sortedActive.forEach(group => {
			const effectiveHeight = group.isExpanded ? group.measuredHeight : group.collapsedHeight;
			const calculatedBottom = group.calculatedTop + effectiveHeight;
			console.log(`Positioning active group ${group.id} (${group.isVisible ? 'visible' : 'off-screen'}, ${group.isExpanded ? 'expanded' : 'collapsed'}):`);
			console.log(`  top=${group.calculatedTop}, height=${effectiveHeight}, bottom=${calculatedBottom}`);
			console.log(`  previousBottom=${activePreviousBottom}`);

			if (group.calculatedTop < activePreviousBottom) {
				console.log(`  -> Overlap detected! Adjusting ${group.calculatedTop} -> ${activePreviousBottom}`);
				group.calculatedTop = activePreviousBottom;
			}

			activePreviousBottom = group.calculatedTop + effectiveHeight + GAP;
			console.log(`  -> Final: top=${group.calculatedTop}, bottom=${group.calculatedTop + effectiveHeight}, nextBottom=${activePreviousBottom}`);
		});

		// Handle resolved comments overlap prevention (separate from active)
		let resolvedPreviousBottom = 0;
		sortedResolved.forEach(group => {
			const effectiveHeight = group.isExpanded ? group.measuredHeight : group.collapsedHeight;
			const calculatedBottom = group.calculatedTop + effectiveHeight;
			console.log(`Positioning resolved group ${group.id} (${group.isVisible ? 'visible' : 'off-screen'}, ${group.isExpanded ? 'expanded' : 'collapsed'}):`);
			console.log(`  top=${group.calculatedTop}, height=${effectiveHeight}, bottom=${calculatedBottom}`);
			console.log(`  previousBottom=${resolvedPreviousBottom}`);

			if (group.calculatedTop < resolvedPreviousBottom) {
				console.log(`  -> Overlap detected! Adjusting ${group.calculatedTop} -> ${resolvedPreviousBottom}`);
				group.calculatedTop = resolvedPreviousBottom;
			}

			resolvedPreviousBottom = group.calculatedTop + effectiveHeight + GAP;
			console.log(`  -> Final: top=${group.calculatedTop}, bottom=${group.calculatedTop + effectiveHeight}, nextBottom=${resolvedPreviousBottom}`);
		});

		// Combine both groups for final output
		const allGroups = [...sortedActive, ...sortedResolved];
		console.log('Final commentGroups:', allGroups);
		commentGroups = allGroups;
	}

	// Schedule position update with requestAnimationFrame to avoid layout thrashing
	function schedulePositionUpdate() {
		if (updateScheduled) return;

		updateScheduled = true;
		requestAnimationFrame(() => {
			calculatePositions();
			updateScheduled = false;
		});
	}

	// Set up ResizeObserver and event listeners
	$effect(() => {
		if (!viewerRef || !planData) return;

		// Set up ResizeObserver to measure comment thread heights
		// The actual observing is done in the registerCommentThread action
		resizeObserver = new ResizeObserver((entries) => {
			let hasChanges = false;

			entries.forEach(entry => {
				const element = entry.target as HTMLElement;
				const groupId = element.dataset.groupId;

				if (groupId) {
					const group = commentGroups.find(g => g.id === groupId);
					// Use offsetHeight to include padding and borders
					const fullHeight = element.offsetHeight;

					if (group && Math.abs(group.measuredHeight - fullHeight) > 1) {
						console.log(`Height updated for ${groupId}: ${group.measuredHeight} -> ${fullHeight}`);
						group.measuredHeight = fullHeight;
						hasChanges = true;
					}
				}
			});

			if (hasChanges) {
				schedulePositionUpdate();
			}
		});

		// Subscribe to editor scroll events
		const checkEditorReady = () => {
			if (viewerRef.isEditorReady && viewerRef.isEditorReady()) {
				// Initial calculation
				calculatePositions();

				// Subscribe to scroll
				scrollUnsubscribe = viewerRef.onScroll(() => {
					schedulePositionUpdate();
				});
			} else {
				setTimeout(checkEditorReady, 50);
			}
		};

		setTimeout(checkEditorReady, 100);

		// Cleanup
		return () => {
			if (resizeObserver) {
				resizeObserver.disconnect();
				resizeObserver = null;
			}
			if (scrollUnsubscribe) {
				scrollUnsubscribe();
				scrollUnsubscribe = null;
			}
		};
	});

	// Recalculate when window resizes
	$effect(() => {
		const handleResize = () => schedulePositionUpdate();
		window.addEventListener('resize', handleResize);

		return () => window.removeEventListener('resize', handleResize);
	});

	// Recalculate when grouped comments change (e.g., new comments added)
	$effect(() => {
		// Access groupedComments to make this reactive
		const groups = groupedComments;
		console.log('Grouped comments changed:', groups.length, 'groups');

		if (viewerRef && viewerRef.isEditorReady && viewerRef.isEditorReady()) {
			console.log('Triggering position recalculation due to comment changes');
			schedulePositionUpdate();
		}
	});

	// Recalculate when active comment changes (expansion/collapse)
	$effect(() => {
		// Access activeCommentId to make this reactive
		const activeId = activeCommentId;
		console.log('Active comment changed:', activeId);

		if (viewerRef && viewerRef.isEditorReady && viewerRef.isEditorReady()) {
			schedulePositionUpdate();
		}
	});

	// Handle comment click to expand/collapse
	function handleCommentClick(commentId: string, event: MouseEvent | KeyboardEvent) {
		// Toggle active state
		if (activeCommentId === commentId) {
			activeCommentId = null;
		} else {
			activeCommentId = commentId;
		}
		event.stopPropagation();
	}

	// Handle keyboard events for comment expansion
	function handleCommentKeydown(commentId: string, event: KeyboardEvent) {
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			handleCommentClick(commentId, event);
		}
	}

	// Click outside to collapse
	$effect(() => {
		const handleClickOutside = (event: MouseEvent) => {
			const target = event.target as HTMLElement;
			if (!target.closest('.inline-comment-thread') && !target.closest('.resolved-column-thread')) {
				activeCommentId = null;
			}
		};

		document.addEventListener('click', handleClickOutside);
		return () => document.removeEventListener('click', handleClickOutside);
	});
</script>

<svelte:head>
	<title>{planData?.plan.title || 'Plan'}</title>
</svelte:head>

<ThemeToggle />
<LoginButton />
<Presence />

<div class="container">
	{#if loading}
		<div class="loading">Loading plan...</div>
	{:else if error && !planData}
		<div class="error">{error}</div>
	{:else if planData}
		<header class="page-header">
			<div class="header-content">
				<a href="/plans" class="back-link">← BACK</a>
				<h1>{planData.plan.title}</h1>
				<div class="plan-meta">
					<span>by {planData.owner_username}</span>
					<span>v{planData.plan.current_version}</span>
					<span>{planData.comments.length} {planData.comments.length === 1 ? 'comment' : 'comments'}</span>
				</div>
			</div>
			<div class="header-actions">
				<button onclick={downloadPlan} class="download-btn">Download</button>
			</div>
		</header>

		{#if error}
			<div class="error-message">{error}</div>
		{/if}

		{#if wsError}
			<div class="error-message">
				⚠️ Real-time updates disconnected: {wsError}. Refresh the page to see latest comments.
			</div>
		{/if}

		<div class="content-wrapper">
			<div class="plan-content-container">
				<div class="panel-header">
					<span>Plan Content</span>
					<ToggleSwitch bind:checked={showResolvedColumn} label="Show Resolved" />
				</div>
				<div class="editor-wrapper">
					<CommentableMarkdownViewer
						bind:this={viewerRef}
						content={planData.plan.content}
						onselectionchange={handleSelectionChange}
						onclearselection={handleClearSelection}
						highlightedLineStart={selectedLineStart}
						highlightedLineEnd={selectedLineEnd}
						commentThreadLines={activeCommentThreadLines}
					/>

					{#if showCommentButton}
						<button
							class="add-comment-btn"
							style="top: {selectionY}px; left: calc(100% + 1rem);"
							onclick={openCommentForm}
							onkeydown={(e) => {
								if (e.key === 'Enter' || e.key === ' ') {
									e.preventDefault();
									openCommentForm();
								}
							}}
							title="Add comment"
							aria-label="Add comment to selected lines"
							tabindex="0"
						>
							+
						</button>
					{/if}
				</div>
			</div>

			<div class="inline-comments-container">
				{#if showCommentForm && selectedLineStart && selectedLineEnd}
					<div class="inline-comment-form" style="top: {selectionY}px; transform: translateY(-50%);">
						<div class="comment-form-header">
							<h3 class="form-title">
								Lines {selectedLineStart}{#if selectedLineEnd !== selectedLineStart}-{selectedLineEnd}{/if}
							</h3>
							<button onclick={cancelComment} class="close-btn">×</button>
						</div>
						<textarea
							bind:value={commentText}
							rows="4"
							placeholder="Enter your comment..."
							class="comment-textarea"
							autofocus
						></textarea>
						<div class="form-actions">
							<button
								onclick={submitComment}
								disabled={submittingComment || !commentText.trim()}
								class="btn-primary"
							>
								{submittingComment ? 'Submitting...' : 'Submit'}
							</button>
							<button onclick={cancelComment} class="cancel-btn-small">Cancel</button>
						</div>
					</div>
				{/if}

				{#if !showCommentForm && commentGroups.filter(g => showResolvedColumn ? g.comments.every(c => c.is_resolved) : !g.comments.every(c => c.is_resolved)).length === 0}
					<div class="empty-comments-state">
						<p>{showResolvedColumn ? 'No resolved comments' : 'No comments yet'}</p>
						{#if !showResolvedColumn}
							<p class="empty-comments-hint">Select text in the plan to add a comment</p>
						{/if}
					</div>
				{/if}

				{#each commentGroups.filter(g => showResolvedColumn ? g.comments.every(c => c.is_resolved) : !g.comments.every(c => c.is_resolved)) as group (group.id)}
					{@const isRejected = group.comments.every(c => c.resolution_action === 'rejected')}
					{@const isAccepted = group.comments.every(c => c.resolution_action === 'accepted')}
					{@const isResolved = group.comments.every(c => c.is_resolved)}
					<div
						use:registerCommentThread={group.id}
						data-group-id={group.id}
						class="inline-comment-thread"
						class:rejected-thread={isRejected}
						class:accepted-thread={isAccepted}
						class:resolved-thread={isResolved}
						class:off-screen={!group.isVisible}
						class:expanded={group.isExpanded}
						style={`top: ${group.calculatedTop}px;`}
						role="button"
						tabindex="0"
						onclick={(e) => handleCommentClick(group.comments[0]?.id, e)}
						onkeydown={(e) => handleCommentKeydown(group.comments[0]?.id, e)}
					>
						<div class="thread-header">
							<p class="thread-lines">Lines {group.lineStart}{#if group.lineEnd !== group.lineStart}-{group.lineEnd}{/if}</p>
							{#if group.comments.length > 0}
								<p class="thread-date">{formatDate(group.comments[0].created_at)}</p>
							{/if}
						</div>
						{#each group.comments as comment (comment.id)}
							<div class="inline-comment" class:resolved={comment.is_resolved}>
								<p class="comment-text">
									<span class="comment-author">@{comment.author_username}</span> {comment.comment_text}
								</p>
								{#if !comment.is_resolved && isOwner}
									<div class="comment-actions">
										<button
											onclick={() => acceptComment(comment.id)}
											disabled={processingAction === comment.id}
											class="accept-btn"
										>
											{processingAction === comment.id ? 'Processing...' : 'Accept'}
										</button>
										<button
											onclick={() => rejectComment(comment.id)}
											disabled={processingAction !== null}
											class="reject-btn"
										>
											Reject
										</button>
									</div>
								{/if}
								{#if comment.is_resolved}
									<div class="comment-actions">
										<span class="resolution-badge" class:accepted={comment.resolution_action === 'accepted'}>
											{comment.resolution_action}
										</span>
									</div>
								{/if}
							</div>
						{/each}
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>

<style>
	.container {
		max-width: 1600px;
		margin: 0 auto;
		padding: 6rem 2rem 4rem 2rem;
	}

	.loading,
	.error {
		text-align: center;
		padding: 3rem;
		font-size: 1rem;
		color: var(--text-primary);
		opacity: 0.8;
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-end;
		margin-bottom: 2rem;
		padding-bottom: 1.5rem;
		border-bottom: 1px solid var(--border-subtle);
		gap: 2rem;
	}

	.header-content {
		flex: 1;
	}

	.header-actions {
		display: flex;
		gap: 1rem;
		align-items: center;
	}


	.back-link {
		display: inline-block;
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		text-decoration: none;
		color: var(--text-primary);
		opacity: 0.7;
		margin-bottom: 1rem;
		transition: opacity 0.2s ease;
	}

	.back-link:hover {
		opacity: 1;
	}

	.page-header h1 {
		font-size: clamp(1.5rem, 4vw, 2.5rem);
		font-weight: 300;
		letter-spacing: 0.1em;
		margin: 0 0 0.75rem 0;
		color: var(--text-primary);
	}

	:global([data-theme='dark']) .page-header h1 {
		font-weight: 500;
	}

	:global([data-theme='light']) .page-header h1 {
		font-weight: 200;
	}

	.plan-meta {
		display: flex;
		gap: 1.5rem;
		font-size: 0.75rem;
		font-weight: 300;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--text-secondary);
	}

	.download-btn {
		padding: 0.75rem 1.5rem;
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--text-primary);
		background: transparent;
		border: 1px solid var(--border-subtle);
		cursor: pointer;
		transition: opacity 0.2s ease, border-color 0.2s ease;
		font-family: inherit;
		white-space: nowrap;
	}

	.download-btn:hover {
		opacity: 0.7;
		border-color: var(--border-active);
	}

	.error-message {
		padding: 1rem 1.5rem;
		margin-bottom: 2rem;
		border: 1px solid var(--border-subtle);
		color: var(--text-primary);
		opacity: 0.8;
		font-size: 0.875rem;
	}

	.instructions {
		padding: 1.5rem;
		margin-bottom: 2rem;
		border: 1px solid var(--border-subtle);
		font-size: 0.875rem;
	}

	.instructions-title {
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		margin-bottom: 0.75rem;
		opacity: 0.8;
	}

	.instructions ul {
		list-style: disc;
		padding-left: 1.5rem;
		margin: 0;
	}

	.instructions li {
		margin-bottom: 0.25rem;
		font-weight: 300;
		opacity: 0.8;
	}

	.content-wrapper {
		display: grid;
		grid-template-columns: 1fr 400px;
		gap: 2rem;
		position: relative;
	}

	.plan-content-container {
		border: 1px solid var(--border-subtle);
		min-height: 600px;
	}

	.panel-header {
		padding: 1rem 1.5rem;
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		border-bottom: 1px solid var(--border-subtle);
		opacity: 0.8;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.editor-wrapper {
		position: relative;
	}

	.add-comment-btn {
		position: absolute;
		width: 2rem;
		height: 2rem;
		border-radius: 0;
		background: var(--text-primary);
		color: var(--bg-primary);
		border: 2px solid var(--bg-primary);
		cursor: pointer;
		font-size: 1.5rem;
		font-weight: 500;
		line-height: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0;
		padding-bottom: 0.125rem;
		box-shadow: none;
		transition: opacity 0.2s ease, border-color 0.2s ease;
		z-index: 10;
	}

	.add-comment-btn:hover {
		opacity: 0.8;
		border-color: var(--border-active);
	}

	.inline-comments-container {
		position: relative;
		min-height: 100px;
		overflow: visible;
	}

	.empty-comments-state {
		padding: 3rem 2rem;
		text-align: center;
		color: var(--text-primary);
		opacity: 0.5;
	}

	.empty-comments-state p {
		margin: 0 0 0.5rem 0;
		font-size: 0.875rem;
		font-weight: 300;
	}

	.empty-comments-hint {
		font-size: 0.75rem;
		opacity: 0.7;
	}

	.inline-comment-form {
		position: absolute;
		padding: 1.5rem;
		border: 1px solid var(--border-subtle);
		margin-bottom: 1.5rem;
		animation: slideIn 0.2s ease-out;
		width: 100%;
		box-sizing: border-box;
		z-index: 20; /* Above comment threads */
	}

	:global([data-theme='dark']) .inline-comment-form {
		background: #000000;
	}

	:global([data-theme='light']) .inline-comment-form {
		background: #ffffff;
	}

	@keyframes slideIn {
		from {
			opacity: 0;
			transform: translateY(-50%) translateX(1rem);
		}
		to {
			opacity: 1;
			transform: translateY(-50%) translateX(0);
		}
	}

	.comment-form-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 1rem;
	}

	.form-title {
		font-size: 0.875rem;
		font-weight: 300;
		letter-spacing: 0.05em;
		margin: 0;
	}

	.close-btn {
		background: transparent;
		border: none;
		font-size: 1.5rem;
		line-height: 1;
		color: var(--text-primary);
		cursor: pointer;
		padding: 0;
		width: 1.5rem;
		height: 1.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
		opacity: 0.5;
		transition: opacity 0.2s ease;
	}

	.close-btn:hover {
		opacity: 1;
	}

	.comment-textarea {
		width: 100%;
		padding: 0.75rem;
		font-size: 0.875rem;
		font-family: inherit;
		font-weight: 300;
		color: var(--text-primary);
		background: transparent;
		border: 1px solid var(--border-subtle);
		outline: none;
		resize: vertical;
		margin-bottom: 1rem;
	}

	.comment-textarea:focus {
		border-color: var(--border-active);
	}

	.comment-textarea::placeholder {
		color: var(--text-primary);
		opacity: 0.3;
	}

	.form-actions {
		display: flex;
		gap: 0.75rem;
	}

	.cancel-btn-small {
		padding: 0.6rem 1rem;
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--text-primary);
		background: transparent;
		border: 1px solid var(--border-subtle);
		cursor: pointer;
		transition: opacity 0.2s ease, border-color 0.2s ease;
		font-family: inherit;
	}

	.cancel-btn-small:hover {
		opacity: 0.7;
		border-color: var(--border-active);
	}

	.btn-primary {
		flex: 1;
	}

	.inline-comment-thread {
		position: absolute;
		width: 100%;
		padding: 1.25rem;
		background: var(--bg-primary);
		border: 1px solid var(--border-subtle);
		box-sizing: border-box;
		transition: transform 0.3s ease, opacity 0.3s ease, top 0.3s ease;
		overflow: hidden;
		max-height: 160px; /* Collapsed: header + 2 lines text + buttons + padding */
		cursor: pointer;
	}

	.inline-comment-thread.expanded {
		max-height: none;
		overflow: visible;
		cursor: default;
	}

	/* Hover effect for collapsed comments */
	.inline-comment-thread:not(.expanded):hover {
		border-color: var(--border-active);
	}

	/* Focus styles for keyboard navigation */
	.inline-comment-thread:focus {
		outline: 2px solid var(--border-active);
		outline-offset: 2px;
	}

	.inline-comment-thread.off-screen {
		position: relative;
	}

	.inline-comment-thread.off-screen::before {
		content: '';
		position: absolute;
		left: 0;
		top: 0;
		bottom: 0;
		width: 3px;
		background: var(--border-subtle);
		opacity: 0.5;
	}

	.thread-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
		gap: 1rem;
	}

	.thread-lines,
	.thread-date {
		font-size: 0.75rem;
		font-weight: 300;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		color: var(--text-secondary);
		margin: 0;
		opacity: 0.8;
	}

	.thread-date {
		white-space: nowrap;
	}

	.inline-comment {
		padding: 0.75rem 0;
		border-top: 1px solid var(--border-subtle);
	}

	.inline-comment:first-of-type {
		border-top: none;
		padding-top: 0;
	}

	.inline-comment:last-of-type {
		padding-bottom: 0;
	}

	.inline-comment.resolved {
		opacity: 0.6;
	}

	.comment-author {
		font-weight: 300;
		opacity: 0.6;
	}

	:global([data-theme='dark']) .comment-author {
		font-weight: 400;
	}

	.resolution-badge {
		padding: 0.25rem 0.75rem;
		font-size: 0.625rem;
		font-weight: 300;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		border: 1px solid var(--border-subtle);
	}

	.resolution-badge.accepted {
		opacity: 0.8;
	}

	.comment-text {
		font-size: 0.875rem;
		font-weight: 300;
		line-height: 1.5;
		margin: 0 0 0.5rem 0;
		white-space: pre-wrap;
	}

	/* Truncate text when comment thread is collapsed */
	.inline-comment-thread:not(.expanded) .comment-text,
	.resolved-column-thread:not(.expanded) .comment-text {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: normal;
	}

	.comment-actions {
		display: flex;
		gap: 0.5rem;
		margin-top: 0.75rem;
	}

	.accept-btn,
	.reject-btn {
		flex: 1;
		padding: 0.4rem 0.75rem;
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
	}

	.accept-btn:hover:not(:disabled),
	.reject-btn:hover:not(:disabled) {
		opacity: 0.7;
		border-color: var(--border-active);
	}

	.accept-btn:disabled,
	.reject-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	@media (max-width: 1024px) {
		.content-wrapper {
			grid-template-columns: 1fr;
		}

		.inline-comments-container {
			order: -1;
		}

		.add-comment-btn {
			left: auto;
			right: 1rem;
		}
	}

	@media (max-width: 768px) {
		.container {
			padding: 5rem 1rem 3rem 1rem;
		}

		.page-header {
			flex-direction: column;
			gap: 1rem;
		}

		.plan-meta {
			flex-wrap: wrap;
			gap: 1rem;
		}

		.instructions {
			padding: 1rem;
		}

		.inline-comment-thread {
			padding: 1.25rem;
		}
	}
</style>
