/**
 * One-shot data migration: Fly Postgres (wenxihuang-db) -> SQLite.
 *
 * Usage:
 *   1. In another terminal:  fly proxy 15432:5432 -a wenxihuang-db
 *   2. PG_URL='postgres://postgres:<password>@localhost:15432/postgres' \
 *      SQLITE_PATH=./data/site.db \
 *      pnpm --filter api migrate-from-postgres
 *
 * - Writes a full JSON dump of every migrated table to ./backup/<table>.json first.
 * - Refuses to run against a SQLite db that already has players (pass --force to wipe).
 * - Verifies row counts per table and exits non-zero on mismatch.
 *
 * Deliberately NOT migrated (multiplayer-chatgpt removal): plans, plan_versions,
 * plan_comments, user_api_keys, and any presence/rate-limit tables.
 * Sessions are also skipped — users just log in again.
 */
import pg from 'pg';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { openDb } from '../src/db.js';

const here = dirname(fileURLToPath(import.meta.url));

const PG_URL = process.env.PG_URL ?? process.env.DATABASE_URL;
const SQLITE_PATH = process.env.SQLITE_PATH ?? join(here, '..', 'data', 'site.db');
const FORCE = process.argv.includes('--force');

if (!PG_URL) {
  console.error('Set PG_URL (or DATABASE_URL) to the Postgres connection string.');
  process.exit(1);
}

const iso = (v: unknown): string | null => (v == null ? null : new Date(v as string | Date).toISOString());
const bool = (v: unknown): number | null => (v == null ? null : v ? 1 : 0);
const json = (v: unknown): string | null => (v == null ? null : JSON.stringify(v));

// table -> [columns in SQLite order, row converter]
type Converter = (r: Record<string, unknown>) => unknown[];
const TABLES: Array<{ name: string; columns: string[]; convert: Converter }> = [
  {
    name: 'users',
    columns: ['id', 'username', 'password_hash', 'role', 'first_name', 'last_name', 'created_at'],
    convert: (r) => [r.id, r.username, r.password_hash, r.role, r.first_name, r.last_name, iso(r.created_at)],
  },
  {
    name: 'elo_configurations',
    columns: [
      'id', 'version_name', 'k_factor', 'base_k_factor', 'new_player_k_bonus',
      'new_player_bonus_period', 'starting_elo', 'description', 'is_active', 'created_at', 'created_by',
    ],
    convert: (r) => [
      r.id, r.version_name, r.k_factor, r.base_k_factor, r.new_player_k_bonus,
      r.new_player_bonus_period, r.starting_elo, r.description, bool(r.is_active), iso(r.created_at), r.created_by,
    ],
  },
  {
    name: 'seasons',
    columns: [
      'id', 'name', 'description', 'start_date', 'starting_elo', 'k_factor', 'base_k_factor',
      'new_player_k_bonus', 'new_player_bonus_period', 'elo_version', 'is_active', 'created_at', 'created_by',
    ],
    convert: (r) => [
      r.id, r.name, r.description, iso(r.start_date), r.starting_elo, r.k_factor, r.base_k_factor,
      r.new_player_k_bonus, r.new_player_bonus_period, r.elo_version, bool(r.is_active), iso(r.created_at), r.created_by,
    ],
  },
  {
    name: 'players',
    columns: ['id', 'first_name', 'last_name', 'current_elo', 'is_active', 'profile_pic', 'created_at', 'updated_at'],
    convert: (r) => [
      r.id, r.first_name, r.last_name, r.current_elo, bool(r.is_active), r.profile_pic ?? null,
      iso(r.created_at), iso(r.updated_at),
    ],
  },
  {
    name: 'player_seasons',
    columns: [
      'id', 'player_id', 'season_id', 'current_elo', 'games_played', 'wins', 'losses',
      'is_included', 'created_at', 'updated_at',
    ],
    convert: (r) => [
      r.id, r.player_id, r.season_id, r.current_elo, r.games_played, r.wins, r.losses,
      bool(r.is_included), iso(r.created_at), iso(r.updated_at),
    ],
  },
  {
    name: 'matches',
    columns: ['id', 'player1_id', 'player2_id', 'season_id', 'submitted_at', 'created_at', 'updated_at'],
    convert: (r) => [
      r.id, r.player1_id, r.player2_id, r.season_id, iso(r.submitted_at), iso(r.created_at), iso(r.updated_at),
    ],
  },
  {
    name: 'games',
    columns: ['id', 'match_id', 'player1_id', 'player2_id', 'season_id', 'elo_version', 'played_at'],
    convert: (r) => [r.id, r.match_id, r.player1_id, r.player2_id, r.season_id, r.elo_version, iso(r.played_at)],
  },
  {
    name: 'elo_history',
    columns: ['id', 'player_id', 'game_id', 'elo_before', 'elo_after', 'elo_version', 'season_id', 'created_at'],
    convert: (r) => [
      r.id, r.player_id, r.game_id, r.elo_before, r.elo_after, r.elo_version, r.season_id, iso(r.created_at),
    ],
  },
  {
    name: 'jobs',
    columns: [
      'id', 'job_type', 'status', 'progress', 'total_items', 'processed_items',
      'result_data', 'created_by', 'created_at', 'started_at', 'completed_at',
    ],
    convert: (r) => [
      r.id, r.job_type, r.status, r.progress, r.total_items, r.processed_items,
      json(r.result_data), r.created_by, iso(r.created_at), iso(r.started_at), iso(r.completed_at),
    ],
  },
];

async function main(): Promise<void> {
  const client = new pg.Client({ connectionString: PG_URL });
  await client.connect();
  console.log('Connected to Postgres');

  const db = openDb(SQLITE_PATH);
  console.log(`SQLite at ${SQLITE_PATH}`);

  const existing = db.prepare('SELECT COUNT(*) AS n FROM players').get() as { n: number };
  if (existing.n > 0 && !FORCE) {
    console.error('Target SQLite already contains player data. Re-run with --force to wipe and re-import.');
    process.exit(1);
  }

  const backupDir = join(here, '..', 'backup');
  mkdirSync(backupDir, { recursive: true });

  // Wipe children before parents so FK constraints hold, then import parents first.
  db.transaction(() => {
    db.prepare('DELETE FROM sessions').run();
    for (const table of [...TABLES].reverse()) {
      db.prepare(`DELETE FROM ${table.name}`).run();
    }
  })();

  let failed = false;

  for (const table of TABLES) {
    const { rows } = await client.query(`SELECT * FROM ${table.name}`);
    writeFileSync(join(backupDir, `${table.name}.json`), JSON.stringify(rows, null, 2));

    const placeholders = table.columns.map(() => '?').join(', ');
    const insert = db.prepare(
      `INSERT INTO ${table.name} (${table.columns.join(', ')}) VALUES (${placeholders})`
    );

    db.transaction(() => {
      for (const row of rows) {
        insert.run(...table.convert(row));
      }
    })();

    const { n } = db.prepare(`SELECT COUNT(*) AS n FROM ${table.name}`).get() as { n: number };
    const ok = n === rows.length;
    if (!ok) failed = true;
    console.log(`${ok ? '✓' : '✗'} ${table.name}: postgres=${rows.length} sqlite=${n}`);
  }

  await client.end();

  if (failed) {
    console.error('Row count mismatch — investigate before using this database.');
    process.exit(1);
  }
  console.log(`\nDone. JSON backups in ${backupDir}. Keep them until you have verified the site.`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
