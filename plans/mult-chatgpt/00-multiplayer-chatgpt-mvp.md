# Multiplayer ChatGPT - MVP (Base System)

## Overview

A collaborative plan review system where users can:
1. Upload markdown engineering plans
2. Comment on specific lines
3. Accept comments to trigger AI integration (Anthropic)
4. Track versions of plan evolution

**Status:** Base implementation
**Next:** Extension 01 - Discussions

---

## Core Features

- ✅ Upload markdown plans (max 1MB)
- ✅ Line-based commenting (with shift-click selection)
- ✅ Accept/reject comments (plan owner only)
- ✅ Async AI integration with job queue
- ✅ Job status polling with UI feedback
- ✅ Rate limiting (10 AI requests/hour)
- ✅ Version tracking
- ✅ Public plan listing
- ✅ Download plans

---

## Database Schema

### Tables
1. `plans` - Plan storage
2. `plan_versions` - Version history
3. `plan_comments` - Line-based comments
4. `ai_integration_jobs` - Async job queue
5. `api_rate_limits` - Rate limiting

### Key Design Decisions
- Line-based anchoring (not character offsets)
- Async AI processing (not blocking HTTP)
- Boolean `is_resolved` for comments
- Transaction safety with rollback
- 1MB max file size

---

## API Endpoints

**Public:**
- `GET /api/plans` - List public plans
- `GET /api/plans/:id` - Get plan with comments

**Authenticated:**
- `POST /api/plans` - Upload plan
- `POST /api/plans/:id/comments` - Create comment
- `POST /api/comments/:id/accept` - Accept (owner only)
- `POST /api/comments/:id/reject` - Reject (owner only)
- `GET /api/ai-jobs/:id` - Check job status

---

## Implementation Time

- Backend: 3-4 days
- Frontend: 3-4 days
- Testing: 2-3 days
- **Total: 10-13 days**

---

## Next Extension

👉 **Extension 01 - Discussions**: Add threaded discussions on comments

See: `01-discussions.md`
