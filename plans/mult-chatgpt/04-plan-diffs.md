# Extension 04 - Plan Diffs & Version Comparison

**Builds on:** Extension 03 - Inline Highlighting
**Next:** Extension 05 - Collaboration Features

---

## What This Adds

Visual diff viewer to compare plan versions and understand what changed over time.

**New Features:**
- Side-by-side diff view
- Unified diff view
- View any two versions
- AI-generated change summaries
- Link versions to comments that triggered changes
- Export diff as markdown

---

## Backend Changes

### New Handler - `backend/src/handlers/plans.rs`

Add endpoint to get specific version content:

```rust
// Get specific version content
pub async fn get_plan_version(
    State(pool): State<PgPool>,
    Path((plan_id, version_number)): Path<(Uuid, i32)>,
) -> Result<Json<PlanVersion>, (StatusCode, String)> {
    let version = sqlx::query_as!(
        PlanVersion,
        r#"
        SELECT
            id, plan_id, version_number, content, comment_id, created_by,
            source as "source: VersionSource", summary, created_at
        FROM plan_versions
        WHERE plan_id = $1 AND version_number = $2
        "#,
        plan_id,
        version_number
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| (StatusCode::NOT_FOUND, "Version not found".to_string()))?;

    Ok(Json(version))
}

// Get diff between two versions (computed on backend)
pub async fn get_version_diff(
    State(pool): State<PgPool>,
    Path((plan_id, from_version, to_version)): Path<(Uuid, i32, i32)>,
) -> Result<Json<Value>, (StatusCode, String)> {
    // Fetch both versions
    let from = sqlx::query_as!(
        PlanVersion,
        r#"
        SELECT
            id, plan_id, version_number, content, comment_id, created_by,
            source as "source: VersionSource", summary, created_at
        FROM plan_versions
        WHERE plan_id = $1 AND version_number = $2
        "#,
        plan_id,
        from_version
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| (StatusCode::NOT_FOUND, "From version not found".to_string()))?;

    let to = sqlx::query_as!(
        PlanVersion,
        r#"
        SELECT
            id, plan_id, version_number, content, comment_id, created_by,
            source as "source: VersionSource", summary, created_at
        FROM plan_versions
        WHERE plan_id = $1 AND version_number = $2
        "#,
        plan_id,
        to_version
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| (StatusCode::NOT_FOUND, "To version not found".to_string()))?;

    // Basic stats
    let from_lines = from.content.lines().count();
    let to_lines = to.content.lines().count();
    let lines_added = to_lines.saturating_sub(from_lines);
    let lines_removed = from_lines.saturating_sub(to_lines);

    Ok(Json(json!({
        "from_version": from,
        "to_version": to,
        "stats": {
            "lines_added": lines_added,
            "lines_removed": lines_removed,
            "from_lines": from_lines,
            "to_lines": to_lines,
        }
    })))
}
```

---

### Update Routes - `backend/src/main.rs`

```rust
// Public plan routes
.route("/api/plans/:plan_id/versions/:version_number",
    get(plans::get_plan_version))
.route("/api/plans/:plan_id/diff/:from_version/:to_version",
    get(plans::get_version_diff))
```

---

## Frontend Implementation

### 1. Install Diff Library

```bash
cd frontend
npm install diff
npm install -D @types/diff
```

---

### 2. Create Diff Viewer Component - `frontend/src/lib/components/DiffViewer.svelte`

