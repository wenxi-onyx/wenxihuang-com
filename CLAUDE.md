# wenxihuang.com

Personal site: SvelteKit frontend + TypeScript API, one Fly.io machine, SQLite
on a volume. The previous Rust/axum + Fly Postgres backend and the multiplayer
ChatGPT feature were removed in the `ts-monorepo` rewrite (June 2026); the
old code lives in git history before that branch.

## Layout

- `apps/api` — Fastify 5 + better-sqlite3. Ping pong (table tennis) API:
  auth/sessions, players, matches, ELO (+ configurable algorithms), seasons,
  background jobs. Schema in `src/schema.sql`; applied automatically on boot.
- `apps/web` — SvelteKit (adapter-node). Talks to the API same-origin at
  `/api/...`. In production the API process serves the built web app itself
  (see `apps/api/src/index.ts`); there is no separate frontend deployment.

## Dev

```bash
pnpm install
pnpm --filter api dev    # Fastify on :8080, SQLite at apps/api/data/dev.db
pnpm --filter web dev    # Vite on :5173, proxies /api -> :8080
```

First boot creates an `admin` user (password from `ADMIN_PASSWORD`, default
`admin`). Existing argon2 password hashes from the old backend verify as-is.

## Deploy (Fly.io)

Single app `wenxihuang-frontend` (kept for the wenxihuang.com cert/DNS), one
always-on machine, volume `data` mounted at `/data`, SQLite at
`/data/site.db`. `fly deploy` from the repo root. The machine must NOT
auto-stop (SSE for silicon-sanfrancisco; also why the old "Fly wipes my
SQLite" problem is gone — the volume persists across deploys and Fly snapshots
it daily).

Data was migrated from the old Fly Postgres (`wenxihuang-db`) with
`apps/api/scripts/migrate-from-postgres.ts`; JSON dumps land in
`apps/api/backup/`. `wenxihuang-db` and `wenxihuang-backend` can be destroyed
once the site is verified (manual step, never automated).

## Hosting silicon-sanfrancisco (the contract)

silicon-sanfrancisco (separate repo, TS monorepo: Fastify + better-sqlite3 +
React) deploys onto this same machine with zero coupling to this codebase:

- **Process model**: SSF runs as its own Node process on an internal port
  (e.g. 3001), started alongside the API by a small supervisor script (swap
  the Docker CMD for it when SSF lands). No shared framework versions, no
  shared node_modules — its build is COPY'd into the image as a self-contained
  directory.
- **Routing**: the API process proxies requests for `ssf.wenxihuang.com`
  (Host-header match, hook point marked in `apps/api/src/index.ts`) to the SSF
  port. Add the cert with `fly certs add ssf.wenxihuang.com` and a CNAME.
- **Auth is separate on purpose**: SSF's plan (its `docs/PLAN.md` §2)
  specifies its own minimal auth — username/password (argon2) + session
  cookie + invite-code registration, own `users`/`sessions` tables in its own
  SQLite db. It does NOT reuse this site's accounts. Sharing auth would mean
  SSF depending on this repo's session schema/API — exactly the coupling this
  contract forbids — to save a handful of friends one one-time registration.
  The separate hostname keeps the two session cookies from ever colliding.
  (If shared accounts are ever truly wanted, the non-coupling way is an
  SSO-style handoff: this site issues a short-lived signed token that SSF
  verifies with a shared secret — SSF still owns its own sessions.)
- **Storage**: SSF owns `/data/ssf/` on the volume (its SQLite db, backups).
  This repo's API only ever touches `/data/site.db`.
- **Resources**: bump [[vm]] memory in fly.toml if needed when SSF lands.

Nothing in this repo may constrain SSF's stack choices; if a conflict appears,
this repo adapts, not SSF.
