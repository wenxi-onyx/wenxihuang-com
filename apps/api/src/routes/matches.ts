import type { FastifyInstance } from 'fastify';
import type { DB } from '../db.js';
import { makeAuthHooks } from '../auth.js';
import { calculateMatchEloChanges, dynamicKFactor, type GameWinner } from '../elo.js';
import { invalidInput } from '../errors.js';
import { getActiveSeason, recalculateSeasonElo } from '../seasons.js';
import { nowIso, toIso, uuid } from '../util.js';

interface CreateMatchBody {
  player1_id: string;
  player2_id: string;
  games: GameWinner[];
  submitted_at?: string;
}

interface GameDetail {
  game_number: number;
  winner: GameWinner;
  player1_elo_before: number;
  player1_elo_after: number;
  player1_elo_change: number;
  player2_elo_before: number;
  player2_elo_after: number;
  player2_elo_change: number;
  played_at: string;
}

function formatPlayerName(first: string, last: string): string {
  const f = first.trim();
  const l = last.trim();
  if (!f && !l) return 'Unknown Player';
  if (!f) return l;
  if (!l) return f;
  return `${f} ${l}`;
}

/** Game N of a match is timestamped 5 minutes before game N+1; the last game = submitted_at. */
const gamePlayedAt = (submittedAt: string, numGames: number, index: number): string =>
  new Date(new Date(submittedAt).getTime() - (numGames - 1 - index) * 5 * 60 * 1000).toISOString();

