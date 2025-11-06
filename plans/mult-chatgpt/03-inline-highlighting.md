# Extension 03 - Inline Highlighting & UI Enhancements

**Builds on:** Extension 02 - Enhanced Status
**Next:** Extension 04 - Plan Diffs

---

## What This Adds

Visual highlighting of commented sections directly in the plan, with click-to-jump navigation and improved markdown rendering.

**New Features:**
- Highlight commented lines in different colors by status
- Click highlighted section to jump to comment
- Hover to see comment preview
- Proper markdown rendering with `marked` library
- Syntax highlighting for code blocks
- Better mobile responsiveness

---

## No Database Changes

This is purely a frontend enhancement.

---

## Frontend Implementation

### 1. Install Dependencies

```bash
cd frontend
npm install marked highlight.js
npm install -D @types/marked
```

---

### 2. Create Markdown Renderer - `frontend/src/lib/components/MarkdownRenderer.svelte`

```svelte
<script lang="ts">
  import { marked } from 'marked';
  import hljs from 'highlight.js';
  import 'highlight.js/styles/github-dark.css';
  import { onMount } from 'svelte';

  interface Props {
    content: string;
    highlightedLines?: Set<number>;  // Lines with comments
    onLineClick?: (lineNum: number) => void;
  }

  let { content, highlightedLines = new Set(), onLineClick }: Props = $props();

  let renderedHtml = $state('');

  // Configure marked with syntax highlighting
  onMount(() => {
    marked.setOptions({
      highlight: (code, lang) => {
        if (lang && hljs.getLanguage(lang)) {
          try {
            return hljs.highlight(code, { language: lang }).value;
          } catch (e) {
            console.error('Highlight error:', e);
          }
        }
        return hljs.highlightAuto(code).value;
      },
      breaks: true,
      gfm: true,
    });

    renderMarkdown();
  });

  function renderMarkdown() {
    try {
      renderedHtml = marked.parse(content) as string;
    } catch (e) {
      console.error('Markdown parse error:', e);
      renderedHtml = `<pre>${content}</pre>`;
    }
  }

  // Re-render when content changes
  $effect(() => {
    content;
    renderMarkdown();
  });
</script>

<div class="prose prose-slate dark:prose-invert max-w-none">
  {@html renderedHtml}
</div>

<style>
  :global(.prose) {
    font-size: 0.95rem;
    line-height: 1.6;
  }

  :global(.prose h1) {
    font-size: 2rem;
    font-weight: bold;
    margin-top: 2rem;
    margin-bottom: 1rem;
    border-bottom: 2px solid #e5e7eb;
    padding-bottom: 0.5rem;
  }

  :global(.prose h2) {
    font-size: 1.5rem;
    font-weight: bold;
    margin-top: 1.5rem;
    margin-bottom: 0.75rem;
  }

  :global(.prose h3) {
    font-size: 1.25rem;
    font-weight: semibold;
    margin-top: 1.25rem;
    margin-bottom: 0.5rem;
  }

  :global(.prose pre) {
    background-color: #1e293b;
    border-radius: 0.5rem;
    padding: 1rem;
    overflow-x: auto;
  }

  :global(.prose code) {
    background-color: #f1f5f9;
    padding: 0.2rem 0.4rem;
    border-radius: 0.25rem;
    font-size: 0.875rem;
  }

  :global(.prose pre code) {
    background-color: transparent;
    padding: 0;
  }

  :global(.prose a) {
    color: #3b82f6;
    text-decoration: underline;
  }

  :global(.prose a:hover) {
    color: #2563eb;
  }

  :global(.prose ul, .prose ol) {
    margin-left: 1.5rem;
  }

  :global(.prose li) {
    margin-top: 0.5rem;
  }

  :global(.prose blockquote) {
    border-left: 4px solid #e5e7eb;
    padding-left: 1rem;
    font-style: italic;
    color: #6b7280;
  }

  :global(.prose table) {
    width: 100%;
    border-collapse: collapse;
    margin: 1rem 0;
  }

  :global(.prose th, .prose td) {
    border: 1px solid #e5e7eb;
    padding: 0.5rem;
    text-align: left;
  }

  :global(.prose th) {
    background-color: #f9fafb;
    font-weight: bold;
  }
</style>
```

