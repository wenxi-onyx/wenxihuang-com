import type { DB } from './db.js';
import { calculateMatchEloChanges, dynamicKFactor, type GameWinner } from './elo.js';
import { asBool, nowIso, uuid } from './util.js';

export interface SeasonRow {
  id: string;
  name: string;
  description: string | null;
  start_date: string;
  starting_elo: number;
  k_factor: number;
  base_k_factor: number | null;
  new_player_k_bonus: number | null;
  new_player_bonus_period: number | null;
  elo_version: string | null;
  is_active: number;
  created_at: string;
  created_by: string | null;
}

/** API shape (matches the old SeasonResponse serialization). */
export interface Season {
  id: string;
  name: string;
  description: string | null;
  start_date: string;
  starting_elo: number;
  k_factor: number;
  base_k_factor: number | null;
  new_player_k_bonus: number | null;
  new_player_bonus_period: number | null;
  elo_version: string | null;
  is_active: boolean;
  created_at: string;
}

export const mapSeason = (row: SeasonRow): Season => ({
  id: row.id,
  name: row.name,
  description: row.description,
  start_date: row.start_date,
  starting_elo: row.starting_elo,
  k_factor: row.k_factor,
  base_k_factor: row.base_k_factor,
  new_player_k_bonus: row.new_player_k_bonus,
  new_player_bonus_period: row.new_player_bonus_period,
  elo_version: row.elo_version,
  is_active: asBool(row.is_active),
  created_at: row.created_at,
});

export const getActiveSeason = (db: DB): SeasonRow | undefined =>
  db.prepare('SELECT * FROM seasons WHERE is_active = 1 LIMIT 1').get() as SeasonRow | undefined;

export const getSeasonById = (db: DB, id: string): SeasonRow | undefined =>
  db.prepare('SELECT * FROM seasons WHERE id = ?').get(id) as SeasonRow | undefined;

export const getSeasonByName = (db: DB, name: string): SeasonRow | undefined =>
  db.prepare('SELECT * FROM seasons WHERE name = ?').get(name) as SeasonRow | undefined;

export const getAllSeasons = (db: DB): SeasonRow[] =>
  db.prepare('SELECT * FROM seasons ORDER BY start_date DESC').all() as SeasonRow[];

export function activateSeason(db: DB, seasonId: string): void {
  db.transaction(() => {
    db.prepare('UPDATE seasons SET is_active = 0').run();
    db.prepare('UPDATE seasons SET is_active = 1 WHERE id = ?').run(seasonId);
  })();
}

export function updateSeasonEloVersion(db: DB, seasonId: string, eloVersion: string | null): void {
  db.prepare('UPDATE seasons SET elo_version = ? WHERE id = ?').run(eloVersion, seasonId);
}

function insertPlayerSeason(db: DB, playerId: string, seasonId: string, startingElo: number, isIncluded = true): void {
  const now = nowIso();
  db.prepare(
    `INSERT INTO player_seasons (id, player_id, season_id, current_elo, games_played, wins, losses, is_included, created_at, updated_at)
     VALUES (?, ?, ?, ?, 0, 0, 0, ?, ?, ?)`
  ).run(uuid(), playerId, seasonId, startingElo, isIncluded ? 1 : 0, now, now);
}

export function initializeSeasonPlayers(db: DB, seasonId: string, playerIds?: string[]): number {
  const season = getSeasonById(db, seasonId);
  if (!season) throw new Error('Season not found');

  const ids =
    playerIds ??
    (db.prepare('SELECT id FROM players WHERE is_active = 1').all() as Array<{ id: string }>).map((r) => r.id);

  let count = 0;
  for (const playerId of ids) {
    if (playerIds) {
      const exists = db.prepare('SELECT 1 FROM players WHERE id = ?').get(playerId);
      if (!exists) continue;
    }
    const existing = db
      .prepare('SELECT id FROM player_seasons WHERE player_id = ? AND season_id = ?')
      .get(playerId, seasonId);
    if (!existing) {
      insertPlayerSeason(db, playerId, seasonId, season.starting_elo);
      count += 1;
    }
  }
  return count;
}

export interface LeaderboardEntry {
  player_id: string;
  first_name: string;
  last_name: string;
  current_elo: number;
  games_played: number;
  wins: number;
  losses: number;
  is_active: number;
}

