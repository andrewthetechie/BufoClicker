# CLAUDE.md

Working notes for Claude Code (or a human) picking this repo back up. This
file tracks *session context and decisions*, not things already obvious from
reading the code - see `README.md` for how to actually run/build/deploy.

## Hard constraint: Docker-only

The user has no Node toolchain on their host machine and wants none. **Every**
`npm`/`node`/`npx` command for this repo must run inside a container - use the
committed `Dockerfile` + `docker-compose.yml`, or a one-off
`docker run --rm -v "$PWD":/app -w /app node:22-bookworm ...`. Never suggest
installing Node locally. `node_modules/` and `dist/` are git-ignored and
expected to be absent/regenerable, not committed - they were removed from
tracking this session on purpose (see git log once committed).

Local dev stack (already built and was running at end of session):
```
docker compose up -d dev site
```
`:9000` = webpack-dev-server with `window.debugTools` in console (dev-mode
only). `:8080` = production build via nginx. `debugTools.help()` lists every
debug helper (resources, generators, upgrades, prestige, golden bufo, boss).

## ⚠️ Nothing in this session has been committed to git

Everything below is sitting **uncommitted in the working tree**. `git status`
shows ~30 modified/added source files plus the git-ignored deletion of the
previously-committed `node_modules`/`dist` (~8200 files). If you're resuming
this later: check `git status` and `git diff --stat` first, and consider
committing in logical chunks (bug fixes / security+Docker / new content /
prestige+golden-bufo / boss feature) rather than one giant commit, since
that's roughly how the work happened. **Ask the user before committing** -
that instruction stood all session.

## What happened this session, roughly in order