---

### 3. Enhanced Line Selector with Highlighting - `frontend/src/lib/components/EnhancedLineSelector.svelte`

```svelte
<script lang="ts">
  import { plansApi, type CommentWithAuthor } from '$lib/api/plans';

  interface Props {
    content: string;
    comments: CommentWithAuthor[];
    selectedLines: { start: number; end: number; text: string } | null;
    planId: string;
    canComment: boolean;
    onselect: (detail: { start: number; end: number; text: string }) => void;
    oncommentsubmit: () => void;
    onjumptocomment: (commentId: string) => void;  // New
  }

  let {
    content,
    comments,
    selectedLines,
    planId,
    canComment,
    onselect,
    oncommentsubmit,
    onjumptocomment,
  }: Props = $props();

  let lines = $derived(content.split('\n'));
  let commentContent = $state('');
  let submitting = $state(false);
  let error = $state<string | null>(null);
  let hoveredLine = $state<number | null>(null);

  // Build map of line -> comments
  const lineToComments = $derived(() => {
    const map = new Map<number, CommentWithAuthor[]>();
    comments.forEach((comment) => {
      for (let i = comment.start_line; i <= comment.end_line; i++) {
        if (!map.has(i)) map.set(i, []);
        map.get(i)!.push(comment);
      }
    });
    return map;
  });

  function handleLineClick(lineNum: number, event: MouseEvent) {
    // Check if clicking on an existing comment
    const commentsOnLine = lineToComments().get(lineNum);
    if (commentsOnLine && commentsOnLine.length > 0 && !event.shiftKey) {
      // Jump to first active comment on this line
      const activeComment = commentsOnLine.find(
        (c) => c.status === 'pending' || c.status === 'debating'
      );
      if (activeComment) {
        onjumptocomment(activeComment.id);
        return;
      }
    }

    if (!canComment) return;

    if (event.shiftKey && selectedLines) {
      // Extend selection
      const start = Math.min(selectedLines.start, lineNum);
      const end = Math.max(selectedLines.start, lineNum);
      const text = lines.slice(start - 1, end).join('\n');
      onselect({ start, end, text });
    } else {
      // Start new selection
      onselect({ start: lineNum, end: lineNum, text: lines[lineNum - 1] });
    }
  }

  function isLineSelected(lineNum: number): boolean {
    if (!selectedLines) return false;
    return lineNum >= selectedLines.start && lineNum <= selectedLines.end;
  }

  function getLineBackgroundClass(lineNum: number): string {
    const commentsOnLine = lineToComments().get(lineNum);

    if (isLineSelected(lineNum)) {
      return 'bg-blue-200 dark:bg-blue-800 border-l-4 border-blue-500';
    }

    if (commentsOnLine && commentsOnLine.length > 0) {
      // Prioritize: pending/debating > accepted > rejected
      const hasPending = commentsOnLine.some(
        (c) => c.status === 'pending' || c.status === 'debating'
      );
      const hasAccepted = commentsOnLine.some((c) => c.status === 'accepted');
      const hasRejected = commentsOnLine.some((c) => c.status === 'rejected');

      if (hasPending) {
        return 'bg-yellow-100 dark:bg-yellow-900 border-l-4 border-yellow-500 cursor-pointer';
      } else if (hasAccepted) {
        return 'bg-green-50 dark:bg-green-900 border-l-2 border-green-400 opacity-60';
      } else if (hasRejected) {
        return 'bg-gray-100 dark:bg-gray-800 border-l-2 border-gray-400 opacity-40';
      }
    }

    return 'hover:bg-gray-100 dark:hover:bg-gray-800';
  }

  function getLineTooltip(lineNum: number): string {
    const commentsOnLine = lineToComments().get(lineNum);
    if (!commentsOnLine || commentsOnLine.length === 0) return '';

    const activeComments = commentsOnLine.filter(
      (c) => c.status === 'pending' || c.status === 'debating'
    );

    if (activeComments.length === 0) return '';

    const first = activeComments[0];
    const preview =
      first.content.length > 60 ? `${first.content.slice(0, 60)}...` : first.content;
    return `@${first.author_username}: ${preview}`;
  }

  async function submitComment() {
    if (!selectedLines || !commentContent.trim()) return;

    submitting = true;
    error = null;

    try {
      await plansApi.createComment(
        planId,
        commentContent,
        selectedLines.start,
        selectedLines.end,
        selectedLines.text
      );
      commentContent = '';
      oncommentsubmit();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to create comment';
    } finally {
      submitting = false;
    }
  }

  function cancelComment() {
    onselect(null as any);
    commentContent = '';
    error = null;
  }
</script>

<div class="border rounded bg-white dark:bg-gray-900 shadow-sm">
  <!-- Line-numbered content with highlighting -->
  <div class="p-4 font-mono text-sm overflow-x-auto">
    {#each lines as line, i}
      {@const lineNum = i + 1}
      {@const bgClass = getLineBackgroundClass(lineNum)}
      {@const tooltip = getLineTooltip(lineNum)}

      <div
        class="flex {bgClass} transition-colors duration-150"
        onclick={(e) => handleLineClick(lineNum, e)}
        onmouseenter={() => (hoveredLine = lineNum)}
        onmouseleave={() => (hoveredLine = null)}
        title={tooltip}
        role="button"
        tabindex="0"
      >
        <span class="text-gray-400 select-none w-14 text-right pr-4 flex-shrink-0">
          {lineNum}
        </span>
        <span class="flex-1 whitespace-pre-wrap break-all pr-4">
          {line || ' '}
        </span>
        {#if hoveredLine === lineNum && lineToComments().get(lineNum)}
          <span class="text-xs text-gray-500 flex-shrink-0">
            💬 {lineToComments().get(lineNum)?.length}
          </span>
        {/if}
      </div>
    {/each}
  </div>

  <!-- Comment Form -->
  {#if selectedLines && selectedLines.start > 0}
    <div class="border-t p-4 bg-blue-50 dark:bg-blue-900">
      <p class="text-sm mb-2 font-semibold">
        Comment on lines {selectedLines.start}-{selectedLines.end}
      </p>
      <textarea
        bind:value={commentContent}
        placeholder="Add your comment..."
        class="w-full border rounded px-3 py-2 mb-2 text-sm"
        rows="3"
      />
      {#if error}
        <p class="text-red-600 text-sm mb-2">{error}</p>
      {/if}
      <div class="flex gap-2">
        <button
          onclick={submitComment}
          disabled={submitting || !commentContent.trim()}
          class="bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white px-4 py-2 rounded text-sm font-medium"
        >
          {submitting ? 'Submitting...' : 'Submit Comment'}
        </button>
        <button
          onclick={cancelComment}
          class="bg-gray-400 hover:bg-gray-500 text-white px-4 py-2 rounded text-sm"
        >
          Cancel
        </button>
      </div>
      <p class="text-xs text-gray-600 dark:text-gray-400 mt-2">
        💡 Tip: Shift+click to select multiple lines
      </p>
    </div>
  {/if}
</div>

<style>
  /* Add smooth transitions */
  div[role='button'] {
    cursor: pointer;
  }
</style>
```