```svelte
<script lang="ts">
  import * as Diff from 'diff';
  import type { PlanVersion } from '$lib/api/plans';

  interface Props {
    fromVersion: PlanVersion;
    toVersion: PlanVersion;
    mode?: 'unified' | 'split';
  }

  let { fromVersion, toVersion, mode = 'unified' }: Props = $props();

  let diffMode = $state<'unified' | 'split'>(mode);

  interface DiffLine {
    type: 'added' | 'removed' | 'unchanged';
    content: string;
    lineNum?: number;
  }

  const diff = $derived(() => {
    const changes = Diff.diffLines(fromVersion.content, toVersion.content);

    const lines: DiffLine[] = [];
    let fromLineNum = 1;
    let toLineNum = 1;

    changes.forEach((part) => {
      const partLines = part.value.split('\n');
      // Remove last empty line if exists
      if (partLines[partLines.length - 1] === '') {
        partLines.pop();
      }

      partLines.forEach((line) => {
        if (part.added) {
          lines.push({ type: 'added', content: line, lineNum: toLineNum });
          toLineNum++;
        } else if (part.removed) {
          lines.push({ type: 'removed', content: line, lineNum: fromLineNum });
          fromLineNum++;
        } else {
          lines.push({ type: 'unchanged', content: line, lineNum: toLineNum });
          fromLineNum++;
          toLineNum++;
        }
      });
    });

    return lines;
  });

  const stats = $derived(() => {
    const added = diff().filter((l) => l.type === 'added').length;
    const removed = diff().filter((l) => l.type === 'removed').length;
    return { added, removed };
  });

  function getLineClass(type: string): string {
    switch (type) {
      case 'added':
        return 'bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200';
      case 'removed':
        return 'bg-red-100 dark:bg-red-900 text-red-800 dark:text-red-200';
      default:
        return 'bg-white dark:bg-gray-900';
    }
  }

  function getLinePrefix(type: string): string {
    switch (type) {
      case 'added':
        return '+';
      case 'removed':
        return '-';
      default:
        return ' ';
    }
  }

  function downloadDiff() {
    const diffText = diff()
      .map((line) => `${getLinePrefix(line.type)} ${line.content}`)
      .join('\n');

    const header = `--- Version ${fromVersion.version_number}\n+++ Version ${toVersion.version_number}\n\n`;
    const fullDiff = header + diffText;

    const blob = new Blob([fullDiff], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `v${fromVersion.version_number}-to-v${toVersion.version_number}.diff`;
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="border rounded bg-white dark:bg-gray-900">
  <!-- Header -->
  <div class="border-b p-4 bg-gray-50 dark:bg-gray-800">
    <div class="flex flex-col md:flex-row justify-between items-start md:items-center gap-4">
      <div>
        <h3 class="font-semibold text-lg">
          Version {fromVersion.version_number} → {toVersion.version_number}
        </h3>
        <div class="flex gap-4 mt-2 text-sm">
          <span class="text-green-600 dark:text-green-400">
            +{stats().added} lines added
          </span>
          <span class="text-red-600 dark:text-red-400">
            -{stats().removed} lines removed
          </span>
        </div>
      </div>
      <div class="flex gap-2">
        <button
          onclick={() => (diffMode = diffMode === 'unified' ? 'split' : 'unified')}
          class="text-sm bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600 px-3 py-1 rounded"
        >
          {diffMode === 'unified' ? 'Split View' : 'Unified View'}
        </button>
        <button
          onclick={downloadDiff}
          class="text-sm bg-blue-600 hover:bg-blue-700 text-white px-3 py-1 rounded"
        >
          Download Diff
        </button>
      </div>
    </div>
  </div>

  <!-- Diff Content -->
  <div class="p-4 overflow-x-auto">
    {#if diffMode === 'unified'}
      <!-- Unified View -->
      <div class="font-mono text-sm">
        {#each diff() as line}
          <div class="flex {getLineClass(line.type)}">
            <span class="w-8 text-right pr-2 text-gray-500 select-none">
              {line.lineNum || ''}
            </span>
            <span class="w-4 text-center select-none">{getLinePrefix(line.type)}</span>
            <span class="flex-1 pl-2 whitespace-pre-wrap break-all">
              {line.content || ' '}
            </span>
          </div>
        {/each}
      </div>
    {:else}
      <!-- Split View -->
      <div class="grid grid-cols-2 gap-4">
        <!-- Left: Removed lines -->
        <div>
          <div class="text-sm font-semibold mb-2 text-red-700 dark:text-red-400">
            Version {fromVersion.version_number}
          </div>
          <div class="font-mono text-sm border rounded">
            {#each diff().filter((l) => l.type !== 'added') as line}
              <div class="flex {line.type === 'removed' ? getLineClass(line.type) : ''}">
                <span class="w-8 text-right pr-2 text-gray-500 select-none">
                  {line.type === 'removed' ? line.lineNum : ''}
                </span>
                <span class="flex-1 pl-2 whitespace-pre-wrap break-all">
                  {line.content || ' '}
                </span>
              </div>
            {/each}
          </div>
        </div>

        <!-- Right: Added lines -->
        <div>
          <div class="text-sm font-semibold mb-2 text-green-700 dark:text-green-400">
            Version {toVersion.version_number}
          </div>
          <div class="font-mono text-sm border rounded">
            {#each diff().filter((l) => l.type !== 'removed') as line}
              <div class="flex {line.type === 'added' ? getLineClass(line.type) : ''}">
                <span class="w-8 text-right pr-2 text-gray-500 select-none">
                  {line.type === 'added' ? line.lineNum : ''}
                </span>
                <span class="flex-1 pl-2 whitespace-pre-wrap break-all">
                  {line.content || ' '}
                </span>
              </div>
            {/each}
          </div>
        </div>
      </div>
    {/if}
  </div>

  <!-- Version Metadata -->
  <div class="border-t p-4 bg-gray-50 dark:bg-gray-800 text-sm">
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <div>
        <p class="font-semibold mb-1">Version {fromVersion.version_number}</p>
        <p class="text-gray-600 dark:text-gray-400">
          {new Date(fromVersion.created_at).toLocaleString()}
        </p>
        {#if fromVersion.summary}
          <p class="text-gray-700 dark:text-gray-300 mt-1">{fromVersion.summary}</p>
        {/if}
      </div>
      <div>
        <p class="font-semibold mb-1">Version {toVersion.version_number}</p>
        <p class="text-gray-600 dark:text-gray-400">
          {new Date(toVersion.created_at).toLocaleString()}
        </p>
        {#if toVersion.summary}
          <p class="text-gray-700 dark:text-gray-300 mt-1">{toVersion.summary}</p>
        {/if}
      </div>
    </div>
  </div>
</div>
```

