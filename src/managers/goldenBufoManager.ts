import { getStateManager } from '../core/stateManager';
import { getEventBus } from '../core/eventBus';
import {
  GOLDEN_BUFO_SPAWNED,
  GOLDEN_BUFO_EXPIRED,
  GOLDEN_BUFO_COLLECTED
} from '../core/eventTypes';
import { getGeneratorManager } from './generatorManager';
import { formatNumber } from '../utils/numberUtils';
import * as Logger from '../utils/logger';

export type GoldenBufoRewardType = 'bufo_frenzy' | 'click_frenzy' | 'lucky';

export interface GoldenBufoSpawn {
  id: number;
  rewardType: GoldenBufoRewardType;
  /** Position as viewport percentages, kept away from the edges. */
  position: { xPct: number; yPct: number };
  /** How long (ms) the bufo stays clickable. */
  ttl: number;
}

export interface GoldenBufoReward {
  rewardType: GoldenBufoRewardType;
  /** Short headline, e.g. "Bufo Frenzy!" */
  label: string;
  /** One-line description of what the player got. */
  detail: string;
}

export interface ActiveFrenzy {
  multiplier: number;
  /** Wall-clock timestamp (ms) this buff expires at. */
  endsAt: number;
}

// Timing (ms)
const FIRST_SPAWN_MIN = 45_000;
const FIRST_SPAWN_MAX = 90_000;
const SPAWN_MIN = 90_000;
const SPAWN_MAX = 190_000;
const ON_SCREEN_TTL = 13_000;

const BUFO_FRENZY_MULT = 7;
const BUFO_FRENZY_MS = 30_000;
const CLICK_FRENZY_MULT = 7;
const CLICK_FRENZY_MS = 15_000;

const rand = (min: number, max: number) => min + Math.random() * (max - min);

/**
 * Spawns a clickable "Golden Bufo" at random intervals. Clicking it grants a
 * short, powerful buff (production frenzy / click frenzy) or an instant windfall.
 */
export class GoldenBufoManager {
  private static instance: GoldenBufoManager;

  private running = false;
  private nextId = 1;
  private spawnTimer: number | null = null;
  private expireTimer: number | null = null;
  private productionFrenzyTimer: number | null = null;
  private clickFrenzyTimer: number | null = null;
  /** 0 = not active. Wall-clock timestamp, so the UI can compute "time left". */
  private productionFrenzyEndsAt = 0;
  private clickFrenzyEndsAt = 0;
  private active: GoldenBufoSpawn | null = null;
  private firstSpawnDone = false;

  private constructor() {}

  public static getInstance(): GoldenBufoManager {
    if (!GoldenBufoManager.instance) {
      GoldenBufoManager.instance = new GoldenBufoManager();
    }
    return GoldenBufoManager.instance;
  }

  public start(): void {
    if (this.running) return;
    this.running = true;
    this.scheduleNextSpawn();
    Logger.debug('GoldenBufoManager started');
  }

  public stop(): void {
    if (!this.running) return;
    this.running = false;

    this.clearTimer('spawnTimer');
    this.clearTimer('expireTimer');

    if (this.active) {
      const expired = this.active;
      this.active = null;
      getEventBus().emit(GOLDEN_BUFO_EXPIRED, { id: expired.id });
    }

    // End any running frenzy — buffs don't persist while the game is paused.
    this.clearTimer('productionFrenzyTimer');
    this.clearTimer('clickFrenzyTimer');
    this.productionFrenzyEndsAt = 0;
    this.clickFrenzyEndsAt = 0;
    this.setFrenzy({ frenzyProductionMultiplier: 1, frenzyClickMultiplier: 1 });

    Logger.debug('GoldenBufoManager stopped');
  }

  /** Is a Golden Bufo on screen right now? */
  public isActive(): boolean {
    return this.active !== null;
  }

  /** Any short-lived buffs currently running, for the UI to render a countdown for. */
  public getActiveFrenzies(): { production: ActiveFrenzy | null; click: ActiveFrenzy | null } {
    const now = Date.now();
    return {
      production:
        this.productionFrenzyEndsAt > now
          ? { multiplier: BUFO_FRENZY_MULT, endsAt: this.productionFrenzyEndsAt }
          : null,
      click:
        this.clickFrenzyEndsAt > now
          ? { multiplier: CLICK_FRENZY_MULT, endsAt: this.clickFrenzyEndsAt }
          : null
    };
  }

  /**
   * Called by the UI when the player clicks the Golden Bufo.
   * @returns the reward that was applied, or null if there was nothing to click.
   */
  public collect(id?: number): GoldenBufoReward | null {
    if (!this.active || (id !== undefined && id !== this.active.id)) {
      return null;
    }

    const spawn = this.active;
    this.active = null;
    this.clearTimer('expireTimer');

    const reward = this.applyReward(spawn.rewardType);

    getEventBus().emit(GOLDEN_BUFO_COLLECTED, { id: spawn.id, ...reward });
    Logger.log(`Golden Bufo collected: ${reward.label} — ${reward.detail}`);

    this.scheduleNextSpawn();
    return reward;
  }

