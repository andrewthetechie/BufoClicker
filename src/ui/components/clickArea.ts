import { Component } from '../core/Component';
import { ComponentOptions } from '../core/types';
import { getGameCore } from '../../game/gameCore';

/**
 * Keep a viewport-positioned effect on screen.
 * @param x Desired left edge, in viewport coordinates
 * @param width Element width, so the right edge is accounted for
 * @param margin Minimum gap to keep from either edge
 */
function clampToViewport(x: number, width: number, margin: number = 6): number {
  const max = window.innerWidth - width - margin;
  // A viewport narrower than the element itself would invert the range.
  return max < margin ? margin : Math.max(margin, Math.min(x, max));
}

export interface ClickAreaOptions extends ComponentOptions {
  /** Path to bufo image */
  imagePath?: string;
  /** Max clicks to track for combo */
  maxComboClicks?: number;
  /** Click combo timeout in ms */
  comboTimeout?: number;
}

/**
 * Interface for click result
 */
interface ClickResult {
  bufosGained: number;
  isCombo: boolean;
  comboMultiplier: number;
}

/**
 * Click area component for the main bufo clicking interaction
 */
export class ClickArea extends Component {
  /** Path to bufo image */
  private imagePath: string;
  /** Max clicks to track */
  private maxComboClicks: number;
  /** Bufo image element */
  private bufoImage: HTMLImageElement | null = null;
  /** Click combo timeout */
  private comboTimeout: number;
  /** Recent clicks timestamps */
  private recentClicks: number[] = [];
  /** Timer for clearing clicks */
  private clearClicksTimer: number | null = null;
  /** Timer that resets the bufo squish transform */
  private squishResetTimer: number | null = null;

  /**
   * Click effects (ripple, floating number, bufo pop) render into this
   * body-level fixed layer rather than into the click area itself.
   *
   * They used to be absolutely positioned children of `.frog-display`, which
   * meant an effect spawned near an edge got clipped: `.column` sets
   * `overflow-y: auto`, and per CSS a non-visible overflow on one axis forces
   * the other to compute as `auto` too, so the column clips horizontally as
   * well. Clicking the right-hand side of the bufo cut the "+N" label in half.
   * A viewport-level layer has no clipping ancestor, and it also lets the
   * effects (the bufo pop drifts up to 70px sideways) spill past the box the
   * way they're supposed to.
   */
  private effectLayer: HTMLElement | null = null;

  /**
   * Create a click area component
   */
  constructor(options: ClickAreaOptions = {}) {
    super({
      id: options.id || 'frog-display',
      className: options.className || 'frog-display',
      ...options
    });

    this.imagePath = options.imagePath || './assets/images/bufo.png';
    this.maxComboClicks = options.maxComboClicks || 5;
    this.comboTimeout = options.comboTimeout || 1000;
  }

  /**
   * Set up the component
   */
  protected setup(): void {
    // Create initial structure
    this.setContent(this.render());

    // Find bufo image
    if (this.element) {
      this.bufoImage = this.element.querySelector('.bufo-image');
    }

    if (!this.effectLayer) {
      this.effectLayer = document.createElement('div');
      this.effectLayer.className = 'click-effect-layer';
      document.body.appendChild(this.effectLayer);
    }

    // Add click event listeners
    if (this.element) {
      this.element.addEventListener('click', this.handleClick.bind(this));
    }

    // Set up click cleaner
    this.clearClicksTimer = window.setInterval(() => {
      this.clearOldClicks();
    }, 1000);
  }

  /**
   * Handle click on the bufo
   */
  private handleClick(e: MouseEvent): void {
    e.preventDefault();

    // NOTE: no artificial click cooldown here. A previous 50ms gate silently
    // dropped every click past ~20/sec, and a stuck animation latch could make
    // clicks appear to stop registering entirely. Real pointer clicks are
    // already rate-limited by the browser; `getGameCore().click()` is cheap.

    // Track click for combo
    this.recentClicks.push(Date.now());
    if (this.recentClicks.length > this.maxComboClicks) {
      this.recentClicks.shift();
    }

    // Trigger game click and get result
    const clickResult = getGameCore().click();

    // Viewport coordinates - the effects live in a fixed, body-level layer.
    this.createClickEffects(e.clientX, e.clientY, clickResult);
  }

  /**
   * Create visual effects for click
   */
  private createClickEffects(x: number, y: number, clickResult: ClickResult): void {
    // Squish the bufo via a plain CSS transition (.bufo-image already has
    // `transition: transform 0.2s ease`). This can never get "stuck" the way a
    // rAF-driven animation latch could: every click just re-applies the squish
    // and a single timer clears it back to rest.
    if (this.bufoImage) {
      this.bufoImage.style.transform = 'scale(0.95)';
      if (this.squishResetTimer !== null) {
        window.clearTimeout(this.squishResetTimer);
      }
      this.squishResetTimer = window.setTimeout(() => {
        if (this.bufoImage) {
          this.bufoImage.style.transform = '';
        }
        this.squishResetTimer = null;
      }, 90);
    }

    // Create click indicator
    this.createClickIndicator(x, y);

    // Create floating number with wider scatter
    this.createFloatingNumber(x, y, clickResult);

    // Pop a little bufo out alongside the points
    this.createEmojiPop(x, y);
  }