export const getSeasonLeaderboard = (db: DB, seasonId: string): LeaderboardEntry[] =>
  db
    .prepare(
      `SELECT p.id AS player_id, p.first_name, p.last_name, ps.current_elo,
              ps.games_played, ps.wins, ps.losses, p.is_active
       FROM player_seasons ps
       JOIN players p ON ps.player_id = p.id
       WHERE ps.season_id = ? AND ps.is_included = 1
       ORDER BY ps.current_elo DESC`
    )
    .all(seasonId) as LeaderboardEntry[];

export const getSeasonPlayers = (
  db: DB,
  seasonId: string
): Array<{ id: string; first_name: string; last_name: string; is_included: number; is_active: number }> =>
  db
    .prepare(
      `SELECT p.id, p.first_name, p.last_name, ps.is_included, p.is_active
       FROM player_seasons ps
       JOIN players p ON ps.player_id = p.id
       WHERE ps.season_id = ?
       ORDER BY p.first_name, p.last_name`
    )
    .all(seasonId) as Array<{ id: string; first_name: string; last_name: string; is_included: number; is_active: number }>;

export const getAvailablePlayersForSeason = (
  db: DB,
  seasonId: string
): Array<{ id: string; first_name: string; last_name: string; is_active: number }> =>
  db
    .prepare(
      `SELECT p.id, p.first_name, p.last_name, p.is_active
       FROM players p
       WHERE NOT EXISTS (
         SELECT 1 FROM player_seasons ps WHERE ps.player_id = p.id AND ps.season_id = ?
       )
       ORDER BY p.first_name, p.last_name`
    )
    .all(seasonId) as Array<{ id: string; first_name: string; last_name: string; is_active: number }>;

export function addPlayerToSeason(db: DB, playerId: string, seasonId: string): void {
  const season = getSeasonById(db, seasonId);
  if (!season) throw new Error('Season not found');

  const existing = db
    .prepare('SELECT id FROM player_seasons WHERE player_id = ? AND season_id = ?')
    .get(playerId, seasonId);
  if (existing) {
    db.prepare('UPDATE player_seasons SET is_included = 1 WHERE player_id = ? AND season_id = ?').run(
      playerId,
      seasonId
    );
  } else {
    insertPlayerSeason(db, playerId, seasonId, season.starting_elo);
  }
}

export function removePlayerFromSeason(db: DB, playerId: string, seasonId: string): void {
  db.prepare('UPDATE player_seasons SET is_included = 0 WHERE player_id = ? AND season_id = ?').run(
    playerId,
    seasonId
  );
}

/**
 * Reassign all matches (and their games) to the season whose start_date is the
 * latest one <= the match's submitted_at. Matches predating every season are
 * left untouched.
 */
export function reassignGamesToSeasons(db: DB): number {
  const matchesResult = db
    .prepare(
      `UPDATE matches
       SET season_id = (
         SELECT s.id FROM seasons s
         WHERE s.start_date <= matches.submitted_at
         ORDER BY s.start_date DESC LIMIT 1
       )
       WHERE EXISTS (SELECT 1 FROM seasons s WHERE s.start_date <= matches.submitted_at)
         AND season_id != (
           SELECT s.id FROM seasons s
           WHERE s.start_date <= matches.submitted_at
           ORDER BY s.start_date DESC LIMIT 1
         )`
    )
    .run();

  const gamesResult = db
    .prepare(
      `UPDATE games
       SET season_id = m.season_id
       FROM matches m
       WHERE games.match_id = m.id AND games.season_id != m.season_id`
    )
    .run();

  return matchesResult.changes + gamesResult.changes;
}

/**
 * Recalculate all ELO for one season, processing games grouped by match so the
 * sequential within-match calculation matches live submission exactly.
 */
