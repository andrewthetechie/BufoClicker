/**
 * Prestige model — the "Transcendence Bufoplier".
 *
 * Transcending is a soft reset: bufos, generators and upgrades are wiped, and in
 * return the player gains Bufoplier points based on how far they got. Every point
 * permanently boosts ALL bufo production and click power.
 */
import { GameState, PrestigeState } from '../core/types';

/** Each lifetime Bufoplier point adds this much to the global multiplier. */
export const PRESTIGE_BONUS_PER_POINT = 0.10; // +10% per point

/** You must have earned at least this many total bufos this run to transcend. */
export const PRESTIGE_MIN_TOTAL_BUFOS = 1_000_000_000; // 1 billion

/** Scale factor for the points curve (sqrt of totalBufos / this). */
const PRESTIGE_CURVE_DIVISOR = 1_000_000_000; // 1e9 -> 1 pt, 1e11 -> 10 pts, 1e13 -> 100 pts

/** Safe default prestige state. */
export const DEFAULT_PRESTIGE_STATE: PrestigeState = {
  points: 0,
  lifetimePoints: 0,
  transcendences: 0
};

/**
 * How many Bufoplier points a run worth `totalBufos` is currently worth.
 */
export function prestigePointsFor(totalBufos: number): number {
  if (!Number.isFinite(totalBufos) || totalBufos < PRESTIGE_MIN_TOTAL_BUFOS) {
    return 0;
  }
  return Math.floor(Math.sqrt(totalBufos / PRESTIGE_CURVE_DIVISOR));
}

/**
 * The permanent global multiplier granted by prestige. Never less than 1.
 * Defensive against a missing `prestige` slice (older saves / partial state).
 */
export function getPrestigeMultiplier(state: Pick<GameState, 'prestige'> | undefined | null): number {
  const lifetime = state?.prestige?.lifetimePoints ?? 0;
  return 1 + Math.max(0, lifetime) * PRESTIGE_BONUS_PER_POINT;
}
