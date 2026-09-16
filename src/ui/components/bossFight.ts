import { getEventBus } from '../../core/eventBus';
import { getStateManager } from '../../core/stateManager';
import { getGameCore } from '../../game/gameCore';
import { saveGame } from '../../game/gameSave';
import { getUIManager } from '../../managers/UIManager';
import { formatNumber } from '../../utils/numberUtils';
import {
  BOSS_FIGHT_STARTED,
  BOSS_DAMAGED,
  BOSS_TICK,
  BOSS_DEFEATED,
  BOSS_FIGHT_LOST,
  BOSS_FIGHT_RETREATED,
  GAME_TICK
} from '../../core/eventTypes';
import type { BossDefinition } from '../../models/boss';
import type { ActiveBossFight } from '../../managers/bossManager';

const MOVE_INTERVAL_MS = 1200;

/**
 * Clicker Boss UI: a banner announcing an available boss, and - once the
 * player opts in - a full-screen fight with a roaming, clickable boss sprite,
 * a health bar and a countdown. Purely presentational; all rules live in
 * BossManager.
 */
export class BossFight {
  private layer: HTMLElement | null = null;
  private banner: HTMLElement | null = null;
  private overlay: HTMLElement | null = null;
  private healthFill: HTMLElement | null = null;
  private healthText: HTMLElement | null = null;
  private timerText: HTMLElement | null = null;
  private spriteEl: HTMLElement | null = null;
  private moveTimer: number | null = null;
  private shownBossId: string | null = null;
  private snoozedUntil = 0;

  public init(): void {
    this.layer = document.createElement('div');
    this.layer.className = 'boss-layer';
    document.body.appendChild(this.layer);

    // Restore the world's visual stage from already-defeated bosses (page load / save restore).
    document.body.dataset.bossStage = String(getGameCore().getBossManager().getDefeatedCount());

    const bus = getEventBus();
    bus.on(GAME_TICK, () => this.refreshBanner());
    bus.on(BOSS_FIGHT_STARTED, (d: { boss: BossDefinition }) => this.showFight(d.boss));
    bus.on(BOSS_DAMAGED, (d: { health: number; maxHealth: number }) => this.updateHealth(d.health, d.maxHealth));
    bus.on(BOSS_TICK, (d: { remainingMs: number }) => this.updateTimer(d.remainingMs));
    bus.on(BOSS_DEFEATED, (d: { boss: BossDefinition; defeatedCount: number; multiplier: number }) => this.onDefeated(d));
    bus.on(BOSS_FIGHT_LOST, (d: { boss: BossDefinition }) => this.onLost(d.boss));
    bus.on(BOSS_FIGHT_RETREATED, () => this.teardownFight());

    this.refreshBanner();
  }

  public destroy(): void {
    this.clearMoveTimer();
    if (this.layer && this.layer.parentNode) {
      this.layer.parentNode.removeChild(this.layer);
    }
    this.layer = null;
  }

  // --- Banner: "a boss is available, opt in when ready" --------------------

  private refreshBanner(): void {
    if (!this.layer) return;
    // Never show the banner while a fight is already running.
    if (getGameCore().getBossManager().getActiveFight()) return;
    // "Not yet" snoozes the banner for a while instead of it reappearing next tick.
    if (Date.now() < this.snoozedUntil) return;

    const boss = getGameCore().getBossManager().getAvailableBoss();

    if (!boss) {
      if (this.banner) {
        this.banner.remove();
        this.banner = null;
      }
      return;
    }

    if (this.banner && this.shownBossId === boss.id) return; // already showing this one

    if (this.banner) this.banner.remove();
    this.shownBossId = boss.id;

    const banner = document.createElement('div');
    banner.className = 'boss-banner';
    banner.innerHTML = `
      <img class="boss-banner__portrait" src="${boss.iconPath}" alt="${boss.name}">
      <div class="boss-banner__body">
        <div class="boss-banner__title">A Boss Has Appeared!</div>
        <div class="boss-banner__name">${boss.name}</div>
        <div class="boss-banner__flavor">${boss.flavorText}</div>
      </div>
      <div class="boss-banner__actions">
        <button type="button" class="boss-banner__fight">Fight! (${formatNumber(boss.maxHealth)} HP, 30s)</button>
        <button type="button" class="boss-banner__later">Not yet</button>
      </div>
    `;

    banner.querySelector('.boss-banner__fight')!.addEventListener('click', () => {
      getGameCore().getBossManager().startFight();
    });
    banner.querySelector('.boss-banner__later')!.addEventListener('click', () => {
      banner.remove();
      this.banner = null;
      this.shownBossId = null;
      this.snoozedUntil = Date.now() + (60_000 + Math.random() * 120_000);
    });

    this.layer.appendChild(banner);
    this.banner = banner;
  }

