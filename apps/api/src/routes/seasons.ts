import type { FastifyInstance } from 'fastify';
import type { DB } from '../db.js';
import { makeAuthHooks } from '../auth.js';
import { invalidInput } from '../errors.js';
import {
  activateSeason,
  addPlayerToSeason,
  createSeason,
  deleteSeason,
  getActiveSeason,
  getAllSeasons,
  getAvailablePlayersForSeason,
  getSeasonById,
  getSeasonByName,
  getSeasonLeaderboard,
  getSeasonPlayers,
  mapSeason,
  recalculateSeasonElo,
  removePlayerFromSeason,
  updateSeasonEloVersion,
} from '../seasons.js';
import { asBool, toIso } from '../util.js';

const MAX_SEASON_NAME_LENGTH = 100;
const MAX_DESCRIPTION_LENGTH = 500;
const MIN_K_FACTOR = 1.0;
const MAX_K_FACTOR = 100.0;
const MIN_STARTING_ELO = 100.0;
const MAX_STARTING_ELO = 3000.0;

interface CreateSeasonBody {
  name: string;
  description?: string | null;
  start_date: string;
  starting_elo: number;
  k_factor: number;
  base_k_factor?: number | null;
  new_player_k_bonus?: number | null;
  new_player_bonus_period?: number | null;
  elo_version?: string | null;
  player_ids?: string[];
}