export function recalculateSeasonElo(db: DB, seasonId: string): void {
  const season = getSeasonById(db, seasonId);
  if (!season) throw new Error('Season not found');

  const eloVersionString = season.elo_version ?? season.name.slice(0, 50);

  // Resolve the ELO configuration: referenced config, else the season's own values.
  let kFactor = season.k_factor;
  let baseK = season.base_k_factor;
  let bonus = season.new_player_k_bonus;
  let period = season.new_player_bonus_period;
  let startingElo = season.starting_elo;

  if (season.elo_version) {
    const config = db
      .prepare(
        `SELECT k_factor, base_k_factor, new_player_k_bonus, new_player_bonus_period, starting_elo
         FROM elo_configurations WHERE version_name = ?`
      )
      .get(season.elo_version) as
      | { k_factor: number; base_k_factor: number | null; new_player_k_bonus: number | null; new_player_bonus_period: number | null; starting_elo: number }
      | undefined;
    if (config) {
      kFactor = config.k_factor;
      baseK = config.base_k_factor;
      bonus = config.new_player_k_bonus;
      period = config.new_player_bonus_period;
      startingElo = config.starting_elo;
    }
  }

  const matches = db
    .prepare('SELECT id, player1_id, player2_id FROM matches WHERE season_id = ? ORDER BY submitted_at ASC')
    .all(seasonId) as Array<{ id: string; player1_id: string; player2_id: string }>;

  const playerElos = new Map<string, number>();
  const gamesPlayed = new Map<string, number>();
  const wins = new Map<string, number>();
  const losses = new Map<string, number>();

  const playerSeasonRows = db
    .prepare('SELECT player_id FROM player_seasons WHERE season_id = ?')
    .all(seasonId) as Array<{ player_id: string }>;
  for (const { player_id } of playerSeasonRows) {
    playerElos.set(player_id, startingElo);
    gamesPlayed.set(player_id, 0);
    wins.set(player_id, 0);
    losses.set(player_id, 0);
  }

  const insertHistory = db.prepare(
    `INSERT INTO elo_history (id, player_id, game_id, elo_before, elo_after, elo_version, season_id, created_at)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`
  );

  db.transaction(() => {
    db.prepare('DELETE FROM elo_history WHERE season_id = ?').run(seasonId);

    for (const match of matches) {
      const games = db
        .prepare('SELECT id, player1_id, player2_id, played_at FROM games WHERE match_id = ? ORDER BY played_at ASC')
        .all(match.id) as Array<{ id: string; player1_id: string; player2_id: string; played_at: string }>;
      if (games.length === 0) continue;

      const p1Before = playerElos.get(match.player1_id);
      const p2Before = playerElos.get(match.player2_id);
      // Players not in this season: skip the match (parity with old behavior).
      if (p1Before === undefined || p2Before === undefined) continue;

      const p1K = dynamicKFactor(kFactor, baseK, bonus, period, gamesPlayed.get(match.player1_id) ?? 0);
      const p2K = dynamicKFactor(kFactor, baseK, bonus, period, gamesPlayed.get(match.player2_id) ?? 0);

      const gameWinners = games.map((g) => ({
        gameId: g.id,
        winner: (g.player1_id === match.player1_id ? 'Player1' : 'Player2') as GameWinner,
      }));

      const changes = calculateMatchEloChanges(p1Before, p2Before, gameWinners, p1K, p2K);

      changes.forEach((change, i) => {
        const playedAt = games[i].played_at;
        insertHistory.run(
          uuid(),
          match.player1_id,
          change.game_id,
          change.player1_elo_before,
          change.player1_elo_after,
          eloVersionString,
          seasonId,
          playedAt
        );
        insertHistory.run(
          uuid(),
          match.player2_id,
          change.game_id,
          change.player2_elo_before,
          change.player2_elo_after,
          eloVersionString,
          seasonId,
          playedAt
        );

        // games.player1_id is the per-game winner.
        const winnerId = games[i].player1_id;
        const loserId = games[i].player2_id;
        gamesPlayed.set(winnerId, (gamesPlayed.get(winnerId) ?? 0) + 1);
        gamesPlayed.set(loserId, (gamesPlayed.get(loserId) ?? 0) + 1);
        wins.set(winnerId, (wins.get(winnerId) ?? 0) + 1);
        losses.set(loserId, (losses.get(loserId) ?? 0) + 1);
      });

      const last = changes[changes.length - 1];
      playerElos.set(match.player1_id, last.player1_elo_after);
      playerElos.set(match.player2_id, last.player2_elo_after);
    }

    const updateStats = db.prepare(
      `UPDATE player_seasons
       SET current_elo = ?, games_played = ?, wins = ?, losses = ?
       WHERE player_id = ? AND season_id = ?`
    );
    for (const [playerId, elo] of playerElos) {
      updateStats.run(
        elo,
        gamesPlayed.get(playerId) ?? 0,
        wins.get(playerId) ?? 0,
        losses.get(playerId) ?? 0,
        playerId,
        seasonId
      );
    }
  })();
}

