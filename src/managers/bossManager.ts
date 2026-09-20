import { getStateManager } from '../core/stateManager';
import { getEventBus } from '../core/eventBus';
import {
  BOSS_FIGHT_STARTED,
  BOSS_DAMAGED,
  BOSS_TICK,
  BOSS_DEFEATED,
  BOSS_FIGHT_LOST,
  BOSS_FIGHT_RETREATED
} from '../core/eventTypes';
import {
  BossDefinition,
  BOSS_FIGHT_DURATION_MS,
  getAvailableBoss,
  getBossHealth,
  getBossMultiplier
} from '../models/boss';
import * as Logger from '../utils/logger';
import { getGeneratorManager } from './generatorManager';

export interface ActiveBossFight {
  boss: BossDefinition;
  health: number;
  /** Health this fight started at - `boss.baseHealth` scaled for this player
   *  by `getBossHealth()`. Read once at `startFight()` so nothing can move the
   *  goalposts mid-fight; always use this rather than the definition's
   *  `baseHealth` for HP bars and "x / y HP" readouts. */
  maxHealth: number;
  remainingMs: number;
}

const TICK_MS = 100;

/**
 * Runs Clicker Boss fights: a sequential ladder of click-power checkpoints.
 * A fight is entirely opt-in (never starts on its own) and entirely paused
 * while the game is paused (tab hidden), so the punishing loss condition can
 * never trigger without the player actively watching it happen.
 */
export class BossManager {
  private static instance: BossManager;

  private fight: ActiveBossFight | null = null;
  private intervalId: number | null = null;

  private constructor() {}

  public static getInstance(): BossManager {
    if (!BossManager.instance) {
      BossManager.instance = new BossManager();
    }
    return BossManager.instance;
  }

  /** The boss currently challengeable, or null if none is available right now. */
  public getAvailableBoss(): BossDefinition | null {
    const state = getStateManager().getState();
    return getAvailableBoss(state.bosses?.defeated ?? [], state.resources.totalBufos);
  }

  public getActiveFight(): ActiveBossFight | null {
    return this.fight;
  }

  /** What `boss` would actually have to be clicked down from right now. */
  public getScaledHealth(boss: BossDefinition): number {
    return getBossHealth(boss, getStateManager().getState());
  }

  public getMultiplier(): number {
    return getBossMultiplier(getStateManager().getState());
  }

  public getDefeatedCount(): number {
    return getStateManager().getState().bosses?.defeated.length ?? 0;
  }

  /** Start a fight against the currently-available boss. No-op if none is available or one is already running. */
  public startFight(): boolean {
    if (this.fight) return false;
    const boss = this.getAvailableBoss();
    if (!boss) return false;

    const maxHealth = getBossHealth(boss, getStateManager().getState());
    this.fight = { boss, health: maxHealth, maxHealth, remainingMs: BOSS_FIGHT_DURATION_MS };
    this.runTimer();

    getEventBus().emit(BOSS_FIGHT_STARTED, { boss, maxHealth });
    Logger.log(`Boss fight started: ${boss.name} (${maxHealth} HP, ${BOSS_FIGHT_DURATION_MS / 1000}s)`);
    return true;
  }

  /**
   * Deal damage equal to the player's current click power (no combo, and this
   * never touches bufos - clicking the boss is the fight, not income).
   * @returns the fight state after the hit, or null if there's no active fight.
   */
  public hit(amount: number): ActiveBossFight | null {
    if (!this.fight || amount <= 0) return this.fight;

    this.fight.health = Math.max(0, this.fight.health - amount);
    getEventBus().emit(BOSS_DAMAGED, {
      boss: this.fight.boss,
      health: this.fight.health,
      maxHealth: this.fight.maxHealth,
      damage: amount
    });

    if (this.fight.health <= 0) {
      this.win();
    }

    return this.fight;
  }

  /** Back out of a fight in progress with no penalty. */
  public retreat(): void {
    if (!this.fight) return;
    const boss = this.fight.boss;
    this.clearTimer();
    this.fight = null;
    getEventBus().emit(BOSS_FIGHT_RETREATED, { boss });
  }

  /** Pause the countdown (game paused / tab hidden). Health and remaining time are preserved. */
  public pause(): void {
    this.clearTimer();
  }

  /** Resume the countdown from wherever it left off. */
  public resume(): void {
    if (this.fight && this.intervalId === null) {
      this.runTimer();
    }
  }

  public reset(): void {
    this.clearTimer();
    this.fight = null;
  }

  // ---------------------------------------------------------------------------

  private runTimer(): void {
    this.clearTimer();
    this.intervalId = window.setInterval(() => {
      if (!this.fight) {
        this.clearTimer();
        return;
      }

      this.fight.remainingMs -= TICK_MS;
      getEventBus().emit(BOSS_TICK, {
        boss: this.fight.boss,
        remainingMs: Math.max(0, this.fight.remainingMs)
      });

      if (this.fight.remainingMs <= 0) {
        this.lose();
      }
    }, TICK_MS);
  }

  private clearTimer(): void {
    if (this.intervalId !== null) {
      window.clearInterval(this.intervalId);
      this.intervalId = null;
    }
  }

  private win(): void {
    if (!this.fight) return;
    const boss = this.fight.boss;
    this.clearTimer();
    this.fight = null;

    const stateManager = getStateManager();
    const state = stateManager.getState();
    const defeated = [...(state.bosses?.defeated ?? []), boss.id];
    stateManager.setState({ bosses: { defeated } });

    // Every generator's effective production just changed (new permanent multiplier).
    getGeneratorManager().recalculateAllGenerators();

    Logger.log(`Boss defeated: ${boss.name}! Permanent multiplier is now x${this.getMultiplier().toFixed(2)}`);
    getEventBus().emit(BOSS_DEFEATED, {
      boss,
      defeatedCount: defeated.length,
      multiplier: this.getMultiplier()
    });
  }

  private lose(): void {
    if (!this.fight) return;
    const boss = this.fight.boss;
    this.clearTimer();
    this.fight = null;

    // The one penalty: current bufos on hand. Everything else - generators,
    // upgrades, prestige, achievements, other defeated bosses - is untouched.
    const stateManager = getStateManager();
    stateManager.setState({ resources: { bufos: 0 } });

    Logger.log(`Boss fight lost: ${boss.name}. Bufos reset to 0.`);
    getEventBus().emit(BOSS_FIGHT_LOST, { boss });
  }
}

export function getBossManager(): BossManager {
  return BossManager.getInstance();
}