  /**
   * Bufo image pool for the click pop effect - real art pulled from both the
   * generator icons and the upgrade icons in assets/images (no emoji).
   */
  private static readonly BUFO_IMAGES = [
    // Generators
    './assets/images/bufo.png',
    './assets/images/generators/bufo-smol.png',
    './assets/images/generators/bufo-brain.png',
    './assets/images/generators/bufo-cash-money.png',
    './assets/images/generators/bufo-galaxy-brain.png',
    './assets/images/generators/bufo-has-midas-touch.png',
    './assets/images/generators/bufo-monstera.png',
    './assets/images/generators/bufo-old.png',
    './assets/images/generators/chonky-bufo-wants-to-be-held.png',
    './assets/images/generators/hypnobufo.png',
    './assets/images/generators/smol-bufo-feels-blessed.png',
    // Upgrades
    './assets/images/upgrades/bufo-dapper.png',
    './assets/images/upgrades/bufo-drake-yes.png',
    './assets/images/upgrades/bufo-mindblown.png',
    './assets/images/upgrades/bufo-simba.png',
    './assets/images/upgrades/bufo-gives-star.png',
    './assets/images/upgrades/bufo-give-money.png',
    './assets/images/upgrades/bufo-chefkiss-with-hat.png',
    './assets/images/upgrades/bufo-deal-with-it.png',
    './assets/images/upgrades/bufo-gentleman.png',
    './assets/images/upgrades/king-bufo.png',
    './assets/images/upgrades/shut-up-and-take-my-bufo.png',
    './assets/images/upgrades/bufo-caught-a-small-bufo.png',
    './assets/images/upgrades/bufo-iron-throne.png',
    './assets/images/upgrades/bufo-universe.png',
    './assets/images/upgrades/confused-math-bufo.png'
  ];

  /**
   * Spawns a random bufo image at the click point that arcs up and back down
   * like a ball tossed under gravity, fading out on the way down.
   */
  private createEmojiPop(x: number, y: number): void {
    if (!this.effectLayer) return;

    const src = ClickArea.BUFO_IMAGES[Math.floor(Math.random() * ClickArea.BUFO_IMAGES.length)];

    const el = document.createElement('div');
    el.className = 'click-emoji-pop';
    // 36px wide and centred on the point via a -18px margin, so keep it that
    // far from either edge (it drifts up to 70px sideways from here, which is
    // allowed to leave the screen - the label isn't).
    el.style.left = `${clampToViewport(x, 0, 22)}px`;
    el.style.top = `${y}px`;

    const img = document.createElement('img');
    img.src = src;
    img.alt = '';
    img.draggable = false;
    // If a specific image ever 404s, just drop the pop instead of showing a
    // broken-image icon.
    img.onerror = () => el.remove();
    el.appendChild(img);

    this.effectLayer.appendChild(el);

    const duration = 800 + Math.random() * 200; // ms
    const peakHeight = 50 + Math.random() * 50; // px risen at the apex
    const drift = Math.random() * 140 - 70; // px of horizontal wander (~2x spread)
    const spin = Math.random() * 100 - 50; // deg of tumble over the whole arc
    const startTime = performance.now();

    const step = (now: number) => {
      const t = Math.min((now - startTime) / duration, 1);

      // Symmetric parabola: 0 at t=0 and t=1, peak (rising) at t=0.5 - a ball
      // thrown straight up under gravity and caught back at the same height.
      const yOffset = -4 * peakHeight * t * (1 - t);
      const xOffset = drift * t;
      const rotation = spin * t;

      el.style.transform = `translate(${xOffset}px, ${yOffset}px) rotate(${rotation}deg)`;

      // Only start fading once past the apex, on the way back down.
      if (t > 0.5) {
        el.style.opacity = `${1 - (t - 0.5) / 0.5}`;
      }

      if (t < 1) {
        requestAnimationFrame(step);
      } else {
        el.remove();
      }
    };

    requestAnimationFrame(step);
  }

