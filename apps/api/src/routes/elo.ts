import type { FastifyInstance } from 'fastify';
import type { DB } from '../db.js';
import { makeAuthHooks } from '../auth.js';
import { calculateMatchEloChanges, dynamicKFactor, type EloConfig } from '../elo.js';
import { invalidInput } from '../errors.js';
import { createJob, getJob, markJobFinished, markJobRunning, updateJobProgressItems } from '../jobs.js';
import { asBool, nowIso, uuid } from '../util.js';

const MAX_VERSION_NAME_LENGTH = 50;
const MIN_K_FACTOR = 1.0;
const MAX_K_FACTOR = 100.0;
const MIN_STARTING_ELO = 100.0;
const MAX_STARTING_ELO = 3000.0;
const MAX_DESCRIPTION_LENGTH = 500;

interface EloConfigBody {
  version_name?: string;
  k_factor: number;
  starting_elo: number;
  base_k_factor?: number | null;
  new_player_k_bonus?: number | null;
  new_player_bonus_period?: number | null;
  description?: string | null;
}

interface EloConfigRow {
  id: string;
  version_name: string;
  k_factor: number;
  starting_elo: number;
  base_k_factor: number | null;
  new_player_k_bonus: number | null;
  new_player_bonus_period: number | null;
  description: string | null;
  is_active: number;
  created_at: string;
  created_by: string | null;
}

const mapConfig = (row: EloConfigRow) => ({
  id: row.id,
  version_name: row.version_name,
  k_factor: row.k_factor,
  starting_elo: row.starting_elo,
  base_k_factor: row.base_k_factor,
  new_player_k_bonus: row.new_player_k_bonus,
  new_player_bonus_period: row.new_player_bonus_period,
  description: row.description,
  is_active: asBool(row.is_active),
  created_at: row.created_at,
});

function validateConfigBody(body: EloConfigBody): void {
  if (body.k_factor < MIN_K_FACTOR || body.k_factor > MAX_K_FACTOR) {
    throw invalidInput(`K-factor must be between ${MIN_K_FACTOR} and ${MAX_K_FACTOR}`);
  }
  if (body.starting_elo < MIN_STARTING_ELO || body.starting_elo > MAX_STARTING_ELO) {
    throw invalidInput(`Starting ELO must be between ${MIN_STARTING_ELO} and ${MAX_STARTING_ELO}`);
  }
  if (body.description != null && body.description.length > MAX_DESCRIPTION_LENGTH) {
    throw invalidInput(`Description must be ${MAX_DESCRIPTION_LENGTH} characters or less`);
  }
  if (body.base_k_factor != null && (body.base_k_factor < MIN_K_FACTOR || body.base_k_factor > MAX_K_FACTOR)) {
    throw invalidInput(`Base K-factor must be between ${MIN_K_FACTOR} and ${MAX_K_FACTOR}`);
  }
  if (body.new_player_k_bonus != null && (body.new_player_k_bonus < 0 || body.new_player_k_bonus > MAX_K_FACTOR)) {
    throw invalidInput(`New player K bonus must be between 0 and ${MAX_K_FACTOR}`);
  }
  if (body.new_player_bonus_period != null && body.new_player_bonus_period <= 0) {
    throw invalidInput('New player bonus period must be positive');
  }
}

/**
 * Global ELO recalculation across ALL games (chronological), tagged with one
 * configuration version. Ported from services/elo.rs::recalculate_all_elo.
 */
