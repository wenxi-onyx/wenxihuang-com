# Extension 07 - Search & Auto-sync

**Builds on:** Extension 06 - Private Plans
**Status:** Final Extension

---

## What This Adds

Full-text search and automatic syncing from local filesystem.

**New Features:**
- Full-text search across all public plans
- Filter by author, date, tags
- Highlight search matches
- CLI tool for auto-syncing local markdown files
- File watcher for automatic uploads
- Sync conflict resolution

---

## Part A: Search Implementation

### Database Changes

#### Migration: `backend/migrations/011_add_search.sql`

```sql
-- Add full-text search support
ALTER TABLE plans
ADD COLUMN search_vector tsvector;

-- Create full-text index
CREATE INDEX idx_plans_search_vector ON plans USING gin(search_vector);

-- Function to update search vector
CREATE OR REPLACE FUNCTION update_plan_search_vector()
RETURNS TRIGGER AS $$
BEGIN
    NEW.search_vector =
        setweight(to_tsvector('english', coalesce(NEW.title, '')), 'A') ||
        setweight(to_tsvector('english', coalesce(NEW.filename, '')), 'B') ||
        setweight(to_tsvector('english', coalesce(NEW.content, '')), 'C');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to auto-update search vector
CREATE TRIGGER trigger_update_plan_search_vector
BEFORE INSERT OR UPDATE ON plans
FOR EACH ROW
EXECUTE FUNCTION update_plan_search_vector();

-- Update existing plans
UPDATE plans SET updated_at = updated_at;  -- Triggers search vector update

-- Add tags support (optional)
CREATE TABLE plan_tags (
    plan_id UUID NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    tag VARCHAR(50) NOT NULL,
    PRIMARY KEY (plan_id, tag)
);

CREATE INDEX idx_plan_tags_tag ON plan_tags(tag);
```

---

### Backend Implementation

#### Update Plan Handlers - `backend/src/handlers/plans.rs`

```rust
// Search plans
pub async fn search_plans(
    State(pool): State<PgPool>,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchResponse>, (StatusCode, String)> {
    let query = params.q.unwrap_or_default();
    let author = params.author;
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    if query.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Query parameter required".to_string()));
    }

    // Build search query
    let mut sql = String::from(
        r#"
        SELECT
            p.id, p.user_id, p.title, p.filename, p.content, p.is_public,
            p.created_at, p.updated_at,
            u.username as author_username,
            ts_rank(p.search_vector, query) as rank,
            ts_headline('english', p.content, query, 'MaxWords=50, MinWords=25') as highlight
        FROM plans p
        JOIN users u ON p.user_id = u.id,
        websearch_to_tsquery('english', $1) query
        WHERE p.is_public = true
        AND p.search_vector @@ query
        "#,
    );

    // Add author filter if provided
    if author.is_some() {
        sql.push_str(" AND u.username = $2");
    }

    sql.push_str(" ORDER BY rank DESC, p.created_at DESC");
    sql.push_str(" LIMIT $");
    sql.push_str(&if author.is_some() { "3" } else { "2" }.to_string());
    sql.push_str(" OFFSET $");
    sql.push_str(&if author.is_some() { "4" } else { "3" }.to_string());

    // Execute query
    let results = if let Some(author_val) = author {
        sqlx::query(&sql)
            .bind(&query)
            .bind(author_val)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&pool)
            .await
    } else {
        sqlx::query(&sql)
            .bind(&query)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&pool)
            .await
    }
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Map results
    let plans: Vec<SearchResult> = results
        .into_iter()
        .map(|row| SearchResult {
            id: row.get("id"),
            title: row.get("title"),
            filename: row.get("filename"),
            author_username: row.get("author_username"),
            highlight: row.get("highlight"),
            created_at: row.get("created_at"),
        })
        .collect();

    Ok(Json(SearchResponse {
        query: query.clone(),
        total: plans.len(),
        results: plans,
    }))
}

// DTOs
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
    pub author: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub id: Uuid,
    pub title: String,
    pub filename: String,
    pub author_username: String,
    pub highlight: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub total: usize,
    pub results: Vec<SearchResult>,
}

// Add tags to plan
pub async fn add_plan_tags(
    user: User,
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
    Json(tags): Json<Vec<String>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    // Verify ownership
    let is_owner = sqlx::query_scalar!(
        "SELECT user_id = $1 FROM plans WHERE id = $2",
        user.id,
        plan_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| (StatusCode::NOT_FOUND, "Plan not found".to_string()))?
    .unwrap_or(false);

    if !is_owner {
        return Err((StatusCode::FORBIDDEN, "Only owner can add tags".to_string()));
    }

    // Add tags
    for tag in tags {
        let tag_clean = tag.trim().to_lowercase();
        if tag_clean.is_empty() || tag_clean.len() > 50 {
            continue;
        }

        sqlx::query!(
            "INSERT INTO plan_tags (plan_id, tag) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            plan_id,
            tag_clean
        )
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(Json(json!({ "message": "Tags added" })))
}

// Get plan tags
pub async fn get_plan_tags(
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    let tags = sqlx::query_scalar!("SELECT tag FROM plan_tags WHERE plan_id = $1", plan_id)
        .fetch_all(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(tags))
}
```