---

### 3. Update Version History Component - `frontend/src/lib/components/VersionHistory.svelte`

Add compare functionality:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { plansApi, type PlanVersion } from '$lib/api/plans';

  interface Props {
    planId: string;
    oncompare?: (from: PlanVersion, to: PlanVersion) => void;  // New
  }

  let { planId, oncompare }: Props = $props();

  let versions = $state<PlanVersion[]>([]);
  let loading = $state(true);
  let expanded = $state(false);
  let selectedVersions = $state<number[]>([]);  // New

  onMount(async () => {
    await loadVersions();
  });

  async function loadVersions() {
    try {
      versions = await plansApi.getPlanVersions(planId);
    } catch (e) {
      console.error('Failed to load versions', e);
    } finally {
      loading = false;
    }
  }

  function getSourceIcon(source: string): string {
    switch (source) {
      case 'manual':
        return '✏️';
      case 'ai_comment':
        return '🤖';
      case 'ai_discussion':
        return '💬🤖';
      default:
        return '📄';
    }
  }

  function getSourceLabel(source: string): string {
    switch (source) {
      case 'manual':
        return 'Manual edit';
      case 'ai_comment':
        return 'AI integration';
      case 'ai_discussion':
        return 'AI after discussion';
      default:
        return source;
    }
  }

  function toggleVersionSelection(versionNum: number) {
    if (selectedVersions.includes(versionNum)) {
      selectedVersions = selectedVersions.filter((v) => v !== versionNum);
    } else {
      if (selectedVersions.length >= 2) {
        selectedVersions = [selectedVersions[1], versionNum];
      } else {
        selectedVersions = [...selectedVersions, versionNum];
      }
    }
  }

  function compareSelected() {
    if (selectedVersions.length !== 2 || !oncompare) return;

    const [from, to] = selectedVersions.sort((a, b) => a - b);
    const fromVersion = versions.find((v) => v.version_number === from);
    const toVersion = versions.find((v) => v.version_number === to);

    if (fromVersion && toVersion) {
      oncompare(fromVersion, toVersion);
    }
  }

  function compareWithPrevious(version: PlanVersion) {
    if (version.version_number === 1 || !oncompare) return;

    const prevVersion = versions.find((v) => v.version_number === version.version_number - 1);
    if (prevVersion) {
      oncompare(prevVersion, version);
    }
  }
</script>

