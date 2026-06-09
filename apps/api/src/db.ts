import Database from 'better-sqlite3';
import { readFileSync } from 'node:fs';
import { mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { nowIso, uuid } from './util.js';

export type DB = Database.Database;

const here = dirname(fileURLToPath(import.meta.url));

export function openDb(path: string): DB {
  mkdirSync(dirname(path), { recursive: true });
  const db = new Database(path);
  db.pragma('journal_mode = WAL');
  db.pragma('foreign_keys = ON');
  db.pragma('busy_timeout = 5000');
  migrate(db);
  return db;
}

function migrate(db: DB): void {
  const version = db.pragma('user_version', { simple: true }) as number;
  if (version >= 1) return;

  const schema = readFileSync(join(here, 'schema.sql'), 'utf8');
  db.transaction(() => {
    db.exec(schema);
    seedEloConfigurations(db);
    db.pragma('user_version = 1');
  })();
}

/** Default v1/v2 ELO configurations, matching the original migration 003. */
function seedEloConfigurations(db: DB): void {
  const insert = db.prepare(
    `INSERT OR IGNORE INTO elo_configurations
       (id, version_name, k_factor, base_k_factor, new_player_k_bonus,
        new_player_bonus_period, starting_elo, description, is_active, created_at)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
  );
  insert.run(uuid(), 'v1', 32.0, 32.0, 0, 0, 1000.0, 'Standard ELO with K=32, starting at 1000', 1, nowIso());
  insert.run(
    uuid(),
    'v2',
    20.0,
    20.0,
    48.0,
    10,
    1000.0,
    'Dynamic K-factor: Base K=20, New Player Bonus=48 over 10 games, Starting ELO=1000',
    0,
    nowIso()
  );
}
