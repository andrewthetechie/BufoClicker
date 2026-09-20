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
import { getPrestigeMultiplier } from './prestige';

export interface BossDefinition {
  id: string;
  name: string;
  flavorText: string;
  /** Total bufos ever earned needed before this boss can be challenged. */
  threshold: number;
  /**
   * Health at baseline - i.e. for a player with no prestige points and no
   * previously-defeated bosses. The real HP of a fight is this run through
   * `getBossHealth()`, which scales it by the passive multipliers the player
   * is carrying. See the note on that function for why.
   */
  baseHealth: number;
  iconPath: string;
}

/** How long a fight lasts once started, in milliseconds. */
export const BOSS_FIGHT_DURATION_MS = 30_000;

/** Permanent multiplier granted per boss defeated (stacks additively: 1 + n*bonus). */
export const BOSS_BONUS_PER_DEFEAT = 0.25; // +25% per boss

/**
 * The boss ladder. `baseHealth` is derived, not guessed: it's the click power a
 * player is expected to have at that rung - the click-upgrade chain in
 * upgrades.json times the achievement ClickBoost rewards they'll have unlocked
 * by then - multiplied by how many clicks the fight should take. The targets
 * ramp from 65 clicks for the opener to 165 for the final boss.
 *
 * Prestige and previously-defeated bosses are deliberately NOT in that base
 * figure; `getBossHealth()` multiplies them back in at fight time. See its
 * note.
 *
 * The click targets are set against a real human rate of 4-8 clicks/sec, and
 * priced for the fact that the sprite hops every MOVE_INTERVAL_MS with a 0.4s
 * glide - roughly 15% of a 30-second fight goes on reacquiring it rather than
 * clicking, leaving ~25.5 effective seconds. So the targets below need 2.5
 * clicks/sec at the opener, rising to 6.5 for the final boss: the early rungs
 * sit under the band as a formality, the back half sits inside it, and
 * nothing requires a rate only the top of the band can hit.
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
    baseHealth: 975,
    iconPath: './assets/images/bosses/bufo-very-angry.png'
  },
  {
    id: 'the_enraged_bufo',
    name: 'The Enraged Bufo',
    flavorText: 'Every click you’ve ever made has led to this moment of pure rage.',
    threshold: 100_000_000,
    baseHealth: 53_800,
    iconPath: './assets/images/bosses/bufo-enraged.png'
  },
  {
    id: 'bufo_dragon',
    name: 'Bufo Dragon',
    flavorText: 'Legends spoke of a bufo that ascended beyond amphibian. This is it.',
    threshold: 5_000_000_000,
    baseHealth: 38_700_000,
    iconPath: './assets/images/bosses/bufo-dragon.png'
  },
  {
    id: 'bufo_devil',
    name: 'Bufo Devil',
    flavorText: 'It offers you a deal. You should probably just click it instead.',
    threshold: 100_000_000_000,
    baseHealth: 65_600_000,
    iconPath: './assets/images/bosses/bufo-devil.png'
  },
  {
    id: 'mega_bufo',
    name: 'MEGA BUFO',
    flavorText: 'The one all other bufos speak of in hushed croaks. Surely nothing tops this... right?',
    threshold: 10_000_000_000_000,
    baseHealth: 7_070_000_000,
    iconPath: './assets/images/bosses/mega-bufo.png'
  },
  {
    id: 'interdimensional_bufo',
    name: 'Interdimensional Bufo',
    flavorText: 'It rests atop the terrarium of existence, watching your entire pond like it were a fish tank.',
    threshold: 50_000_000_000_000,
    baseHealth: 78_200_000_000,
    iconPath: './assets/images/bosses/terrarium.png'
  },
  {
    id: 'omniscient_bufo',
    name: 'The Omniscient Bufo',
    flavorText: 'It already knows how this fight ends. Prove it wrong.',
    threshold: 2_000_000_000_000_000,
    baseHealth: 1_670_000_000_000,
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

/**
 * Permanent multiplier from defeated bosses (>= 1). Counts this run's defeats
 * plus every previous run's, so transcending re-opens the ladder (and resets
 * the background) without ever taking back a multiplier you already earned.
 * Defensive against a missing slice.
 */
export function getBossMultiplier(state: Pick<GameState, 'bosses'> | undefined | null): number {
  const defeated = state?.bosses?.defeated ?? [];
  const lifetime = state?.bosses?.lifetimeDefeats ?? 0;
  return 1 + (defeated.length + lifetime) * BOSS_BONUS_PER_DEFEAT;
}

/**
 * The real health of a fight against `boss` for the player in `state`.
 *
 * A fixed number cannot work here. Boss damage is `resources.clickPower`, which
 * is `baseClickPower x clickMultiplier x prestige x bossBonus x clickFrenzy` -
 * and two players sitting on the same `totalBufos` can differ by a factor of a
 * thousand in that product depending on how often they've transcended. The
 * original ladder was calibrated against the click-upgrade chain alone, so the
 * moment a player prestiged even once, every boss became a one- or two-click
 * kill.
 *
 * So the ladder normalises out the power the player did NOT earn by climbing
 * the ladder itself:
 *
 *  - `prestige` is multiplied back into HP. Transcending is meant to speed up
 *    production, not delete the click checkpoints.
 *  - `bossBonus` is multiplied back in too. It counts lifetime defeats, so
 *    without this a second-run player would restart the ladder already holding
 *    a 2.75x head start over the numbers it was tuned for.
 *
 * What stays a real advantage, by design:
 *
 *  - The click-upgrade chain. That IS the ladder's progression axis - buying
 *    the next click upgrade is exactly how you beat the next boss.
 *  - Achievement ClickBoost rewards, which are baked into `baseHealth` at the
 *    rung where they're expected, so unlocking them early pays off.
 *  - Golden Bufo's Click Frenzy, which is not normalised out at all. Saving a
 *    frenzy for a boss is a genuine (and deliberate) strategy.
 *
 * Health is read once when the fight starts, so a frenzy expiring mid-fight
 * can't move the goalposts.
 */
export function getBossHealth(
  boss: BossDefinition,
  state: Pick<GameState, 'bosses' | 'prestige'> | undefined | null
): number {
  const passive = getPrestigeMultiplier(state) * getBossMultiplier(state);
  return Math.max(1, Math.ceil(boss.baseHealth * passive));
}
