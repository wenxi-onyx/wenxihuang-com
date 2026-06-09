import type { FastifyInstance } from 'fastify';
import type { DB } from '../db.js';
import { makeAuthHooks } from '../auth.js';
import { databaseError } from '../errors.js';
import { asBool, nowIso } from '../util.js';

export interface EloHistoryPoint {
  match_id: string;
  elo_before: number;
  elo_after: number;
  elo_version: string;
  season_id: string;
  season_name: string;
  created_at: string;
}

/**
 * Per-match ELO history for one player: elo_before of the first game and
 * elo_after of the last game of each match, ordered by match submission time.
 */
export function getPlayerHistory(db: DB, playerId: string): EloHistoryPoint[] {
  const rows = db
    .prepare(
      `SELECT g.match_id, eh.elo_before, eh.elo_after, eh.elo_version, eh.season_id,
              s.name AS season_name, m.submitted_at, g.played_at
       FROM elo_history eh
       JOIN games g ON eh.game_id = g.id
       JOIN matches m ON g.match_id = m.id
       JOIN seasons s ON eh.season_id = s.id
       WHERE eh.player_id = ?
       ORDER BY m.submitted_at ASC, g.played_at ASC`
    )
    .all(playerId) as Array<{
    match_id: string;
    elo_before: number;
    elo_after: number;
    elo_version: string;
    season_id: string;
    season_name: string;
    submitted_at: string;
    played_at: string;
  }>;

  const byMatch = new Map<string, EloHistoryPoint>();
  for (const row of rows) {
    const existing = byMatch.get(row.match_id);
    if (!existing) {
      byMatch.set(row.match_id, {
        match_id: row.match_id,
        elo_before: row.elo_before,
        elo_after: row.elo_after,
        elo_version: row.elo_version,
        season_id: row.season_id,
        season_name: row.season_name,
        created_at: row.submitted_at,
      });
    } else {
      existing.elo_after = row.elo_after;
    }
  }
  return [...byMatch.values()];
}

export function registerPlayerRoutes(app: FastifyInstance, db: DB): void {
  const { requireAdmin } = makeAuthHooks(db);

  app.get('/api/players', async () => {
    const rows = db
      .prepare(
        `SELECT p.id,
                p.first_name || ' ' || p.last_name AS name,
                p.current_elo,
                p.is_active,
                COUNT(DISTINCT g.id) AS games_played,
                COUNT(DISTINCT CASE WHEN g.player1_id = p.id THEN g.id END) AS wins,
                COUNT(DISTINCT CASE WHEN g.player2_id = p.id THEN g.id END) AS losses,
                p.created_at,
                COALESCE(p.updated_at, p.created_at) AS updated_at
         FROM players p
         LEFT JOIN games g ON (g.player1_id = p.id OR g.player2_id = p.id)
         GROUP BY p.id
         ORDER BY p.current_elo DESC`
      )
      .all() as Array<Record<string, unknown>>;

    return rows.map((r) => ({ ...r, is_active: asBool(r.is_active) }));
  });

  app.get<{ Params: { playerId: string } }>('/api/players/:playerId/history', async (request) => {
    return getPlayerHistory(db, request.params.playerId);
  });

  app.get('/api/players/history/all', async (_request, reply) => {
    const players = db
      .prepare(
        `SELECT id, first_name || ' ' || last_name AS name
         FROM players WHERE is_active = 1 ORDER BY current_elo DESC`
      )
      .all() as Array<{ id: string; name: string }>;

    const result = players.map((p) => ({
      player_id: p.id,
      player_name: p.name,
      history: getPlayerHistory(db, p.id),
    }));

    reply.header('Cache-Control', 'public, max-age=60, stale-while-revalidate=300');
    return result;
  });

  app.get<{ Params: { playerId: string } }>('/api/players/:playerId/matches', async (request) => {
    const playerId = request.params.playerId;
    const rows = db
      .prepare(
        `SELECT m.id AS match_id, m.player1_id, m.player2_id,
                p1.first_name AS p1_first, p1.last_name AS p1_last,
                p2.first_name AS p2_first, p2.last_name AS p2_last,
                COUNT(CASE WHEN g.player1_id = @pid THEN 1 END) AS player_games_won,
                COUNT(CASE WHEN g.player1_id != @pid THEN 1 END) AS opponent_games_won,
                s.name AS season_name, m.submitted_at
         FROM matches m
         JOIN players p1 ON m.player1_id = p1.id
         JOIN players p2 ON m.player2_id = p2.id
         JOIN seasons s ON m.season_id = s.id
         JOIN games g ON g.match_id = m.id
         WHERE m.player1_id = @pid OR m.player2_id = @pid
         GROUP BY m.id
         ORDER BY m.submitted_at DESC`
      )
      .all({ pid: playerId }) as Array<{
      match_id: string;
      player1_id: string;
      player2_id: string;
      p1_first: string;
      p1_last: string;
      p2_first: string;
      p2_last: string;
      player_games_won: number;
      opponent_games_won: number;
      season_name: string;
      submitted_at: string;
    }>;

    return rows.map((row) => {
      const [opponentId, oppFirst, oppLast] =
        row.player1_id === playerId
          ? [row.player2_id, row.p2_first, row.p2_last]
          : [row.player1_id, row.p1_first, row.p1_last];
      return {
        match_id: row.match_id,
        opponent_id: opponentId,
        opponent_name: `${oppFirst.trim()} ${oppLast.trim()}`.trim(),
        player_games_won: row.player_games_won,
        opponent_games_won: row.opponent_games_won,
        result: row.player_games_won > row.opponent_games_won ? 'W' : 'L',
        season_name: row.season_name,
        submitted_at: row.submitted_at,
      };
    });
  });

  app.post<{ Params: { playerId: string } }>(
    '/api/admin/players/:playerId/toggle-active',
    { preHandler: requireAdmin },
    async (request) => {
      const row = db
        .prepare(
          `UPDATE players
           SET is_active = NOT is_active, updated_at = ?
           WHERE id = ?
           RETURNING id, first_name || ' ' || last_name AS name, current_elo, is_active,
                     created_at, updated_at`
        )
        .get(nowIso(), request.params.playerId) as Record<string, unknown> | undefined;
      if (!row) throw databaseError();
      return { ...row, is_active: asBool(row.is_active) };
    }
  );
}