---

### 4. Update Plan Viewer with Toggle - `frontend/src/routes/plans/[id]/+page.svelte`

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { plansApi, type PlanDetailResponse, type CommentWithAuthor } from '$lib/api/plans';
  import { authStore } from '$lib/stores/auth';
  import EnhancedLineSelector from '$lib/components/EnhancedLineSelector.svelte';
  import MarkdownRenderer from '$lib/components/MarkdownRenderer.svelte';
  import CommentSidebar from '$lib/components/CommentSidebar.svelte';
  import VersionHistory from '$lib/components/VersionHistory.svelte';

  let data = $state<PlanDetailResponse | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let selectedLines = $state<{ start: number; end: number; text: string } | null>(null);
  let viewMode = $state<'rendered' | 'raw'>('raw');  // Default to raw for commenting

  const planId = $derived($page.params.id);
  const isOwner = $derived(
    data?.plan && $authStore.user && data.plan.user_id === $authStore.user.id
  );

  onMount(async () => {
    await loadPlan();
  });

  async function loadPlan() {
    loading = true;
    try {
      data = await plansApi.getPlan(planId);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load plan';
    } finally {
      loading = false;
    }
  }

  function handleLineSelection(detail: { start: number; end: number; text: string }) {
    selectedLines = detail;
  }

  async function handleCommentSubmitted() {
    selectedLines = null;
    await loadPlan();
  }

  async function handleCommentAction() {
    await loadPlan();
  }

  function jumpToComment(commentId: string) {
    // Scroll to comment in sidebar
    const commentElement = document.getElementById(`comment-${commentId}`);
    if (commentElement) {
      commentElement.scrollIntoView({ behavior: 'smooth', block: 'center' });
      // Flash highlight
      commentElement.classList.add('highlight-flash');
      setTimeout(() => {
        commentElement.classList.remove('highlight-flash');
      }, 2000);
    }
  }

  function downloadPlan() {
    if (!data) return;
    const blob = new Blob([data.plan.content], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = data.plan.filename;
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="container mx-auto px-4 py-8">
  {#if loading}
    <div class="flex items-center justify-center h-64">
      <p class="text-gray-600">Loading plan...</p>
    </div>
  {:else if error}
    <p class="text-red-600">{error}</p>
  {:else if data}
    <!-- Header -->
    <div class="flex flex-col md:flex-row justify-between items-start md:items-center gap-4 mb-6">
      <div>
        <h1 class="text-3xl font-bold">{data.plan.title}</h1>
        <p class="text-gray-600">{data.plan.filename}</p>
        <p class="text-sm text-gray-500 mt-1">
          Version {data.current_version} • Updated {new Date(data.plan.updated_at).toLocaleDateString()}
        </p>
      </div>
      <div class="flex gap-2">
        <button
          onclick={() => (viewMode = viewMode === 'rendered' ? 'raw' : 'rendered')}
          class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded text-sm font-medium"
        >
          {viewMode === 'rendered' ? '📝 Raw View' : '👁 Rendered View'}
        </button>
        <button
          onclick={downloadPlan}
          class="bg-gray-600 hover:bg-gray-700 text-white px-4 py-2 rounded text-sm font-medium"
        >
          ⬇ Download
        </button>
      </div>
    </div>

    <!-- Version History (collapsible) -->
    <div class="mb-6">
      <VersionHistory planId={data.plan.id} />
    </div>

    <!-- Info banner -->
    {#if viewMode === 'rendered'}
      <div class="bg-yellow-50 dark:bg-yellow-900 border border-yellow-200 rounded p-3 mb-4 text-sm">
        ℹ️ Rendered view is read-only. Switch to Raw View to add comments.
      </div>
    {/if}

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Plan Content (2/3 width on large screens) -->
      <div class="lg:col-span-2">
        {#if viewMode === 'rendered'}
          <div class="border rounded p-6 bg-white dark:bg-gray-900 shadow-sm">
            <MarkdownRenderer content={data.plan.content} />
          </div>
        {:else}
          <EnhancedLineSelector
            content={data.plan.content}
            comments={data.comments}
            {selectedLines}
            planId={data.plan.id}
            canComment={!!$authStore.user}
            onselect={handleLineSelection}
            oncommentsubmit={handleCommentSubmitted}
            onjumptocomment={jumpToComment}
          />
        {/if}
      </div>

      <!-- Comments Sidebar (1/3 width on large screens) -->
      <div class="lg:col-span-1">
        <div class="lg:sticky lg:top-4">
          <CommentSidebar
            comments={data.comments}
            {isOwner}
            onaction={handleCommentAction}
          />
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  :global(.highlight-flash) {
    animation: flash 2s ease-in-out;
  }

  @keyframes flash {
    0%, 100% {
      background-color: transparent;
    }
    50% {
      background-color: rgba(59, 130, 246, 0.3);
    }
  }
</style>
```

---

### 5. Update Comment Sidebar to Support Jumping - `frontend/src/lib/components/CommentSidebar.svelte`

Add `id` attributes to comments:

```svelte
<div id="comment-{comment.id}" class="border rounded p-4 bg-white dark:bg-gray-900"
     class:border-yellow-400={comment.status === 'debating'}>
  <!-- ... rest of comment content ... -->
</div>
```

---

### 6. Add Legend Component - `frontend/src/lib/components/HighlightLegend.svelte`

```svelte
<script lang="ts">
  let expanded = $state(false);
</script>

<div class="border rounded p-3 bg-white dark:bg-gray-900 text-sm">
  <button
    onclick={() => (expanded = !expanded)}
    class="w-full flex items-center justify-between text-left font-semibold"
  >
    <span>💡 Highlight Guide</span>
    <span>{expanded ? '▼' : '▶'}</span>
  </button>

  {#if expanded}
    <div class="mt-3 space-y-2">
      <div class="flex items-center gap-2">
        <div class="w-4 h-4 bg-yellow-100 dark:bg-yellow-900 border-l-4 border-yellow-500"></div>
        <span>Active comment (click to view)</span>
      </div>
      <div class="flex items-center gap-2">
        <div class="w-4 h-4 bg-blue-200 dark:bg-blue-800 border-l-4 border-blue-500"></div>
        <span>Selected for new comment</span>
      </div>
      <div class="flex items-center gap-2">
        <div class="w-4 h-4 bg-green-50 dark:bg-green-900 border-l-2 border-green-400"></div>
        <span>Accepted comment</span>
      </div>
      <div class="flex items-center gap-2">
        <div class="w-4 h-4 bg-gray-100 dark:bg-gray-800 border-l-2 border-gray-400"></div>
        <span>Rejected comment</span>
      </div>
    </div>
  {/if}
</div>
```

Add to plan viewer:

```svelte
<!-- Before the grid -->
<div class="mb-4">
  <HighlightLegend />
</div>
```

---

### 7. Add TailwindCSS Typography Plugin (if not already installed)

```bash
cd frontend
npm install -D @tailwindcss/typography
```

Update `tailwind.config.js`:

```javascript
export default {
  // ... existing config
  plugins: [
    require('@tailwindcss/typography'),
  ],
}
```

---

## Testing Checklist

- [ ] Commented lines highlighted in yellow
- [ ] Click highlighted line jumps to comment in sidebar
- [ ] Hover shows comment preview
- [ ] Selected lines highlighted in blue
- [ ] Accepted comments shown in faded green
- [ ] Rejected comments shown in faded gray
- [ ] Multiple comments on same line show count on hover
- [ ] Rendered view shows properly formatted markdown
- [ ] Code blocks have syntax highlighting
- [ ] Sidebar scrolls to comment with flash animation
- [ ] Mobile responsive (sidebar below content)
- [ ] Dark mode works for all highlights

---

## Implementation Time

- Frontend: 2 days
- Testing & Polish: 1 day
- **Total: 3 days**

---

## Next Extension

👉 **Extension 04 - Plan Diffs & Version Comparison**

See: `04-plan-diffs.md`