  public reset(): void {
    this.stop();
    this.firstSpawnDone = false;
  }

  /**
   * Force a spawn right now, bypassing the random timer. Used by the debug
   * console (`debugTools.golden.spawn()`) so the event can be tested without
   * waiting; harmless to call in production (a manual "why not").
   */
  public forceSpawn(): void {
    if (!this.running) this.start();
    this.clearTimer('spawnTimer');
    if (this.active) return; // one at a time
    this.spawn();
  }

  // ---------------------------------------------------------------------------

  private scheduleNextSpawn(): void {
    if (!this.running) return;
    this.clearTimer('spawnTimer');

    const delay = this.firstSpawnDone
      ? rand(SPAWN_MIN, SPAWN_MAX)
      : rand(FIRST_SPAWN_MIN, FIRST_SPAWN_MAX);

    this.spawnTimer = window.setTimeout(() => this.spawn(), delay);
  }

  private spawn(): void {
    if (!this.running || this.active) {
      this.scheduleNextSpawn();
      return;
    }
    this.firstSpawnDone = true;

    const roll = Math.random();
    const rewardType: GoldenBufoRewardType =
      roll < 0.5 ? 'bufo_frenzy' : roll < 0.8 ? 'lucky' : 'click_frenzy';

    const spawn: GoldenBufoSpawn = {
      id: this.nextId++,
      rewardType,
      position: { xPct: rand(8, 84), yPct: rand(14, 78) },
      ttl: ON_SCREEN_TTL
    };
    this.active = spawn;

    getEventBus().emit(GOLDEN_BUFO_SPAWNED, spawn);

    this.expireTimer = window.setTimeout(() => {
      if (this.active && this.active.id === spawn.id) {
        this.active = null;
        getEventBus().emit(GOLDEN_BUFO_EXPIRED, { id: spawn.id });
        this.scheduleNextSpawn();
      }
    }, spawn.ttl);
  }

  private applyReward(type: GoldenBufoRewardType): GoldenBufoReward {
    const stateManager = getStateManager();

    switch (type) {
      case 'bufo_frenzy': {
        this.clearTimer('productionFrenzyTimer');
        this.setFrenzy({ frenzyProductionMultiplier: BUFO_FRENZY_MULT });
        getGeneratorManager().recalculateAllGenerators();
        this.productionFrenzyEndsAt = Date.now() + BUFO_FRENZY_MS;

        this.productionFrenzyTimer = window.setTimeout(() => {
          this.setFrenzy({ frenzyProductionMultiplier: 1 });
          getGeneratorManager().recalculateAllGenerators();
          this.productionFrenzyTimer = null;
          this.productionFrenzyEndsAt = 0;
        }, BUFO_FRENZY_MS);

        return {
          rewardType: type,
          label: 'Bufo Frenzy!',
          detail: `x${BUFO_FRENZY_MULT} production for ${BUFO_FRENZY_MS / 1000}s`
        };
      }

      case 'click_frenzy': {
        this.clearTimer('clickFrenzyTimer');
        this.setFrenzy({ frenzyClickMultiplier: CLICK_FRENZY_MULT });
        this.clickFrenzyEndsAt = Date.now() + CLICK_FRENZY_MS;

        this.clickFrenzyTimer = window.setTimeout(() => {
          this.setFrenzy({ frenzyClickMultiplier: 1 });
          this.clickFrenzyTimer = null;
          this.clickFrenzyEndsAt = 0;
        }, CLICK_FRENZY_MS);

        return {
          rewardType: type,
          label: 'Click Frenzy!',
          detail: `x${CLICK_FRENZY_MULT} click power for ${CLICK_FRENZY_MS / 1000}s`
        };
      }

      case 'lucky':
      default: {
        const state = stateManager.getState();
        const perSecond = getGeneratorManager().calculateTotalProduction();
        // 15% of the bank, capped at 20 minutes of production, +13 for luck.
        const gain = Math.floor(
          Math.min(state.resources.bufos * 0.15, perSecond * 60 * 20) + 13
        );
        stateManager.setState({
          resources: {
            bufos: state.resources.bufos + gain,
            totalBufos: state.resources.totalBufos + gain
          }
        });
        return {
          rewardType: 'lucky',
          label: 'Lucky!',
          detail: `+${formatNumber(gain)} bufos`
        };
      }
    }
  }

  private setFrenzy(patch: { frenzyProductionMultiplier?: number; frenzyClickMultiplier?: number }): void {
    getStateManager().setState({ resources: patch });
  }

  private clearTimer(
    key: 'spawnTimer' | 'expireTimer' | 'productionFrenzyTimer' | 'clickFrenzyTimer'
  ): void {
    const id = this[key];
    if (id !== null) {
      window.clearTimeout(id);
      this[key] = null;
    }
  }
}

export function getGoldenBufoManager(): GoldenBufoManager {
  return GoldenBufoManager.getInstance();
}