function recalculateAllElo(db: DB, config: EloConfig, jobId: string): void {
  const players = db.prepare('SELECT id FROM players').all() as Array<{ id: string }>;
  const playerElos = new Map<string, number>();
  const gamesPlayed = new Map<string, number>();
  for (const { id } of players) {
    playerElos.set(id, config.starting_elo);
    gamesPlayed.set(id, 0);
  }

  const games = db
    .prepare('SELECT id, player1_id, player2_id, season_id, played_at FROM games ORDER BY played_at ASC')
    .all() as Array<{ id: string; player1_id: string; player2_id: string; season_id: string; played_at: string }>;

  const insertHistory = db.prepare(
    `INSERT INTO elo_history (id, player_id, game_id, elo_before, elo_after, elo_version, season_id, created_at)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`
  );
  const updateGameVersion = db.prepare('UPDATE games SET elo_version = ? WHERE id = ?');

  db.transaction(() => {
    db.prepare('DELETE FROM elo_history WHERE elo_version = ?').run(config.version_name);

    games.forEach((game, i) => {
      // game.player1_id is the winner of the game.
      const winnerBefore = playerElos.get(game.player1_id)!;
      const loserBefore = playerElos.get(game.player2_id)!;
      const winnerK = dynamicKFactor(
        config.k_factor,
        config.base_k_factor,
        config.new_player_k_bonus,
        config.new_player_bonus_period,
        gamesPlayed.get(game.player1_id) ?? 0
      );
      const loserK = dynamicKFactor(
        config.k_factor,
        config.base_k_factor,
        config.new_player_k_bonus,
        config.new_player_bonus_period,
        gamesPlayed.get(game.player2_id) ?? 0
      );

      const [change] = calculateMatchEloChanges(
        winnerBefore,
        loserBefore,
        [{ gameId: game.id, winner: 'Player1' }],
        winnerK,
        loserK
      );

      playerElos.set(game.player1_id, change.player1_elo_after);
      playerElos.set(game.player2_id, change.player2_elo_after);
      gamesPlayed.set(game.player1_id, (gamesPlayed.get(game.player1_id) ?? 0) + 1);
      gamesPlayed.set(game.player2_id, (gamesPlayed.get(game.player2_id) ?? 0) + 1);

      updateGameVersion.run(config.version_name, game.id);
      insertHistory.run(
        uuid(),
        game.player1_id,
        game.id,
        change.player1_elo_before,
        change.player1_elo_after,
        config.version_name,
        game.season_id,
        game.played_at
      );
      insertHistory.run(
        uuid(),
        game.player2_id,
        game.id,
        change.player2_elo_before,
        change.player2_elo_after,
        config.version_name,
        game.season_id,
        game.played_at
      );

      if ((i + 1) % 100 === 0) {
        updateJobProgressItems(db, jobId, i + 1, games.length);
      }
    });

    const updateElo = db.prepare('UPDATE players SET current_elo = ? WHERE id = ?');
    for (const [playerId, elo] of playerElos) {
      updateElo.run(elo, playerId);
    }
  })();
}