---

#### Update Routes - `backend/src/main.rs`

```rust
// Public search
.route("/api/plans/search", get(plans::search_plans))

// Tags
.route("/api/plans/:plan_id/tags", get(plans::get_plan_tags))
.route("/api/plans/:plan_id/tags", post(plans::add_plan_tags)
    .layer(from_fn_with_state(pool.clone(), require_auth)))
```

---

### Frontend Implementation

#### Update API Client - `frontend/src/lib/api/plans.ts`

```typescript
export interface SearchResult {
  id: string;
  title: string;
  filename: string;
  author_username: string;
  highlight: string;
  created_at: string;
}

export interface SearchResponse {
  query: string;
  total: number;
  results: SearchResult[];
}

export const plansApi = {
  // ... existing methods ...

  async searchPlans(
    query: string,
    options?: { author?: string; limit?: number; offset?: number }
  ): Promise<SearchResponse> {
    const params = new URLSearchParams({ q: query });
    if (options?.author) params.append('author', options.author);
    if (options?.limit) params.append('limit', options.limit.toString());
    if (options?.offset) params.append('offset', options.offset.toString());

    const response = await fetch(`${API_URL}/api/plans/search?${params}`);
    if (!response.ok) throw new Error('Search failed');
    return response.json();
  },

  async getPlanTags(planId: string): Promise<string[]> {
    const response = await fetch(`${API_URL}/api/plans/${planId}/tags`);
    if (!response.ok) throw new Error('Failed to get tags');
    return response.json();
  },

  async addPlanTags(planId: string, tags: string[]): Promise<void> {
    const response = await fetch(`${API_URL}/api/plans/${planId}/tags`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify(tags),
    });
    if (!response.ok) throw new Error('Failed to add tags');
  },
};
```

---

#### Create Search Page - `frontend/src/routes/search/+page.svelte`

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { plansApi, type SearchResponse } from '$lib/api/plans';

  let query = $state('');
  let results = $state<SearchResponse | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  onMount(() => {
    const q = $page.url.searchParams.get('q');
    if (q) {
      query = q;
      performSearch();
    }
  });

  async function performSearch() {
    if (!query.trim()) return;

    loading = true;
    error = null;

    // Update URL
    goto(`/search?q=${encodeURIComponent(query)}`, { replaceState: true });

    try {
      results = await plansApi.searchPlans(query);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Search failed';
    } finally {
      loading = false;
    }
  }

  function handleSubmit(e: Event) {
    e.preventDefault();
    performSearch();
  }
</script>

