import { getEventBus } from '../../core/eventBus';
import { getGameCore } from '../../game/gameCore';
import {
  GOLDEN_BUFO_SPAWNED,
  GOLDEN_BUFO_EXPIRED,
  GOLDEN_BUFO_COLLECTED
} from '../../core/eventTypes';
import type { GoldenBufoSpawn, GoldenBufoReward } from '../../managers/goldenBufoManager';

/**
 * Renders the roaming Golden Bufo and the little reward toast you get for
 * catching it. Purely presentational — all timing/reward logic lives in
 * {@link GoldenBufoManager}.
 */
export class GoldenBufo {
  private layer: HTMLElement | null = null;
  private current: HTMLElement | null = null;
  private currentId: number | null = null;

  public init(): void {
    this.layer = document.createElement('div');
    this.layer.className = 'golden-bufo-layer';
    document.body.appendChild(this.layer);

    const bus = getEventBus();
    bus.on(GOLDEN_BUFO_SPAWNED, (s: GoldenBufoSpawn) => this.showBufo(s));
    bus.on(GOLDEN_BUFO_EXPIRED, (d: { id: number }) => this.removeBufo(d.id, true));
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
    this.layer = null;
    this.current = null;
    this.currentId = null;
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