  /**
   * Create floating number with improved styling and moderate scatter
   */
  private createFloatingNumber(x: number, y: number, clickResult: ClickResult): void {
    const element = document.createElement('div');
    element.classList.add('floating-number');
    
    // Add combo styling
    if (clickResult.isCombo) {
      element.classList.add('combo');
    }
    
    // Set content - just the value, no multiplier
    const valueSpan = document.createElement('span');
    valueSpan.classList.add('value');
    valueSpan.textContent = `+${clickResult.bufosGained.toFixed(1)}`;
    element.appendChild(valueSpan);
    
    // Apply CSS directly for immediate effect
    element.style.cssText = `
      position: absolute;
      font-weight: bold;
      font-size: ${clickResult.isCombo ? '1.4em' : '1.2em'};
      color: ${clickResult.isCombo ? '#FFD700' : '#FFFFFF'};
      text-shadow: 0 1px 2px rgba(0,0,0,0.8);
      pointer-events: none;
      z-index: 1000;
    `;
    
    if (clickResult.isCombo) {
      element.style.cssText += `
        background-color: rgba(76, 175, 80, 0.2);
        padding: 4px 8px;
        border-radius: 10px;
      `;
    }
    
    // Position with moderate random offset for scatter effect
    const offsetX = Math.random() * 30 - 15; // Reduced scatter: +/- 15px
    const offsetY = Math.random() * 20 - 10; // Reduced scatter: +/- 10px
    element.style.top = `${y + offsetY}px`;

    // Add to container
    if (this.effectLayer) {
      this.effectLayer.appendChild(element);
    }

    // Animate with improved trajectory
    let progress = 0;
    const duration = 1500; // Fixed duration
    const startY = y + offsetY;

    // Nothing clips this layer any more, so the label has to keep itself
    // inside the viewport - on a narrow phone, clicking the right-hand edge
    // of the bufo would otherwise push "+N" off the side of the screen.
    // Measured after appending, since the width depends on the number.
    const drift = Math.random() * 40 - 20; // Slight horizontal drift
    const startX = clampToViewport(x + offsetX, element.offsetWidth);
    const targetX = clampToViewport(startX + drift, element.offsetWidth);
    element.style.left = `${startX}px`;

    // Moderate range of movement
    const targetY = startY - (50 + Math.random() * 20); // Upward motion
    
    const animationFrame = () => {
      progress += 16 / duration; // ~60fps
      
      if (progress >= 1) {
        // Remove when done
        if (element.parentNode) {
          element.parentNode.removeChild(element);
        }
        return;
      }
      
      // Non-linear movement for more natural floating
      const easeOutCubic = 1 - Math.pow(1 - progress, 3);
      const currentY = startY + (targetY - startY) * easeOutCubic;
      
      // Gentle arc to horizontal movement
      const horizontalProgress = progress < 0.5 
        ? 2 * progress * progress 
        : 1 - Math.pow(-2 * progress + 2, 2) / 2;
      const currentX = startX + (targetX - startX) * horizontalProgress;
      
      // Opacity with fade-out
      const opacity = progress < 0.7 ? 1 : 1 - ((progress - 0.7) / 0.3);
      
      // Scale - start small, grow, then shrink
      let scale = 1;
      if (progress < 0.2) {
        scale = 0.7 + (progress / 0.2) * 0.5;
      } else {
        scale = 1.2 - ((progress - 0.2) / 0.8) * 0.4;
      }
      
      // Apply styles
      element.style.transform = `translate(${currentX - startX}px, ${currentY - startY}px) scale(${scale})`;
      element.style.opacity = opacity.toString();
      
      requestAnimationFrame(animationFrame);
    };
    
    requestAnimationFrame(animationFrame);
  }

  /**
   * Create click indicator
   */
  private createClickIndicator(x: number, y: number): void {
    const container = this.effectLayer;
    if (!container) return;
    
    // Create indicator element
    const indicator = document.createElement('div');
    indicator.classList.add('click-indicator');
    
    // Position at click point
    indicator.style.left = `${x}px`;
    indicator.style.top = `${y}px`;
    
    // Add to container
    container.appendChild(indicator);
    
    // Remove after animation completes
    setTimeout(() => {
      if (indicator.parentNode === container) {
        container.removeChild(indicator);
      }
    }, 1000);
  }

  /**
   * Clear clicks older than combo timeout
   */
  private clearOldClicks(): void {
    const now = Date.now();
    this.recentClicks = this.recentClicks.filter(time => {
      return (now - time) < this.comboTimeout;
    });
  }

  /**
   * Render the click area
   */
  public render(): string {
    return `
      <img src="${this.imagePath}" alt="Bufo" class="bufo-image" draggable="false">
      <div class="click-indicator-container"></div>
    `;
  }

  /**
   * Clean up resources
   */
  public destroy(): void {
    // Clear timers
    if (this.clearClicksTimer !== null) {
      clearInterval(this.clearClicksTimer);
      this.clearClicksTimer = null;
    }
    if (this.squishResetTimer !== null) {
      clearTimeout(this.squishResetTimer);
      this.squishResetTimer = null;
    }

    if (this.effectLayer && this.effectLayer.parentNode) {
      this.effectLayer.parentNode.removeChild(this.effectLayer);
    }
    this.effectLayer = null;

    super.destroy();
  }
}