export function registerSeasonRoutes(app: FastifyInstance, db: DB): void {
  const { requireAdmin } = makeAuthHooks(db);

  const eloVersionExists = (version: string): boolean =>
    !!db.prepare('SELECT 1 FROM elo_configurations WHERE version_name = ?').get(version);

  // ----- public -----

  app.get('/api/seasons', async () => getAllSeasons(db).map(mapSeason));

  app.get('/api/seasons/active', async () => {
    const season = getActiveSeason(db);
    return season ? mapSeason(season) : null;
  });

  app.get('/api/seasons/active/players', async () => {
    const season = getActiveSeason(db);
    if (!season) throw invalidInput('No active season found');

    const players = db
      .prepare(
        `SELECT p.id, p.first_name, p.last_name, ps.current_elo, p.is_active
         FROM players p
         JOIN player_seasons ps ON p.id = ps.player_id
         WHERE ps.season_id = ? AND ps.is_included = 1 AND p.is_active = 1
         ORDER BY p.first_name, p.last_name`
      )
      .all(season.id) as Array<{
      id: string;
      first_name: string;
      last_name: string;
      current_elo: number;
      is_active: number;
    }>;

    return players.map((p) => ({
      id: p.id,
      name: `${p.first_name} ${p.last_name}`,
      current_elo: p.current_elo,
      is_active: asBool(p.is_active),
    }));
  });

  app.get<{ Params: { seasonId: string } }>('/api/seasons/:seasonId', async (request) => {
    const season = getSeasonById(db, request.params.seasonId);
    if (!season) throw invalidInput('Season not found');
    return mapSeason(season);
  });

  app.get<{ Params: { seasonId: string } }>('/api/seasons/:seasonId/leaderboard', async (request) => {
    return getSeasonLeaderboard(db, request.params.seasonId).map((entry) => ({
      player_id: entry.player_id,
      player_name: `${entry.first_name} ${entry.last_name}`,
      current_elo: entry.current_elo,
      games_played: entry.games_played,
      wins: entry.wins,
      losses: entry.losses,
      win_rate: entry.games_played > 0 ? (entry.wins / entry.games_played) * 100 : 0,
      is_active: asBool(entry.is_active),
    }));
  });

  // ----- admin -----

  app.post<{ Body: CreateSeasonBody }>('/api/admin/seasons', { preHandler: requireAdmin }, async (request) => {
    const body = request.body;

    if (!body.name || body.name.length > MAX_SEASON_NAME_LENGTH) {
      throw invalidInput(`Season name must be 1-${MAX_SEASON_NAME_LENGTH} characters`);
    }
    if (body.k_factor < MIN_K_FACTOR || body.k_factor > MAX_K_FACTOR) {
      throw invalidInput(`K-factor must be between ${MIN_K_FACTOR} and ${MAX_K_FACTOR}`);
    }
    if (body.starting_elo < MIN_STARTING_ELO || body.starting_elo > MAX_STARTING_ELO) {
      throw invalidInput(`Starting ELO must be between ${MIN_STARTING_ELO} and ${MAX_STARTING_ELO}`);
    }
    if (body.description != null && body.description.length > MAX_DESCRIPTION_LENGTH) {
      throw invalidInput(`Description must be ${MAX_DESCRIPTION_LENGTH} characters or less`);
    }

    const hasBaseK = body.base_k_factor != null;
    const hasBonus = body.new_player_k_bonus != null;
    const hasPeriod = body.new_player_bonus_period != null;
    if ((hasBaseK || hasBonus || hasPeriod) && !(hasBaseK && hasBonus && hasPeriod)) {
      throw invalidInput(
        'Dynamic K-factor requires all three fields: base_k_factor, new_player_k_bonus, and new_player_bonus_period'
      );
    }
    if (hasBaseK && (body.base_k_factor! < MIN_K_FACTOR || body.base_k_factor! > MAX_K_FACTOR)) {
      throw invalidInput(`Base K-factor must be between ${MIN_K_FACTOR} and ${MAX_K_FACTOR}`);
    }
    if (hasBonus && (body.new_player_k_bonus! < 0 || body.new_player_k_bonus! > MAX_K_FACTOR)) {
      throw invalidInput(`New player K bonus must be between 0 and ${MAX_K_FACTOR}`);
    }
    if (hasPeriod && body.new_player_bonus_period! <= 0) {
      throw invalidInput('New player bonus period must be positive');
    }

    if (getSeasonByName(db, body.name)) throw invalidInput('Season name already exists');
    if (body.elo_version != null && !eloVersionExists(body.elo_version)) {
      throw invalidInput(`ELO configuration '${body.elo_version}' does not exist`);
    }

    const season = createSeason(db, {
      name: body.name,
      description: body.description ?? null,
      start_date: toIso(body.start_date),
      starting_elo: body.starting_elo,
      k_factor: body.k_factor,
      base_k_factor: body.base_k_factor ?? null,
      new_player_k_bonus: body.new_player_k_bonus ?? null,
      new_player_bonus_period: body.new_player_bonus_period ?? null,
      elo_version: body.elo_version ?? null,
      created_by: request.user.id,
      player_ids: body.player_ids,
    });

    return mapSeason(season);
  });

  app.post<{ Params: { seasonId: string } }>(
    '/api/admin/seasons/:seasonId/activate',
    { preHandler: requireAdmin },
    async (request) => {
      const season = getSeasonById(db, request.params.seasonId);
      if (!season) throw invalidInput('Season not found');
      activateSeason(db, season.id);
      return { message: `Season '${season.name}' activated` };
    }
  );

  app.post<{ Params: { seasonId: string } }>(
    '/api/admin/seasons/:seasonId/recalculate',
    { preHandler: requireAdmin },
    async (request) => {
      const season = getSeasonById(db, request.params.seasonId);
      if (!season) throw invalidInput('Season not found');

      setImmediate(() => {
        try {
          recalculateSeasonElo(db, season.id);
        } catch (err) {
          app.log.error(err, 'Failed to recalculate season ELO');
        }
      });

      return { message: `Started ELO recalculation for season '${season.name}'` };
    }
  );

  app.patch<{ Params: { seasonId: string }; Body: { elo_version?: string | null } }>(
    '/api/admin/seasons/:seasonId/elo-version',
    { preHandler: requireAdmin },
    async (request) => {
      const eloVersion = request.body.elo_version ?? null;
      if (eloVersion != null && !eloVersionExists(eloVersion)) {
        throw invalidInput(`ELO configuration '${eloVersion}' does not exist`);
      }
      updateSeasonEloVersion(db, request.params.seasonId, eloVersion);

      const season = getSeasonById(db, request.params.seasonId);
      if (!season) throw invalidInput('Season not found');
      return mapSeason(season);
    }
  );

  app.delete<{ Params: { seasonId: string } }>(
    '/api/admin/seasons/:seasonId',
    { preHandler: requireAdmin },
    async (request) => {
      const season = getSeasonById(db, request.params.seasonId);
      if (!season) throw invalidInput('Season not found');
      deleteSeason(db, season.id);
      return {
        message: `Season '${season.name}' deleted successfully. Games reassigned and affected seasons recalculated.`,
      };
    }
  );

  app.get<{ Params: { seasonId: string } }>(
    '/api/admin/seasons/:seasonId/players',
    { preHandler: requireAdmin },
    async (request) => {
      return getSeasonPlayers(db, request.params.seasonId).map((p) => ({
        player_id: p.id,
        player_name: `${p.first_name} ${p.last_name}`,
        is_included: asBool(p.is_included),
        is_active: asBool(p.is_active),
      }));
    }
  );

  app.get<{ Params: { seasonId: string } }>(
    '/api/admin/seasons/:seasonId/available-players',
    { preHandler: requireAdmin },
    async (request) => {
      return getAvailablePlayersForSeason(db, request.params.seasonId).map((p) => ({
        player_id: p.id,
        player_name: `${p.first_name} ${p.last_name}`,
        is_included: false,
        is_active: asBool(p.is_active),
      }));
    }
  );

  app.post<{ Params: { seasonId: string }; Body: { player_id: string } }>(
    '/api/admin/seasons/:seasonId/players/add',
    { preHandler: requireAdmin },
    async (request) => {
      addPlayerToSeason(db, request.body.player_id, request.params.seasonId);
      return { message: 'Player added to season successfully' };
    }
  );

  app.post<{ Params: { seasonId: string }; Body: { player_id: string } }>(
    '/api/admin/seasons/:seasonId/players/remove',
    { preHandler: requireAdmin },
    async (request) => {
      removePlayerFromSeason(db, request.body.player_id, request.params.seasonId);
      return { message: 'Player removed from season successfully' };
    }
  );
}