export function registerMatchRoutes(app: FastifyInstance, db: DB): void {
  const { requireAuth, requireAdmin } = makeAuthHooks(db);

  app.post<{ Body: CreateMatchBody }>('/api/user/matches', { preHandler: requireAuth }, async (request, reply) => {
    const payload = request.body;

    if (payload.player1_id === payload.player2_id) throw invalidInput('Players must be different');
    if (!payload.games || payload.games.length === 0) throw invalidInput('Match must have at least one game');
    for (const w of payload.games) {
      if (w !== 'Player1' && w !== 'Player2') throw invalidInput('Invalid game winner');
    }

    const season = getActiveSeason(db);
    if (!season) throw invalidInput('No active season found');

    const getPlayer = db.prepare('SELECT id, first_name, last_name, is_active FROM players WHERE id = ?');
    const player1 = getPlayer.get(payload.player1_id) as
      | { id: string; first_name: string; last_name: string; is_active: number }
      | undefined;
    const player2 = getPlayer.get(payload.player2_id) as
      | { id: string; first_name: string; last_name: string; is_active: number }
      | undefined;
    if (!player1) throw invalidInput('Player 1 not found');
    if (!player2) throw invalidInput('Player 2 not found');
    if (!player1.is_active)
      throw invalidInput(`Player ${player1.first_name} ${player1.last_name} is not active`);
    if (!player2.is_active)
      throw invalidInput(`Player ${player2.first_name} ${player2.last_name} is not active`);

    const getPlayerSeason = db.prepare(
      'SELECT current_elo, games_played, is_included FROM player_seasons WHERE player_id = ? AND season_id = ?'
    );
    const p1Season = getPlayerSeason.get(payload.player1_id, season.id) as
      | { current_elo: number; games_played: number; is_included: number }
      | undefined;
    const p2Season = getPlayerSeason.get(payload.player2_id, season.id) as
      | { current_elo: number; games_played: number; is_included: number }
      | undefined;
    if (!p1Season)
      throw invalidInput(`Player ${player1.first_name} ${player1.last_name} is not in the active season`);
    if (!p2Season)
      throw invalidInput(`Player ${player2.first_name} ${player2.last_name} is not in the active season`);
    if (!p1Season.is_included)
      throw invalidInput(`Player ${player1.first_name} ${player1.last_name} is not included in the active season`);
    if (!p2Season.is_included)
      throw invalidInput(`Player ${player2.first_name} ${player2.last_name} is not included in the active season`);

    const submittedAt = payload.submitted_at ? toIso(payload.submitted_at) : nowIso();
    const numGames = payload.games.length;

    const kOf = (gamesPlayed: number) =>
      dynamicKFactor(
        season.k_factor,
        season.base_k_factor,
        season.new_player_k_bonus,
        season.new_player_bonus_period,
        gamesPlayed
      );
    const player1K = kOf(p1Season.games_played);
    const player2K = kOf(p2Season.games_played);

    const matchId = uuid();
    const gameDetails: GameDetail[] = [];
    let player1EloBefore = 0;
    let player1EloAfter = 0;
    let player2EloBefore = 0;
    let player2EloAfter = 0;

    db.transaction(() => {
      const now = nowIso();
      db.prepare(
        `INSERT INTO matches (id, player1_id, player2_id, season_id, submitted_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)`
      ).run(matchId, payload.player1_id, payload.player2_id, season.id, submittedAt, now, now);

      const insertGame = db.prepare(
        `INSERT INTO games (id, match_id, player1_id, player2_id, season_id, elo_version, played_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)`
      );

      const gamesWithIds: Array<{ gameId: string; winner: GameWinner; playedAt: string }> = payload.games.map(
        (winner, i) => {
          const gameId = uuid();
          const playedAt = gamePlayedAt(submittedAt, numGames, i);
          // games.player1_id is always the winner of that game.
          const [winnerId, loserId] =
            winner === 'Player1'
              ? [payload.player1_id, payload.player2_id]
              : [payload.player2_id, payload.player1_id];
          insertGame.run(gameId, matchId, winnerId, loserId, season.id, season.elo_version ?? 'v1', playedAt);
          return { gameId, winner, playedAt };
        }
      );

      const changes = calculateMatchEloChanges(
        p1Season.current_elo,
        p2Season.current_elo,
        gamesWithIds.map(({ gameId, winner }) => ({ gameId, winner })),
        player1K,
        player2K
      );

      const insertHistory = db.prepare(
        `INSERT INTO elo_history (id, player_id, game_id, elo_before, elo_after, elo_version, season_id, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)`
      );

      changes.forEach((change, i) => {
        const { winner, playedAt } = gamesWithIds[i];
        insertHistory.run(
          uuid(),
          payload.player1_id,
          change.game_id,
          change.player1_elo_before,
          change.player1_elo_after,
          season.elo_version,
          season.id,
          playedAt
        );
        insertHistory.run(
          uuid(),
          payload.player2_id,
          change.game_id,
          change.player2_elo_before,
          change.player2_elo_after,
          season.elo_version,
          season.id,
          playedAt
        );
        gameDetails.push({
          game_number: i + 1,
          winner,
          player1_elo_before: change.player1_elo_before,
          player1_elo_after: change.player1_elo_after,
          player1_elo_change: change.player1_elo_change,
          player2_elo_before: change.player2_elo_before,
          player2_elo_after: change.player2_elo_after,
          player2_elo_change: change.player2_elo_change,
          played_at: playedAt,
        });
      });

      const first = changes[0];
      const last = changes[changes.length - 1];
      player1EloBefore = first.player1_elo_before;
      player1EloAfter = last.player1_elo_after;
      player2EloBefore = first.player2_elo_before;
      player2EloAfter = last.player2_elo_after;

      const p1GamesWon = payload.games.filter((w) => w === 'Player1').length;
      const p2GamesWon = payload.games.filter((w) => w === 'Player2').length;

      const updateSeasonStats = db.prepare(
        `UPDATE player_seasons
         SET current_elo = ?, games_played = games_played + ?, wins = wins + ?, losses = losses + ?
         WHERE player_id = ? AND season_id = ?`
      );
      updateSeasonStats.run(player1EloAfter, numGames, p1GamesWon, p2GamesWon, payload.player1_id, season.id);
      updateSeasonStats.run(player2EloAfter, numGames, p2GamesWon, p1GamesWon, payload.player2_id, season.id);

      const updateElo = db.prepare('UPDATE players SET current_elo = ? WHERE id = ?');
      updateElo.run(player1EloAfter, payload.player1_id);
      updateElo.run(player2EloAfter, payload.player2_id);
    })();

    const p1GamesWon = payload.games.filter((w) => w === 'Player1').length;
    const p2GamesWon = payload.games.filter((w) => w === 'Player2').length;

    reply.code(201);
    return {
      message: 'Match created successfully',
      match_data: {
        id: matchId,
        player1_id: payload.player1_id,
        player1_name: formatPlayerName(player1.first_name, player1.last_name),
        player1_games_won: p1GamesWon,
        player1_elo_before: player1EloBefore,
        player1_elo_after: player1EloAfter,
        player1_elo_change: player1EloAfter - player1EloBefore,
        player2_id: payload.player2_id,
        player2_name: formatPlayerName(player2.first_name, player2.last_name),
        player2_games_won: p2GamesWon,
        player2_elo_before: player2EloBefore,
        player2_elo_after: player2EloAfter,
        player2_elo_change: player2EloAfter - player2EloBefore,
        season_id: season.id,
        season_name: season.name,
        total_games: numGames,
        submitted_at: submittedAt,
        games: gameDetails,
      },
    };
  });

  app.get<{ Querystring: { page?: string; limit?: string } }>('/api/matches', async (request) => {
    const limit = Math.min(Math.max(Number(request.query.limit) || 50, 1), 100);
    const page = Math.max(Number(request.query.page) || 1, 1);
    const offset = (page - 1) * limit;

    const { total } = db.prepare('SELECT COUNT(*) AS total FROM matches').get() as { total: number };
    const totalPages = Math.ceil(total / limit);

    const matches = db
      .prepare(
        `SELECT m.id, m.player1_id, m.player2_id, m.season_id, m.submitted_at,
                p1.first_name AS player1_first_name, p1.last_name AS player1_last_name,
                p2.first_name AS player2_first_name, p2.last_name AS player2_last_name,
                s.name AS season_name
         FROM matches m
         JOIN players p1 ON m.player1_id = p1.id
         JOIN players p2 ON m.player2_id = p2.id
         JOIN seasons s ON m.season_id = s.id
         ORDER BY m.submitted_at DESC
         LIMIT ? OFFSET ?`
      )
      .all(limit, offset) as Array<{
      id: string;
      player1_id: string;
      player2_id: string;
      season_id: string;
      submitted_at: string;
      player1_first_name: string;
      player1_last_name: string;
      player2_first_name: string;
      player2_last_name: string;
      season_name: string;
    }>;

    const getGames = db.prepare(
      `SELECT g.id, g.player1_id, g.player2_id, g.played_at,
              eh1.elo_before AS player1_elo_before, eh1.elo_after AS player1_elo_after,
              eh2.elo_before AS player2_elo_before, eh2.elo_after AS player2_elo_after
       FROM games g
       JOIN elo_history eh1 ON g.id = eh1.game_id AND eh1.player_id = ? AND eh1.season_id = g.season_id
       JOIN elo_history eh2 ON g.id = eh2.game_id AND eh2.player_id = ? AND eh2.season_id = g.season_id
       WHERE g.match_id = ?
       ORDER BY g.played_at ASC`
    );

    const matchesWithDetails = [];
    for (const m of matches) {
      const games = getGames.all(m.player1_id, m.player2_id, m.id) as Array<{
        id: string;
        player1_id: string;
        player2_id: string;
        played_at: string;
        player1_elo_before: number;
        player1_elo_after: number;
        player2_elo_before: number;
        player2_elo_after: number;
      }>;
      if (games.length === 0) continue;

      const first = games[0];
      const last = games[games.length - 1];
      const p1GamesWon = games.filter((g) => g.player1_id === m.player1_id).length;
      const p2GamesWon = games.filter((g) => g.player1_id === m.player2_id).length;

      matchesWithDetails.push({
        id: m.id,
        player1_id: m.player1_id,
        player1_name: formatPlayerName(m.player1_first_name, m.player1_last_name),
        player1_games_won: p1GamesWon,
        player1_elo_before: first.player1_elo_before,
        player1_elo_after: last.player1_elo_after,
        player1_elo_change: last.player1_elo_after - first.player1_elo_before,
        player2_id: m.player2_id,
        player2_name: formatPlayerName(m.player2_first_name, m.player2_last_name),
        player2_games_won: p2GamesWon,
        player2_elo_before: first.player2_elo_before,
        player2_elo_after: last.player2_elo_after,
        player2_elo_change: last.player2_elo_after - first.player2_elo_before,
        season_id: m.season_id,
        season_name: m.season_name,
        total_games: games.length,
        submitted_at: m.submitted_at,
        games: games.map((g, i) => ({
          game_number: i + 1,
          winner: g.player1_id === m.player1_id ? 'Player1' : 'Player2',
          player1_elo_before: g.player1_elo_before,
          player1_elo_after: g.player1_elo_after,
          player1_elo_change: g.player1_elo_after - g.player1_elo_before,
          player2_elo_before: g.player2_elo_before,
          player2_elo_after: g.player2_elo_after,
          player2_elo_change: g.player2_elo_after - g.player2_elo_before,
          played_at: g.played_at,
        })),
      });
    }

    return { matches: matchesWithDetails, total, page, limit, total_pages: totalPages };
  });

  app.delete<{ Params: { matchId: string } }>(
    '/api/admin/matches/:matchId',
    { preHandler: requireAdmin },
    async (request) => {
      const match = db.prepare('SELECT season_id FROM matches WHERE id = ?').get(request.params.matchId) as
        | { season_id: string }
        | undefined;
      if (!match) throw invalidInput('Match not found');

      // Games cascade via FK; elo_history is rebuilt by the recalculation.
      db.prepare('DELETE FROM matches WHERE id = ?').run(request.params.matchId);
      recalculateSeasonElo(db, match.season_id);

      return { message: 'Match deleted successfully' };
    }
  );
}