  // --- Active fight ----------------------------------------------------------

  private showFight(boss: BossDefinition): void {
    if (!this.layer) return;

    if (this.banner) {
      this.banner.remove();
      this.banner = null;
      this.shownBossId = null;
    }

    const overlay = document.createElement('div');
    overlay.className = 'boss-fight-overlay';
    overlay.innerHTML = `
      <div class="boss-fight-hud">
        <div class="boss-fight-hud__name">${boss.name}</div>
        <div class="boss-health-bar">
          <div class="boss-health-bar__fill"></div>
          <div class="boss-health-bar__text"></div>
        </div>
        <div class="boss-fight-hud__timer">30.0s</div>
        <button type="button" class="boss-fight-hud__retreat">Retreat</button>
      </div>
    `;
    this.layer.appendChild(overlay);
    this.overlay = overlay;
    this.healthFill = overlay.querySelector('.boss-health-bar__fill');
    this.healthText = overlay.querySelector('.boss-health-bar__text');
    this.timerText = overlay.querySelector('.boss-fight-hud__timer');
    this.updateHealth(boss.maxHealth, boss.maxHealth);
    this.updateTimer(30_000);

    overlay.querySelector('.boss-fight-hud__retreat')!.addEventListener('click', () => {
      getGameCore().getBossManager().retreat();
    });

    document.body.classList.add('boss-fight-active');

    const sprite = document.createElement('button');
    sprite.type = 'button';
    sprite.className = 'boss-sprite';
    sprite.setAttribute('aria-label', `Click ${boss.name}`);
    sprite.innerHTML = `<img src="${boss.iconPath}" alt="${boss.name}" draggable="false">`;
    sprite.addEventListener('click', (e) => this.handleHit(e));
    // Rapid clicking on a moving target was starting native drag/text-select
    // gestures; CSS (user-select: none) stops the selection itself, this
    // stops the ghost-image drag some browsers still try to start.
    sprite.addEventListener('mousedown', (e) => e.preventDefault());
    sprite.addEventListener('dragstart', (e) => e.preventDefault());
    overlay.appendChild(sprite);
    this.spriteEl = sprite;

    this.repositionSprite();
    this.moveTimer = window.setInterval(() => this.repositionSprite(), MOVE_INTERVAL_MS);
  }

  private repositionSprite(): void {
    if (!this.spriteEl) return;
    const x = 10 + Math.random() * 75; // vw, kept off the very edges
    const y = 15 + Math.random() * 65; // vh
    this.spriteEl.style.left = `${x}vw`;
    this.spriteEl.style.top = `${y}vh`;
  }

  private handleHit(e: MouseEvent): void {
    e.preventDefault();
    e.stopPropagation();

    const clickPower = getStateManager().getState().resources.clickPower;
    const fight = getGameCore().getBossManager().hit(clickPower);
    if (!fight) return; // fight ended (defeated) - event handlers take it from here

    // Damage number, reusing the same "pop" feel as normal clicks but red/aggressive.
    if (this.spriteEl) {
      const rect = this.spriteEl.getBoundingClientRect();
      this.showDamageNumber(rect.left + rect.width / 2, rect.top, clickPower);
      this.spriteEl.classList.remove('boss-sprite--hit');
      // eslint-disable-next-line no-void
      void this.spriteEl.offsetWidth; // restart the hit animation
      this.spriteEl.classList.add('boss-sprite--hit');
    }
  }

