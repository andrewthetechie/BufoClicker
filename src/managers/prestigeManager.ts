import { getStateManager } from '../core/stateManager';
import { getEventBus } from '../core/eventBus';
import { PRESTIGE_TRANSCENDED } from '../core/eventTypes';
import { INITIAL_GENERATORS, GeneratorType, GeneratorData } from '../models/generators';
import {
  prestigePointsFor,
  getPrestigeMultiplier,
  PRESTIGE_MIN_TOTAL_BUFOS,
  PRESTIGE_BONUS_PER_POINT
} from '../models/prestige';
import * as Logger from '../utils/logger';
import { getGeneratorManager } from './generatorManager';
import { getUpgradeManager } from './upgradeManager';

/**
 * Handles the "Transcendence Bufoplier" prestige loop.
 *
 * Transcending performs a soft reset (bufos / generators / upgrades wiped) and
 * awards Bufoplier points based on this run's total bufos. Points are permanent
 * and multiply ALL bufo output via {@link getPrestigeMultiplier}.
 */
export class PrestigeManager {
  private static instance: PrestigeManager;

  private constructor() {}

  public static getInstance(): PrestigeManager {
    if (!PrestigeManager.instance) {
      PrestigeManager.instance = new PrestigeManager();
    }
    return PrestigeManager.instance;
  }

  /** Points the player would gain by transcending right now. */
  public getPendingPoints(): number {
    const state = getStateManager().getState();
    return prestigePointsFor(state.resources.totalBufos);
  }

  /** Current permanent multiplier from prestige (>= 1). */
  public getMultiplier(): number {
    return getPrestigeMultiplier(getStateManager().getState());
  }

  public getState() {
    const p = getStateManager().getState().prestige;
    return p ?? { points: 0, lifetimePoints: 0, transcendences: 0 };
  }

  public getBonusPerPoint(): number {
    return PRESTIGE_BONUS_PER_POINT;
  }

  public getMinTotalBufos(): number {
    return PRESTIGE_MIN_TOTAL_BUFOS;
  }

  /** Whether a transcend is currently allowed (would yield at least 1 point). */
  public canTranscend(): boolean {
    return this.getPendingPoints() >= 1;
  }

  /**
   * Perform the soft reset and bank the earned Bufoplier points.
   * @returns points gained, or 0 if the transcend was not allowed.
   */
  public transcend(): number {
    const stateManager = getStateManager();
    const state = stateManager.getState();
    const gained = prestigePointsFor(state.resources.totalBufos);

    if (gained < 1) {
      Logger.warn('Transcend attempted below the threshold — ignored');
      return 0;
    }

    const prev = state.prestige ?? { points: 0, lifetimePoints: 0, transcendences: 0 };
    const newPrestige = {
      points: prev.points + gained,
      lifetimePoints: prev.lifetimePoints + gained,
      transcendences: prev.transcendences + 1
    };

    // --- Soft reset -------------------------------------------------------
    // Generators back to their pristine JSON defaults.
    const freshGenerators = structuredClone(INITIAL_GENERATORS) as Record<GeneratorType, GeneratorData>;

    stateManager.setState({
      prestige: newPrestige,
      resources: {
        bufos: 0,
        totalBufos: 0,
        clickMultiplier: 1,
        productionMultiplier: 1,
        frenzyProductionMultiplier: 1,
        frenzyClickMultiplier: 1
      },
      generators: freshGenerators,
      upgrades: {
        purchased: [],
        available: []
      }
    });

    // Rebuild derived generator values with the (now larger) prestige multiplier.
    const generatorManager = getGeneratorManager();
    generatorManager.reset();
    generatorManager.recalculateAllGenerators();

    // Upgrade manager keeps its state in the store, which we just cleared.
    getUpgradeManager().reset();

    Logger.log(
      `Transcended! +${gained} Bufoplier points ` +
      `(lifetime ${newPrestige.lifetimePoints}, x${this.getMultiplier().toFixed(2)} forever)`
    );

    getEventBus().emit(PRESTIGE_TRANSCENDED, {
      gained,
      prestige: newPrestige,
      multiplier: this.getMultiplier()
    });

    return gained;
  }

  public reset(): void {
    // Prestige state lives in the state manager; nothing local to clear.
  }
}

export function getPrestigeManager(): PrestigeManager {
  return PrestigeManager.getInstance();
}