/** Recalculate every season whose start_date >= fromDate, oldest first. */
export function recalculateSeasonsFrom(db: DB, fromDate: string): void {
  const seasons = db
    .prepare('SELECT id FROM seasons WHERE start_date >= ? ORDER BY start_date ASC')
    .all(fromDate) as Array<{ id: string }>;
  for (const { id } of seasons) {
    recalculateSeasonElo(db, id);
  }
}

export interface CreateSeasonInput {
  name: string;
  description: string | null;
  start_date: string;
  starting_elo: number;
  k_factor: number;
  base_k_factor: number | null;
  new_player_k_bonus: number | null;
  new_player_bonus_period: number | null;
  elo_version: string | null;
  created_by: string;
  player_ids?: string[];
}

/**
 * Create + activate a season, initialize its players, reassign games by
 * timestamp, and recalculate every season from its start date onward.
 * On any failure after insertion the season is rolled back.
 */
export function createSeason(db: DB, input: CreateSeasonInput): SeasonRow {
  const name = input.name.trim();
  if (!name) throw new Error('Season name cannot be empty');

  const id = uuid();
  db.transaction(() => {
    db.prepare('UPDATE seasons SET is_active = 0').run();
    db.prepare(
      `INSERT INTO seasons
         (id, name, description, start_date, starting_elo, k_factor, base_k_factor,
          new_player_k_bonus, new_player_bonus_period, elo_version, is_active, created_at, created_by)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?)`
    ).run(
      id,
      name,
      input.description,
      input.start_date,
      input.starting_elo,
      input.k_factor,
      input.base_k_factor,
      input.new_player_k_bonus,
      input.new_player_bonus_period,
      input.elo_version,
      nowIso(),
      input.created_by
    );
  })();

  try {
    initializeSeasonPlayers(db, id, input.player_ids);
    reassignGamesToSeasons(db);
    recalculateSeasonsFrom(db, input.start_date);
  } catch (err) {
    cleanupSeason(db, id);
    throw err;
  }

  return getSeasonById(db, id)!;
}

function cleanupSeason(db: DB, seasonId: string): void {
  db.transaction(() => {
    db.prepare('DELETE FROM player_seasons WHERE season_id = ?').run(seasonId);
    db.prepare('DELETE FROM elo_history WHERE season_id = ?').run(seasonId);
    db.prepare('DELETE FROM seasons WHERE id = ?').run(seasonId);
  })();
}

/**
 * Delete a season: reassign its matches/games to the chronologically previous
 * season, drop its stats/history, then recalculate affected seasons.
 */
export function deleteSeason(db: DB, seasonId: string): void {
  const season = getSeasonById(db, seasonId);
  if (!season) throw new Error('Season not found');

  const target = db
    .prepare('SELECT * FROM seasons WHERE start_date < ? ORDER BY start_date DESC LIMIT 1')
    .get(season.start_date) as SeasonRow | undefined;

  db.transaction(() => {
    if (target) {
      db.prepare('UPDATE matches SET season_id = ? WHERE season_id = ?').run(target.id, seasonId);
      db.prepare('UPDATE games SET season_id = ? WHERE season_id = ?').run(target.id, seasonId);
    } else {
      const { n } = db.prepare('SELECT COUNT(*) AS n FROM matches WHERE season_id = ?').get(seasonId) as {
        n: number;
      };
      if (n > 0) {
        throw new Error(
          `Cannot delete season '${season.name}': no previous season exists to reassign ${n} matches to`
        );
      }
    }

    db.prepare('DELETE FROM elo_history WHERE season_id = ?').run(seasonId);
    db.prepare('DELETE FROM player_seasons WHERE season_id = ?').run(seasonId);
    db.prepare('DELETE FROM seasons WHERE id = ?').run(seasonId);
  })();

  recalculateSeasonsFrom(db, target ? target.start_date : season.start_date);
}