  private showDamageNumber(x: number, y: number, amount: number): void {
    if (!this.overlay) return;
    const el = document.createElement('div');
    el.className = 'boss-damage-number';
    el.textContent = `-${formatNumber(amount)}`;
    el.style.left = `${x}px`;
    el.style.top = `${y}px`;
    this.overlay.appendChild(el);
    setTimeout(() => el.remove(), 700);
  }

  private updateHealth(health: number, maxHealth: number): void {
    if (!this.healthFill || !this.healthText) return;
    const pct = maxHealth > 0 ? Math.max(0, Math.min(100, (health / maxHealth) * 100)) : 0;
    this.healthFill.style.width = `${pct}%`;
    this.healthText.textContent = `${formatNumber(health)} / ${formatNumber(maxHealth)} HP`;
  }

  private updateTimer(remainingMs: number): void {
    if (!this.timerText) return;
    const seconds = Math.max(0, remainingMs / 1000);
    this.timerText.textContent = `${seconds.toFixed(1)}s`;
    this.timerText.classList.toggle('boss-fight-hud__timer--urgent', seconds <= 10);
  }

  private onDefeated(d: { boss: BossDefinition; defeatedCount: number; multiplier: number }): void {
    this.teardownFight();
    document.body.dataset.bossStage = String(d.defeatedCount);
    saveGame(true);

    getUIManager().showModal({
      id: 'boss-victory-modal',
      title: `${d.boss.name} Defeated!`,
      content: `
        <div class="boss-result boss-result--win">
          <img class="boss-result__portrait" src="${d.boss.iconPath}" alt="${d.boss.name}">
          <p>Your bufos will remember this croak for generations.</p>
          <p class="boss-result__area">The world around you has changed - you've entered a new area.</p>
          <p class="boss-result__reward">Permanent multiplier is now <strong>x${d.multiplier.toFixed(2)}</strong> to all bufo production and click power.</p>
        </div>
      `,
      buttons: [
        { text: 'Nice!', callback: () => getUIManager().closeModal(), className: 'modal-button confirm-button' }
      ],
      closeOnBackdrop: true
    });
  }

  private onLost(boss: BossDefinition): void {
    this.teardownFight();
    saveGame(true);

    getUIManager().showModal({
      id: 'boss-defeat-modal',
      title: `Defeated by ${boss.name}...`,
      content: `
        <div class="boss-result boss-result--lose">
          <img class="boss-result__portrait" src="${boss.iconPath}" alt="${boss.name}">
          <p>Your bufos scatter, and your bufo stash resets to 0.</p>
          <p class="boss-result__hint">Your generators, upgrades and prestige are untouched - buy some more click upgrades and try again whenever you're ready.</p>
        </div>
      `,
      buttons: [
        { text: 'Try again later', callback: () => getUIManager().closeModal(), className: 'modal-button cancel-button' }
      ],
      closeOnBackdrop: true
    });
  }

  private teardownFight(): void {
    this.clearMoveTimer();
    document.body.classList.remove('boss-fight-active');
    if (this.overlay) {
      this.overlay.remove();
      this.overlay = null;
    }
    this.spriteEl = null;
    this.healthFill = null;
    this.healthText = null;
    this.timerText = null;
    // A new boss (or none) may now be relevant; let the next tick re-check.
    this.refreshBanner();
  }

  private clearMoveTimer(): void {
    if (this.moveTimer !== null) {
      window.clearInterval(this.moveTimer);
      this.moveTimer = null;
    }
  }
}