<div class="container mx-auto px-4 py-8 max-w-4xl">
  <h1 class="text-3xl font-bold mb-6">Search Plans</h1>

  <form onsubmit={handleSubmit} class="mb-8">
    <div class="flex gap-2">
      <input
        type="text"
        bind:value={query}
        placeholder="Search plans..."
        class="flex-1 border rounded px-4 py-3 text-lg"
      />
      <button
        type="submit"
        disabled={loading}
        class="bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white px-6 py-3 rounded font-medium"
      >
        {loading ? 'Searching...' : 'Search'}
      </button>
    </div>
  </form>

  {#if error}
    <p class="text-red-600">{error}</p>
  {:else if results}
    <div class="mb-4">
      <p class="text-gray-600">
        Found {results.total} result{results.total !== 1 ? 's' : ''} for "{results.query}"
      </p>
    </div>

    <div class="space-y-4">
      {#each results.results as result}
        <a
          href="/plans/{result.id}"
          class="block border rounded p-4 hover:bg-gray-50 dark:hover:bg-gray-800 transition"
        >
          <h2 class="text-xl font-semibold mb-1">{result.title}</h2>
          <p class="text-sm text-gray-600 mb-2">
            by @{result.author_username} • {result.filename} •
            {new Date(result.created_at).toLocaleDateString()}
          </p>
          <div class="text-sm text-gray-700 dark:text-gray-300">
            {@html result.highlight}...
          </div>
        </a>
      {/each}
    </div>
  {/if}
</div>
```

---

#### Add Search to Navigation - `frontend/src/routes/+layout.svelte`

```svelte
<nav>
  <a href="/">Home</a>
  <a href="/plans">Plans</a>
  <a href="/search">Search</a>
  <!-- ... rest of nav ... -->
</nav>
```

---

## Part B: CLI Auto-sync Tool

### Create CLI Tool - `cli/sync.js`

```javascript
#!/usr/bin/env node

const fs = require('fs').promises;
const path = require('path');
const chokidar = require('chokidar');
const fetch = require('node-fetch');
const readline = require('readline');

const CONFIG_FILE = path.join(process.env.HOME, '.plan-sync-config.json');

class PlanSync {
  constructor() {
    this.config = null;
    this.watcher = null;
  }

  async loadConfig() {
    try {
      const data = await fs.readFile(CONFIG_FILE, 'utf8');
      this.config = JSON.parse(data);
    } catch (e) {
      console.log('No config found. Run setup first.');
      return false;
    }
    return true;
  }

  async saveConfig(config) {
    await fs.writeFile(CONFIG_FILE, JSON.stringify(config, null, 2));
    this.config = config;
  }

  async setup() {
    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout,
    });

    const question = (q) => new Promise((resolve) => rl.question(q, resolve));

    console.log('Plan Sync Setup\n');

    const apiUrl = await question('API URL (default: http://localhost:8080): ');
    const watchDir = await question('Directory to watch (default: ./plans): ');
    const email = await question('Your email: ');
    const password = await question('Your password: ');

    rl.close();

    // Login to get session cookie
    const loginResponse = await fetch(`${apiUrl || 'http://localhost:8080'}/api/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email, password }),
    });

    if (!loginResponse.ok) {
      console.error('Login failed');
      process.exit(1);
    }

    const cookies = loginResponse.headers.get('set-cookie');

    const config = {
      apiUrl: apiUrl || 'http://localhost:8080',
      watchDir: watchDir || './plans',
      cookies,
    };

    await this.saveConfig(config);
    console.log('Setup complete! Config saved to', CONFIG_FILE);
  }

  async uploadFile(filePath) {
    const content = await fs.readFile(filePath, 'utf8');
    const filename = path.basename(filePath);
    const title = filename.replace('.md', '').replace(/[-_]/g, ' ');

    console.log(`Uploading ${filename}...`);

    const response = await fetch(`${this.config.apiUrl}/api/plans`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Cookie: this.config.cookies,
      },
      body: JSON.stringify({ title, filename, content }),
    });

    if (response.ok) {
      const plan = await response.json();
      console.log(`✓ Uploaded: ${plan.title} (${plan.id})`);
      return plan;
    } else {
      const error = await response.text();
      console.error(`✗ Failed to upload ${filename}: ${error}`);
      return null;
    }
  }

  async watch() {
    if (!await this.loadConfig()) {
      console.log('Run: plan-sync setup');
      return;
    }

    console.log(`Watching ${this.config.watchDir} for changes...`);

    this.watcher = chokidar.watch(`${this.config.watchDir}/**/*.md`, {
      persistent: true,
      ignoreInitial: false,
    });

    this.watcher
      .on('add', (filePath) => this.uploadFile(filePath))
      .on('change', (filePath) => this.uploadFile(filePath));

    console.log('Press Ctrl+C to stop watching');

    process.on('SIGINT', () => {
      console.log('\nStopping watcher...');
      this.watcher.close();
      process.exit(0);
    });
  }

  async uploadOnce() {
    if (!await this.loadConfig()) {
      console.log('Run: plan-sync setup');
      return;
    }

    const files = await this.findMarkdownFiles(this.config.watchDir);
    console.log(`Found ${files.length} markdown files`);

    for (const file of files) {
      await this.uploadFile(file);
    }
  }

  async findMarkdownFiles(dir) {
    const files = [];
    const entries = await fs.readdir(dir, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        files.push(...(await this.findMarkdownFiles(fullPath)));
      } else if (entry.name.endsWith('.md')) {
        files.push(fullPath);
      }
    }

    return files;
  }
}

// CLI
const sync = new PlanSync();
const command = process.argv[2];

switch (command) {
  case 'setup':
    sync.setup();
    break;
  case 'watch':
    sync.watch();
    break;
  case 'upload':
    sync.uploadOnce();
    break;
  default:
    console.log(`
Plan Sync CLI

Commands:
  setup   - Configure API credentials and watch directory
  watch   - Watch directory for changes and auto-upload
  upload  - Upload all markdown files once

Usage:
  plan-sync setup
  plan-sync watch
  plan-sync upload
    `);
}
```

---

### Package CLI Tool - `cli/package.json`

```json
{
  "name": "plan-sync-cli",
  "version": "1.0.0",
  "description": "CLI tool to sync markdown files to Plan platform",
  "main": "sync.js",
  "bin": {
    "plan-sync": "./sync.js"
  },
  "dependencies": {
    "chokidar": "^3.5.3",
    "node-fetch": "^2.6.7"
  },
  "keywords": ["markdown", "sync", "plans"],
  "author": "Your Name",
  "license": "MIT"
}
```

---

### Install CLI Globally

```bash
cd cli
npm install
npm link  # Makes plan-sync available globally
```

---

### Usage

```bash
# Setup
plan-sync setup

# One-time upload
plan-sync upload

# Watch mode
plan-sync watch
```

---

## Testing Checklist

### Search:
- [ ] Search by keyword in title
- [ ] Search by keyword in content
- [ ] Search by author
- [ ] Search highlights relevant text
- [ ] Pagination works
- [ ] No results message displays correctly
- [ ] Tags can be added to plans
- [ ] Tags filterable

### CLI Sync:
- [ ] Setup saves config correctly
- [ ] Upload single file works
- [ ] Upload all files works
- [ ] Watch mode detects new files
- [ ] Watch mode detects file changes
- [ ] Authentication persists across uploads
- [ ] Large files upload correctly
- [ ] Error handling works

---

## Implementation Time

### Search:
- Backend: 1 day
- Frontend: 1 day
- **Subtotal: 2 days**

### CLI Tool:
- Development: 1 day
- Testing: 0.5 days
- **Subtotal: 1.5 days**

**Total: 3.5 days**

---

## Complete System Summary

With all 7 extensions implemented, you have:

✅ **MVP (Base)** - 10-13 days
- Upload & comment on plans
- AI integration
- Version tracking

✅ **Extension 01** - 3 days
- Threaded discussions

✅ **Extension 02** - 4 days
- Enhanced status tracking
- Version metadata

✅ **Extension 03** - 3 days
- Inline highlighting
- Markdown rendering

✅ **Extension 04** - 3 days
- Version diffs
- Change visualization

✅ **Extension 05** - 5 days
- @mentions
- Notifications
- Subscriptions

✅ **Extension 06** - 4 days
- Private plans
- Share links
- Access control

✅ **Extension 07** - 3.5 days
- Full-text search
- CLI auto-sync

**Total Implementation Time: 35.5-38.5 days**

**Complete Feature Set:**
- Plan upload & management
- Line-based commenting
- Threaded discussions
- AI-powered integration
- Version history & diffs
- Real-time notifications
- @mentions
- Private plans & sharing
- Access control
- Full-text search
- Auto-sync from filesystem
- Markdown rendering
- Inline highlighting
- Email notifications
- Rate limiting
- Comprehensive access logs

🎉 **You now have a complete multiplayer collaborative plan review system!**
