import { getEventBus } from '../../core/eventBus';
import { getGameCore } from '../../game/gameCore';
import {
  GOLDEN_BUFO_SPAWNED,
  GOLDEN_BUFO_EXPIRED,
  GOLDEN_BUFO_COLLECTED,
  GAME_TICK
} from '../../core/eventTypes';
import type { GoldenBufoSpawn, GoldenBufoReward, ActiveFrenzy } from '../../managers/goldenBufoManager';

/**
 * Renders the roaming Golden Bufo, the reward toast you get for catching it,
 * and a countdown badge for whichever short-lived frenzy buff is currently
 * running. All timing/reward logic lives in {@link GoldenBufoManager} - this
 * just polls it every tick.
 */
export class GoldenBufo {
  private layer: HTMLElement | null = null;
  private current: HTMLElement | null = null;
  private currentId: number | null = null;
  private frenzyLayer: HTMLElement | null = null;
  private frenzyBadges: Partial<Record<'production' | 'click', HTMLElement>> = {};

  public init(): void {
    this.layer = document.createElement('div');
    this.layer.className = 'golden-bufo-layer';
    document.body.appendChild(this.layer);

    this.frenzyLayer = document.createElement('div');
    this.frenzyLayer.className = 'frenzy-indicator-layer';
    document.body.appendChild(this.frenzyLayer);

    const bus = getEventBus();
    bus.on(GOLDEN_BUFO_SPAWNED, (s: GoldenBufoSpawn) => this.showBufo(s));
    bus.on(GOLDEN_BUFO_EXPIRED, (d: { id: number }) => this.removeBufo(d.id, true));
    bus.on(GAME_TICK, () => this.refreshFrenzyIndicators());
    bus.on(GOLDEN_BUFO_COLLECTED, (r: GoldenBufoReward & { id: number }) => {
      this.removeBufo(r.id, false);
      this.showToast(r);
    });
  }

  private showBufo(spawn: GoldenBufoSpawn): void {
    if (!this.layer) return;
    this.removeBufo(this.currentId ?? -1, false);

    const el = document.createElement('button');
    el.className = 'golden-bufo';
    el.type = 'button';
    el.setAttribute('aria-label', 'Golden Bufo — click me!');
    el.style.left = `${spawn.position.xPct}vw`;
    el.style.top = `${spawn.position.yPct}vh`;
    el.style.setProperty('--ttl', `${spawn.ttl}ms`);
    el.innerHTML =
      `<img src="./assets/images/generators/bufo-has-midas-touch.png" alt="Golden Bufo" draggable="false">`;

    el.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      getGameCore().getGoldenBufoManager().collect(spawn.id);
    });

    this.layer.appendChild(el);
    this.current = el;
    this.currentId = spawn.id;
  }

  private removeBufo(id: number, fade: boolean): void {
    if (!this.current || this.currentId !== id) return;
    const el = this.current;
    this.current = null;
    this.currentId = null;

    if (fade) {
      el.classList.add('golden-bufo--leaving');
      setTimeout(() => el.remove(), 400);
    } else {
      el.remove();
    }
  }

  /** Present so UIManager's generic component teardown can call it safely. */
  public destroy(): void {
    if (this.layer && this.layer.parentNode) {
      this.layer.parentNode.removeChild(this.layer);
    }
    if (this.frenzyLayer && this.frenzyLayer.parentNode) {
      this.frenzyLayer.parentNode.removeChild(this.frenzyLayer);
    }
    this.layer = null;
    this.current = null;
    this.currentId = null;
    this.frenzyLayer = null;
    this.frenzyBadges = {};
  }

  // --- Timed buff countdown -------------------------------------------------

  private refreshFrenzyIndicators(): void {
    if (!this.frenzyLayer) return;
    const { production, click } = getGameCore().getGoldenBufoManager().getActiveFrenzies();
    this.syncFrenzyBadge('production', production, 'Bufo Frenzy', 'production');
    this.syncFrenzyBadge('click', click, 'Click Frenzy', 'click');
  }

  private syncFrenzyBadge(
    kind: 'production' | 'click',
    frenzy: ActiveFrenzy | null,
    label: string,
    modifier: string
  ): void {
    if (!this.frenzyLayer) return;

    if (!frenzy) {
      const existing = this.frenzyBadges[kind];
      if (existing) {
        existing.remove();
        delete this.frenzyBadges[kind];
      }
      return;
    }

    const remainingMs = Math.max(0, frenzy.endsAt - Date.now());
    const seconds = (remainingMs / 1000).toFixed(1);

    let badge = this.frenzyBadges[kind];
    if (!badge) {
      badge = document.createElement('div');
      badge.className = `frenzy-badge frenzy-badge--${modifier}`;
      badge.innerHTML =
        `<span class="frenzy-badge__label"></span>` +
        `<span class="frenzy-badge__time"></span>` +
        `<span class="frenzy-badge__bar"><span class="frenzy-badge__bar-fill"></span></span>`;
      this.frenzyLayer.appendChild(badge);
      this.frenzyBadges[kind] = badge;
      // Total duration isn't stored on the badge - derive the bar's starting
      // width from how far into the buff we are on first render instead of
      // tracking a separate "total duration" field.
      badge.dataset.totalMs = String(remainingMs);
    }

    const totalMs = Number(badge.dataset.totalMs) || remainingMs || 1;
    const pct = Math.max(0, Math.min(100, (remainingMs / totalMs) * 100));

    badge.querySelector('.frenzy-badge__label')!.textContent = `${label} x${frenzy.multiplier}`;
    badge.querySelector('.frenzy-badge__time')!.textContent = `${seconds}s`;
    (badge.querySelector('.frenzy-badge__bar-fill') as HTMLElement).style.width = `${pct}%`;
  }

  private showToast(reward: GoldenBufoReward): void {
    if (!this.layer) return;
    const toast = document.createElement('div');
    toast.className = `golden-bufo-toast golden-bufo-toast--${reward.rewardType}`;
    toast.innerHTML =
      `<span class="golden-bufo-toast__label">${reward.label}</span>` +
      `<span class="golden-bufo-toast__detail">${reward.detail}</span>`;
    this.layer.appendChild(toast);

    // trigger enter transition
    requestAnimationFrame(() => toast.classList.add('is-visible'));

    setTimeout(() => {
      toast.classList.remove('is-visible');
      setTimeout(() => toast.remove(), 400);
    }, 2600);
  }
}
