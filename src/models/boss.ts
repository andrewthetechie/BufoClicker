/**
 * Clicker Boss model - a sequential ladder of click-power checkpoints.
 *
 * Crossing a boss's totalBufos threshold makes it available to fight. Fighting
 * is opt-in (never ambushes the player) and gives 30 seconds to click its
 * health down to zero using nothing but click power - no bufo income during
 * the fight. Win: a permanent production/click multiplier, forever. Lose:
 * current bufos drop to zero (generators, upgrades, prestige, achievements all
 * untouched) - go upgrade your clicks and try again, no cooldown.
 */
import { GameState } from '../core/types';

export interface BossDefinition {
  id: string;
  name: string;
  flavorText: string;
  /** Total bufos ever earned needed before this boss can be challenged. */
  threshold: number;
  /** Health points - must be clicked to 0 within the fight duration. */
  maxHealth: number;
  iconPath: string;
}

/** How long a fight lasts once started, in milliseconds. */
export const BOSS_FIGHT_DURATION_MS = 30_000;

/** Permanent multiplier granted per boss defeated (stacks additively: 1 + n*bonus). */
export const BOSS_BONUS_PER_DEFEAT = 0.25; // +25% per boss

/**
 * The boss ladder. Thresholds and HP are calibrated (roughly) against the
 * click-upgrade chain in upgrades.json so each boss is beatable once you've
 * bought the click upgrades available by that point, and a real grind if you
 * haven't. Treat these as a first pass - tune after playtesting.
 *
 * The last two bosses (interdimensional_bufo, omniscient_bufo) exist because
 * the native click-upgrade chain tops out at quantum_click, well before the
 * nebula/omega/singularity generator tiers - stronger_clicks_6 and
 * omniscient_clicks (upgrades.json) were added alongside these two bosses
 * specifically so there's still a click-power wall to climb late-game
 * instead of the ladder just running out of upgrades to gate on.
 */
export const BOSSES: BossDefinition[] = [
  {
    id: 'furious_froglet',
    name: 'Furious Froglet',
    flavorText: "It's smaller than you, but it is FURIOUS about it.",
    threshold: 10_000,
    maxHealth: 1_200,
    iconPath: './assets/images/bosses/bufo-very-angry.png'
  },
  {
    id: 'the_enraged_bufo',
    name: 'The Enraged Bufo',
    flavorText: 'Every click you’ve ever made has led to this moment of pure rage.',
    threshold: 100_000_000,
    maxHealth: 6_000,
    iconPath: './assets/images/bosses/bufo-enraged.png'
  },
  {
    id: 'bufo_dragon',
    name: 'Bufo Dragon',
    flavorText: 'Legends spoke of a bufo that ascended beyond amphibian. This is it.',
    threshold: 5_000_000_000,
    maxHealth: 2_250_000,
    iconPath: './assets/images/bosses/bufo-dragon.png'
  },
  {
    id: 'bufo_devil',
    name: 'Bufo Devil',
    flavorText: 'It offers you a deal. You should probably just click it instead.',
    threshold: 100_000_000_000,
    maxHealth: 112_000_000,
    iconPath: './assets/images/bosses/bufo-devil.png'
  },
  {
    id: 'mega_bufo',
    name: 'MEGA BUFO',
    flavorText: 'The one all other bufos speak of in hushed croaks. Surely nothing tops this... right?',
    threshold: 10_000_000_000_000,
    maxHealth: 225_000_000,
    iconPath: './assets/images/bosses/mega-bufo.png'
  },
  {
    id: 'interdimensional_bufo',
    name: 'Interdimensional Bufo',
    flavorText: 'It rests atop the terrarium of existence, watching your entire pond like it were a fish tank.',
    threshold: 50_000_000_000_000,
    maxHealth: 3_000_000_000,
    iconPath: './assets/images/bosses/terrarium.png'
  },
  {
    id: 'omniscient_bufo',
    name: 'The Omniscient Bufo',
    flavorText: 'It already knows how this fight ends. Prove it wrong.',
    threshold: 2_000_000_000_000_000,
    maxHealth: 70_000_000_000,
    iconPath: './assets/images/bosses/omniscient.png'
  }
];

export function findBoss(id: string): BossDefinition | undefined {
  return BOSSES.find(b => b.id === id);
}

/**
 * The next boss the player can challenge right now: the first one in the
 * ladder they haven't defeated yet whose threshold they've met. Bosses must be
 * beaten in order.
 */
export function getAvailableBoss(
  defeated: string[],
  totalBufos: number
): BossDefinition | null {
  for (const boss of BOSSES) {
    if (defeated.includes(boss.id)) continue;
    return totalBufos >= boss.threshold ? boss : null;
  }
  return null; // all bosses defeated
}

/** Permanent multiplier from defeated bosses (>= 1). Defensive against a missing slice. */
export function getBossMultiplier(state: Pick<GameState, 'bosses'> | undefined | null): number {
  const defeated = state?.bosses?.defeated ?? [];
  return 1 + defeated.length * BOSS_BONUS_PER_DEFEAT;
}