1. **Fixed the two open GitHub issues** (`pjscheetz/BufoClicker` #1 "Golden
   Bufo does not work", #2 "Bufo Academy Upgrade doesn't work"):
   - Root cause of #2 (and the real bug behind #1's "no error, nothing
     happens"): unaffordable buy buttons had the native `disabled` property
     set, which silently swallows click events, AND purchases were never
     saved before a refresh (`beforeunload` handler had been deliberately
     removed). Fixed in `shopItem.ts`/`upgradeItem.ts` (keep buttons
     clickable, let the manager's real success/fail drive feedback) and
     `initialization.ts` (save on `pagehide`/`beforeunload`/tab-hidden/every
     purchase).
2. **Security**: bumped the whole webpack toolchain
   (`webpack-dev-server` 4→6, `copy-webpack-plugin` 12→14, webpack, loaders,
   `gh-pages`), dropped unused `clean-webpack-plugin`. `npm audit` = 0 vulns.
3. **Repo hygiene**: added `.gitignore`, untracked `node_modules`/`dist`,
   added `Dockerfile` + `docker-compose.yml` + `.dockerignore`, rewrote
   `README.md`.
4. **Click bug** ("sometimes clicks stop registering"): `clickArea.ts` had a
   50ms hard cooldown that silently dropped fast clicks, AND the click-squish
   animation used a `dataset.animating` latch that could get stuck forever if
   the tab was backgrounded mid-animation (rAF freezes in background tabs).
   Removed the cooldown; replaced the squish with a plain CSS-transition (self
   -healing, can't latch); added a failsafe timeout to the old `pulse()`
   helper defensively too.
5. **Number formatting**: `formatNumber`/`formatNumberWithPrecision` in
   `numberUtils.ts` used bare `.toLocaleString()` (locale-dependent - could
   render "1.000" instead of "1,000" depending on runtime locale) and had a
   bug where huge numbers (very high owned-generator counts) rendered as raw
   scientific notation. Forced `'en-US'` everywhere; capped/clamped display
   for absurd magnitudes.
6. **Click bufo pop**: every click now spawns a random bufo image (pulled from
   both `assets/images/generators/` and `assets/images/upgrades/`, not emoji)
   that arcs up/down under a fake-gravity parabola and fades on the way down.
   Lives in `clickArea.ts` (`createEmojiPop`, name is stale - it's images now)
   + `styles/animations.css` (`.click-emoji-pop`).
7. **Fixed broken upgrade icons**: two typos in `assets/data/upgrades.json`
   (`bufo-mostera.png`→`bufo-monstera.png`, `bufo-gives-money.png`→
   `bufo-give-money.png`) - correctly-named files already existed, just fixed
   the references.
8. **New background**: swapped `assets/images/background/pond.jpg` for a
   proper Unsplash-licensed lily-pond photo, and fixed the CSS (previous rule
   had no `background-size`, so it was tiling a small image - probably why it
   "sucked"). See Asset provenance note below.
9. **New generator tiers**: added `nebula_bufo`, `omega_bufo`,
   `singularity_bufo` to `assets/data/generators.json` (continuing past
   `quantum_bufo`), each with 2 boost upgrades + a synergy upgrade in
   `upgrades.json`, plus matching achievements in `achievements.json`. All
   pure data - the engine is data-driven, no code changes needed for these.
10. **Prestige** ("Transcendence Bufoplier", `src/models/prestige.ts` +
    `src/managers/prestigeManager.ts`): reach 1B total bufos this run →
    Transcend button lights up → soft-resets bufos/generators/upgrades, banks
    `floor(sqrt(totalBufos/1e9))` points, each point = permanent +10% to all
    production and click power, forever.
11. **Golden Bufo random event** (`src/managers/goldenBufoManager.ts` +
    `src/ui/components/goldenBufo.ts`): roams the screen every ~1.5-3 min,
    click it for Bufo Frenzy (x7 production/30s), Click Frenzy (x7
    clicks/15s), or a Lucky bufo windfall.
12. **Clicker Bosses** (`src/models/boss.ts` + `src/managers/bossManager.ts` +
    `src/ui/components/bossFight.ts`) - the newest, least-battle-tested piece:
    a sequential 5-boss ladder gated by totalBufos thresholds. Fully opt-in
    (a banner appears, fight only starts when you click "Fight") so the
    punishing loss condition can never trigger unattended. 30s to click a
    boss's HP to 0 using **click power only, no bufo income during the
    fight**. Win → permanent +25%-per-boss multiplier (stacks with prestige)
    + the world's background visibly shifts (`body[data-boss-stage]` CSS
    filter, swapping to a space photo at the final boss). Lose → `bufos → 0`
    only; every generator/upgrade/prestige point/achievement is untouched.
    Pauses correctly when the tab is hidden (reuses the existing
    `gameCore.start()/stop()` hooks). **Boss HP/thresholds are a first-pass
    design-time calibration** against the click-upgrade chain in
    `upgrades.json` (2x/2x/2x/5x/75x/5x/50x cumulative) - verified to work
    mechanically (win/lose/pause-resume/multiplier all pass headless-Chrome
    tests) but not human-playtested for feel/difficulty yet.

13. **Click-drag text selection fix**: rapid clicking (especially chasing the
    moving boss sprite) was drag-selecting page text/images. Fixed with
    `user-select: none` on the global `button` rule, `.frog-display`, and a
    `body.boss-fight-active` class toggled for the duration of a fight (plus
    `mousedown`/`dragstart` prevention on the boss sprite specifically).
    Verified with a simulated mousedown→mousemove→mouseup drag across the
    sprite during a fight - `window.getSelection().toString()` is empty.

## TODO for next session (explicitly requested, not yet done)

- **Boss banner "Not yet" should snooze, not just re-appear.** Right now
  `BossFight.refreshBanner()` re-checks on every `GAME_TICK` and immediately
  re-shows the banner once dismissed (`shownBossId` is cleared on "Not yet",
  so the very next tick shows it again) - annoying. Fix: when "Not yet" is
  clicked, hide the banner and suppress re-showing it for a **random 60-180
  seconds** before `refreshBanner()` is allowed to show it again (e.g. a
  `private snoozedUntil: number = 0` timestamp checked at the top of
  `refreshBanner()`, set to `Date.now() + (60_000 + Math.random()*120_000)`
  in the "Not yet" handler).

## Things intentionally NOT done

- **RPG/Explorer mini-game**: `explorerManager.ts`/`models/explorer.ts`/
  `combat.ts`/`enemies.ts` are a fully-built backend (combat sim, leveling,
  enemy types including a `Boss` variant, drop tables) with **zero UI**. User
  explicitly said to drop this in favor of the simpler Clicker Boss mechanic
  above. The backend is still there, just unused - fine to leave, or delete
  later if it's clearly dead weight.
- **Export/Import save UI**: backend (`Game.exportSave/importSave`) fully
  works, no UI button exists. Proposed, not built (deprioritized in favor of
  bosses/prestige/golden bufo).
- A pre-existing, unrelated bug was spotted but not fixed: the
  `ribbit_resonance` upgrade (`assets/data/upgrades.json`) has an effect type
  `clickBpsBonus` that `upgradeManager.ts`'s `applySingleEffect` switch
  doesn't handle - it silently no-ops (falls into the `default: Logger.warn`
  branch). Buying it currently does nothing but cost bufos.

## Architecture notes for extending this further

- **Multiplier hook points**: there are exactly two places that combine every
  multiplier source (upgrades, prestige, boss defeats, golden-bufo frenzy) -
  `GeneratorManager.recalculateGenerator()` (production) and
  `calculateDerivedState()` (click power, duplicated in
  `utils/stateUtils.ts` - the authoritative one used by `StateManager` - and
  `game/gameState.ts`, kept for API-export parity, both must be edited
  together). Any new permanent or temporary multiplier should plug into these
  two spots, not invent a third path.
- **New top-level state slice pattern** (prestige, bosses): add the interface
  to `core/types.ts` (`GameState` + `PartialGameState`), a merge branch in
  `utils/stateUtils.ts` `updateState()` + a mirrored one in
  `game/gameState.ts` `updateState()`, a default value in both
  `createDefaultState()` (stateUtils) and `DEFAULT_GAME_STATE` (gameState),
  and restoration in `game/gameSave.ts` `loadGame()`'s `properState` object
  (which is hand-built field-by-field, not a spread - easy to forget a slice
  here). `validateState()` doesn't need updating unless the slice is required
  for a save to be considered valid.
- **New manager pattern** (prestige/goldenBufo/boss managers): singleton
  class with `getInstance()`, own `reset()`, registered in
  `managers/index.ts` (`initializeManagers`/`resetManagers`), instantiated in
  `gameCore.ts` alongside the others (start/stop hooks if it needs to
  pause with the game), and exposed via `debugTools.ts` for console testing.
- **Testing approach used all session**: no test runner is configured
  (`npm test` is a stub). Verification was headless-Chrome (Puppeteer) driven
  through `window.debugTools`, run inside a `node:22-bookworm` container with
  Chrome deps apt-installed ad hoc, hitting a plain `http.createServer`
  serving `./dist`. No test files were added to the repo; this was all
  throwaway scratchpad scripting. Worth formalizing into a real test setup at
  some point given how much of this session was "build, then hand-write a
  Puppeteer script to prove it works."

## Asset provenance

All bufo art (including everything added this session - new generator/
upgrade/boss icons, the click-pop images) comes from the "Bufo/Froge"
Discord-emote meme set, sourced this session from
[knobiknows/all-the-bufo](https://github.com/knobiknows/all-the-bufo) (1700+
images, same naming convention as the images already in the repo before this
session). **That repo states no explicit license** ("vetted to be safe for
work but use your own best judgement"). This is the same provenance the
project's original ~60 images already had - nothing new was introduced
license-wise, just more of the same pool, per the user's explicit request to
pull more of these images. Flagging here in case that matters later (e.g. if
this ever needs a real distribution license). Background photos are properly
Unsplash-License (free, no attribution required): the pond photo and the
nebula/space photo used for the final boss stage.

## Numbers that are genuinely first-pass and may need tuning

- Boss HP/thresholds in `src/models/boss.ts` (see comment there).
- New generator tier costs/production in `assets/data/generators.json`
  (`nebula_bufo`/`omega_bufo`/`singularity_bufo`) and their upgrades.
- Prestige curve (`prestigePointsFor` in `src/models/prestige.ts`):
  `floor(sqrt(totalBufos / 1e9))`, +10%/point.
- Golden Bufo timing/rewards in `src/managers/goldenBufoManager.ts`
  (spawn interval, frenzy multipliers/durations, Lucky payout formula).

None of these were playtested by a human this session, only verified to be
*mechanically* correct (right math, right event flow, no crashes/errors).