export function registerEloRoutes(app: FastifyInstance, db: DB): void {
  const { requireAdmin } = makeAuthHooks(db);

  const getByVersion = (versionName: string): EloConfigRow | undefined =>
    db.prepare('SELECT * FROM elo_configurations WHERE version_name = ?').get(versionName) as
      | EloConfigRow
      | undefined;

  app.post<{ Body: EloConfigBody }>(
    '/api/admin/elo-configurations',
    { preHandler: requireAdmin },
    async (request) => {
      const body = request.body;
      if (!body.version_name || body.version_name.length > MAX_VERSION_NAME_LENGTH) {
        throw invalidInput(`Version name must be 1-${MAX_VERSION_NAME_LENGTH} characters`);
      }
      validateConfigBody(body);
      if (getByVersion(body.version_name)) throw invalidInput('Version name already exists');

      const id = uuid();
      db.prepare(
        `INSERT INTO elo_configurations
           (id, version_name, k_factor, starting_elo, base_k_factor, new_player_k_bonus,
            new_player_bonus_period, description, is_active, created_at, created_by)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)`
      ).run(
        id,
        body.version_name,
        body.k_factor,
        body.starting_elo,
        body.base_k_factor ?? null,
        body.new_player_k_bonus ?? null,
        body.new_player_bonus_period ?? null,
        body.description ?? null,
        nowIso(),
        request.user.id
      );
      return mapConfig(getByVersion(body.version_name)!);
    }
  );

  app.get('/api/admin/elo-configurations', { preHandler: requireAdmin }, async () => {
    const rows = db
      .prepare('SELECT * FROM elo_configurations ORDER BY created_at DESC')
      .all() as EloConfigRow[];
    return rows.map(mapConfig);
  });

  app.put<{ Params: { versionName: string }; Body: EloConfigBody }>(
    '/api/admin/elo-configurations/:versionName',
    { preHandler: requireAdmin },
    async (request) => {
      const body = request.body;
      validateConfigBody(body);

      const existing = getByVersion(request.params.versionName);
      if (!existing) throw invalidInput('Configuration not found');
      if (asBool(existing.is_active)) {
        throw invalidInput('Cannot update active configuration. Deactivate it first.');
      }

      db.prepare(
        `UPDATE elo_configurations
         SET k_factor = ?, starting_elo = ?, base_k_factor = ?, new_player_k_bonus = ?,
             new_player_bonus_period = ?, description = ?
         WHERE version_name = ?`
      ).run(
        body.k_factor,
        body.starting_elo,
        body.base_k_factor ?? null,
        body.new_player_k_bonus ?? null,
        body.new_player_bonus_period ?? null,
        body.description ?? null,
        request.params.versionName
      );
      return mapConfig(getByVersion(request.params.versionName)!);
    }
  );

  app.delete<{ Params: { versionName: string } }>(
    '/api/admin/elo-configurations/:versionName',
    { preHandler: requireAdmin },
    async (request) => {
      const existing = getByVersion(request.params.versionName);
      if (existing && asBool(existing.is_active)) {
        throw invalidInput('Cannot delete active configuration. Deactivate it first.');
      }
      const result = db
        .prepare('DELETE FROM elo_configurations WHERE version_name = ?')
        .run(request.params.versionName);
      if (result.changes === 0) throw invalidInput('Configuration not found');
      return { message: `Configuration '${request.params.versionName}' deleted` };
    }
  );

  app.post<{ Params: { versionName: string } }>(
    '/api/admin/elo-configurations/:versionName/activate',
    { preHandler: requireAdmin },
    async (request) => {
      if (!getByVersion(request.params.versionName)) throw invalidInput('Configuration not found');
      db.transaction(() => {
        db.prepare('UPDATE elo_configurations SET is_active = 0').run();
        db.prepare('UPDATE elo_configurations SET is_active = 1 WHERE version_name = ?').run(
          request.params.versionName
        );
      })();
      return { message: `Configuration '${request.params.versionName}' activated` };
    }
  );

  app.post<{ Params: { versionName: string } }>(
    '/api/admin/elo-configurations/:versionName/recalculate',
    { preHandler: requireAdmin },
    async (request) => {
      const row = getByVersion(request.params.versionName);
      if (!row) throw invalidInput('Configuration not found');

      const config: EloConfig = {
        version_name: row.version_name,
        k_factor: row.k_factor,
        starting_elo: row.starting_elo,
        base_k_factor: row.base_k_factor,
        new_player_k_bonus: row.new_player_k_bonus,
        new_player_bonus_period: row.new_player_bonus_period,
      };
      const jobId = createJob(db, 'elo_recalculation', request.user.id);

      setImmediate(() => {
        try {
          markJobRunning(db, jobId);
          recalculateAllElo(db, config, jobId);
          markJobFinished(db, jobId, 'completed', {
            version: config.version_name,
            message: 'Recalculation completed successfully',
          });
        } catch (err) {
          app.log.error(err, 'ELO recalculation failed');
          markJobFinished(db, jobId, 'failed', { error: `Recalculation failed: ${(err as Error).message}` });
        }
      });

      return {
        message: `Started ELO recalculation for version '${request.params.versionName}'`,
        job_id: jobId,
        version: request.params.versionName,
      };
    }
  );

  app.get<{ Params: { jobId: string } }>(
    '/api/admin/jobs/:jobId',
    { preHandler: requireAdmin },
    async (request) => {
      const job = getJob(db, request.params.jobId);
      if (!job) throw invalidInput('Job not found');
      return job;
    }
  );
}
