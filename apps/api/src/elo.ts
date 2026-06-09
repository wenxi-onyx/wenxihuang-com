/** ELO math, ported verbatim from the original services/elo.rs. */

export type GameWinner = 'Player1' | 'Player2';

export interface EloConfig {
  version_name: string;
  k_factor: number;
  starting_elo: number;
  base_k_factor: number | null;
  new_player_k_bonus: number | null;
  new_player_bonus_period: number | null;
}

/** K = base_k + (new_player_k_bonus * e^(-games_played / bonus_period)), else static k. */
export function dynamicKFactor(
  kFactor: number,
  baseK: number | null,
  bonus: number | null,
  period: number | null,
  gamesPlayed: number
): number {
  if (baseK != null && bonus != null && period != null && period > 0) {
    const decay = Math.exp(-gamesPlayed / period);
    return baseK + bonus * decay;
  }
  return kFactor;
}

export interface MatchEloChange {
  game_id: string;
  player1_elo_before: number;
  player1_elo_after: number;
  player1_elo_change: number;
  player2_elo_before: number;
  player2_elo_after: number;
  player2_elo_change: number;
}

/**
 * Sequential ELO for every game in a match: each game starts from the
 * previous game's resulting ratings.
 */
export function calculateMatchEloChanges(
  player1StartingElo: number,
  player2StartingElo: number,
  games: Array<{ gameId: string; winner: GameWinner }>,
  player1K: number,
  player2K: number
): MatchEloChange[] {
  let p1 = player1StartingElo;
  let p2 = player2StartingElo;
  const changes: MatchEloChange[] = [];

  for (const { gameId, winner } of games) {
    const expectedP1 = 1.0 / (1.0 + Math.pow(10, (p2 - p1) / 400.0));
    const expectedP2 = 1.0 - expectedP1;
    const [p1Score, p2Score] = winner === 'Player1' ? [1.0, 0.0] : [0.0, 1.0];

    const p1Change = player1K * (p1Score - expectedP1);
    const p2Change = player2K * (p2Score - expectedP2);
    const p1After = p1 + p1Change;
    const p2After = p2 + p2Change;

    changes.push({
      game_id: gameId,
      player1_elo_before: p1,
      player1_elo_after: p1After,
      player1_elo_change: p1Change,
      player2_elo_before: p2,
      player2_elo_after: p2After,
      player2_elo_change: p2Change,
    });

    p1 = p1After;
    p2 = p2After;
  }

  return changes;
}