<div class="border rounded p-4 bg-gray-50 dark:bg-gray-900">
  <button
    onclick={() => (expanded = !expanded)}
    class="w-full flex items-center justify-between text-left"
  >
    <h3 class="font-semibold">📚 Version History ({versions.length})</h3>
    <span class="text-sm">{expanded ? '▼' : '▶'}</span>
  </button>

  {#if expanded}
    <div class="mt-4 space-y-2">
      {#if selectedVersions.length === 2}
        <button
          onclick={compareSelected}
          class="w-full bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded text-sm font-medium mb-2"
        >
          Compare v{Math.min(...selectedVersions)} → v{Math.max(...selectedVersions)}
        </button>
      {:else if selectedVersions.length === 1}
        <p class="text-sm text-gray-600 mb-2">Select another version to compare</p>
      {/if}

      {#if loading}
        <p class="text-sm text-gray-600">Loading versions...</p>
      {:else if versions.length === 0}
        <p class="text-sm text-gray-600">No version history yet</p>
      {:else}
        {#each versions as version}
          {@const isSelected = selectedVersions.includes(version.version_number)}

          <div
            class="border rounded p-3 bg-white dark:bg-gray-800 text-sm"
            class:ring-2={isSelected}
            class:ring-blue-500={isSelected}
          >
            <div class="flex items-center justify-between mb-1">
              <div class="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={isSelected}
                  onchange={() => toggleVersionSelection(version.version_number)}
                  class="rounded"
                />
                <span class="font-semibold">v{version.version_number}</span>
                {#if version.version_number === versions[0].version_number}
                  <span class="text-xs bg-blue-100 text-blue-800 px-2 py-0.5 rounded">
                    Current
                  </span>
                {/if}
              </div>
              <span class="text-xs text-gray-500">
                {new Date(version.created_at).toLocaleString()}
              </span>
            </div>

            <div class="flex items-center gap-2 text-xs text-gray-600 mb-2">
              <span>{getSourceIcon(version.source)}</span>
              <span>{getSourceLabel(version.source)}</span>
            </div>

            {#if version.summary}
              <p class="text-xs text-gray-700 dark:text-gray-300 mb-2">
                {version.summary}
              </p>
            {/if}

            {#if version.version_number > 1 && oncompare}
              <button
                onclick={() => compareWithPrevious(version)}
                class="text-xs text-blue-600 hover:underline"
              >
                Compare with v{version.version_number - 1}
              </button>
            {/if}
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</div>
```

---

### 4. Update API Client - `frontend/src/lib/api/plans.ts`

```typescript
export const plansApi = {
  // ... existing methods ...

  async getPlanVersion(planId: string, versionNumber: number): Promise<PlanVersion> {
    const response = await fetch(`${API_URL}/api/plans/${planId}/versions/${versionNumber}`);
    if (!response.ok) throw new Error('Failed to fetch version');
    return response.json();
  },

  async getVersionDiff(
    planId: string,
    fromVersion: number,
    toVersion: number
  ): Promise<{
    from_version: PlanVersion;
    to_version: PlanVersion;
    stats: {
      lines_added: number;
      lines_removed: number;
      from_lines: number;
      to_lines: number;
    };
  }> {
    const response = await fetch(
      `${API_URL}/api/plans/${planId}/diff/${fromVersion}/${toVersion}`
    );
    if (!response.ok) throw new Error('Failed to fetch diff');
    return response.json();
  },
};
```

---

### 5. Create Diff View Page - `frontend/src/routes/plans/[id]/diff/+page.svelte`

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { plansApi, type PlanVersion } from '$lib/api/plans';
  import DiffViewer from '$lib/components/DiffViewer.svelte';

  const planId = $derived($page.params.id);
  const fromVersionNum = $derived(parseInt($page.url.searchParams.get('from') || '1'));
  const toVersionNum = $derived(parseInt($page.url.searchParams.get('to') || '2'));

  let fromVersion = $state<PlanVersion | null>(null);
  let toVersion = $state<PlanVersion | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    await loadVersions();
  });

  async function loadVersions() {
    loading = true;
    error = null;

    try {
      const [from, to] = await Promise.all([
        plansApi.getPlanVersion(planId, fromVersionNum),
        plansApi.getPlanVersion(planId, toVersionNum),
      ]);

      fromVersion = from;
      toVersion = to;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load versions';
    } finally {
      loading = false;
    }
  }
</script>

<div class="container mx-auto px-4 py-8">
  <div class="mb-6">
    <button
      onclick={() => goto(`/plans/${planId}`)}
      class="text-blue-600 hover:underline text-sm mb-2"
    >
      ← Back to plan
    </button>
    <h1 class="text-3xl font-bold">Plan Diff</h1>
  </div>

  {#if loading}
    <p>Loading diff...</p>
  {:else if error}
    <p class="text-red-600">{error}</p>
  {:else if fromVersion && toVersion}
    <DiffViewer {fromVersion} {toVersion} />
  {/if}
</div>
```

---

### 6. Update Plan Viewer - `frontend/src/routes/plans/[id]/+page.svelte`

Add compare handler:

```svelte
<script lang="ts">
  // ... existing imports ...
  import { goto } from '$app/navigation';

  // ... existing code ...

  function handleCompareVersions(from: PlanVersion, to: PlanVersion) {
    goto(`/plans/${planId}/diff?from=${from.version_number}&to=${to.version_number}`);
  }
</script>

<!-- Update VersionHistory component -->
<VersionHistory planId={data.plan.id} oncompare={handleCompareVersions} />
```

---

## Testing Checklist

- [ ] View diff between two versions
- [ ] Toggle between unified and split view
- [ ] Stats show correct added/removed lines
- [ ] Download diff as text file
- [ ] Compare with previous version from history
- [ ] Select two versions and compare
- [ ] Version metadata displayed correctly
- [ ] Diff rendering works with large files
- [ ] Mobile responsive diff viewer
- [ ] Dark mode works for diff colors

---

## Implementation Time

- Backend: 0.5 days
- Frontend: 2 days
- Testing: 0.5 days
- **Total: 3 days**

---

## Next Extension

👉 **Extension 05 - Collaboration Features**

See: `05-collaboration-features.md`
