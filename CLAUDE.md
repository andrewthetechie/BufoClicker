# CLAUDE.md

Working notes for Claude Code (or a human) picking this repo back up. This
file tracks decisions and non-obvious context - see `README.md` for how to
actually run/build/deploy, and `git log` for chronological history.

## Hard constraint: Docker-only

The user has no Node toolchain on their host machine and wants none. **Every**
`npm`/`node`/`npx` command for this repo must run inside a container - use the
committed `Dockerfile` + `docker-compose.yml`, or a one-off
`docker run --rm -v "$PWD":/app -w /app node:22-bookworm ...`. Never suggest
installing Node locally. `node_modules/` and `dist/` are git-ignored and
expected to be absent/regenerable, not committed.

```
docker compose up -d dev site
```
`:9000` = webpack-dev-server with `window.debugTools` in console (dev-mode
only). `:8080` = production build via nginx. `debugTools.help()` lists every
debug helper (resources, generators, upgrades, prestige, golden bufo, boss).

`dist/` has ended up root-owned before (an nginx/root build wrote into a
bind-mounted host dir). If a `docker compose run --rm build` fails with
`EACCES: permission denied, unlink ...`, that's why - clear it via a
root container rather than `rm -rf` from the host:
```
docker run --rm -v "$PWD":/app -w /app node:22-bookworm bash -lc \
  "rm -rf /app/dist && mkdir -p /app/dist && chown 1000:1000 /app/dist"
```

## Non-obvious bugs and fixes worth knowing about

- **Unaffordable buy buttons must stay clickable.** Setting the native
  `disabled` DOM property on a shop/upgrade button silently swallows clicks
  with no feedback - this was the root cause behind GH #1 ("Golden Bufo does
  not work") and #2 ("Bufo Academy Upgrade doesn't work"). `shopItem.ts` /
  `upgradeItem.ts` keep the button clickable and let the manager's real
  success/fail response drive shake/flash feedback instead. Relatedly,
  purchases and the game state save on `pagehide`/`beforeunload`/tab-hidden
  (see `initialization.ts`) - a bare in-memory purchase used to vanish on
  refresh.
- **Don't gate clicks with a cooldown or a rAF-driven animation latch.**
  `clickArea.ts` used to have a 50ms hard click cooldown, plus a squish
  animation that set a `dataset.animating` flag cleared by
  `requestAnimationFrame` - which never fires in a backgrounded tab, so the
  flag (and clicking) could get stuck forever. The squish is now a plain
  CSS-transition (self-healing, nothing to latch); `pulse()` in
  `animationUtils.ts` also has a failsafe timeout defensively even though
  `clickArea.ts` no longer calls it.
- **Number formatting must force `'en-US'`.** Bare `.toLocaleString()` is
  locale-dependent (can render "1.000" for "1,000" depending on runtime
  locale) and unclamped huge numbers render as raw scientific notation.
  `numberUtils.ts`'s `formatNumber`/`formatNumberWithPrecision` force
  `'en-US'` and clamp/format the K/M/B-suffix path explicitly.
- **`ribbit_resonance` upgrade is a known no-op.** Its effect type
  `clickBpsBonus` (`assets/data/upgrades.json`) isn't handled by
  `upgradeManager.ts`'s `applySingleEffect` switch - it silently falls into
  the `default: Logger.warn` branch. Buying it currently does nothing but
  cost bufos. Pre-existing, not introduced by any of the above.

## Things intentionally not done

- **RPG/Explorer mini-game**: `explorerManager.ts`/`models/explorer.ts`/
  `combat.ts`/`enemies.ts` are a fully-built backend (combat sim, leveling,
  enemy types including a `Boss` variant, drop tables) with **zero UI**.
  Superseded by the simpler Clicker Boss mechanic (`src/models/boss.ts` +
  `bossManager.ts` + `bossFight.ts`) per explicit direction to drop the RPG
  approach. The backend is still there, just unused - fine to leave, or
  delete later if it's clearly dead weight.
- **Export/Import save UI**: backend (`Game.exportSave/importSave`) fully
  works, no UI button exists. Deprioritized in favor of bosses/prestige/
  golden bufo.

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
  `gameCore.ts` alongside the others (start/stop hooks if it needs to pause
  with the game), and exposed via `debugTools.ts` for console testing.
- **Testing approach**: no test runner is configured (`npm test` is a stub).
  Verification is headless-Chrome (Puppeteer) driven through
  `window.debugTools`, run inside a `node:22-bookworm` container with Chrome
  deps apt-installed ad hoc, hitting the dev server or `./dist`. No test
  files are checked into the repo; it's throwaway scratchpad scripting each
  time. Worth formalizing into a real test setup at some point given how much
  verification work has taken this shape.
- **Boss banner dismissal is snoozed, not permanent.** `BossFight` in
  `src/ui/components/bossFight.ts` has a `snoozedUntil` wall-clock timestamp
  checked at the top of `refreshBanner()`; clicking "Not yet" sets it to
  `Date.now() + (60_000 + Math.random()*120_000)`. If you add more banner
  dismiss actions, route them through the same field rather than a fresh
  ad hoc flag.

## Asset provenance

All bufo art (generator/upgrade/boss icons, the click-pop images) comes from
the "Bufo/Froge" Discord-emote meme set, pulled from
[knobiknows/all-the-bufo](https://github.com/knobiknows/all-the-bufo) (1700+
images, same naming convention as the images already in the repo). **That
repo states no explicit license** ("vetted to be safe for work but use your
own best judgement"). Same provenance as the project's original ~60 images -
flagging in case it matters later (e.g. if this ever needs a real
distribution license). Background photos are properly Unsplash-License (free,
no attribution required): the pond photo and the nebula/space photo used for
the final boss stage.

## Numbers that are first-pass and may need tuning

None of these have been human-playtested, only verified to be *mechanically*
correct (right math, right event flow, no crashes/errors):

- Boss HP/thresholds in `src/models/boss.ts` (5-boss ladder, gated by
  totalBufos thresholds, calibrated against the click-upgrade chain in
  `upgrades.json`).
- New generator tier costs/production in `assets/data/generators.json`
  (`nebula_bufo`/`omega_bufo`/`singularity_bufo`) and their upgrades.
- Prestige curve (`prestigePointsFor` in `src/models/prestige.ts`):
  `floor(sqrt(totalBufos / 1e9))`, +10%/point.
- Golden Bufo timing/rewards in `src/managers/goldenBufoManager.ts`
  (spawn interval, frenzy multipliers/durations, Lucky payout formula).
