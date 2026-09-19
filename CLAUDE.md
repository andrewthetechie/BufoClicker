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

`dist/` and `node_modules/` have both ended up root-owned before - any
one-off `docker run` without `--user` writes into the bind mount as root. If
a `docker compose run --rm build` fails with `EACCES: permission denied,
unlink ...`, that's why. Fix ownership from a root container rather than
`rm -rf`-ing from the host:
```
docker run --rm -v "$PWD":/app -w /app node:22-bookworm bash -lc \
  "chown -R 1000:1000 /app/node_modules /app/dist"
```
Only those two directories are ever affected (nothing git tracks), and both
are regenerable. Pass `--user 1000:1000` on one-off `docker run`s that write
to the repo to avoid creating the problem in the first place.

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
- **Only `clickMultiplier`, `generatorProduction`, and `globalMultiplier` are
  live upgrade effect types.** `upgradeManager.ts`'s `applySingleEffect`
  switch silently no-ops (`default: Logger.warn`) on anything else - this bit
  `ribbit_resonance`, which used an unhandled `clickBpsBonus` type and cost
  100,000 bufos for nothing. Fixed by repointing it to `clickMultiplier`
  instead of building a new temporary-buff mechanic for one upgrade. If you
  add a new effect type to `upgrades.json`, it needs a case in that switch or
  it'll fail the same way, silently.
- **An unrecognised unlock-condition type used to silently *pass*.**
  `upgrades.json` has conditions like `{"type": "upgrade", "id": "..."}`
  (prerequisite upgrade), but `mapUnlockConditionTypeFromString` had no case
  for them, so they fell through to `default: TotalBufos` - with `value`
  undefined, `totalBufos < undefined` is `false`, the condition "passed", and
  three upgrades (`ribbit_resonance`, `metamorphosis_mastery`, `super_clicker`)
  ignored their prerequisites entirely. Now a real `Upgrade` condition type,
  and the `default` branch returns something *always unmet* plus a warning
  rather than something always met. If you add a condition type to the JSON it
  still needs a case in both that mapper and `meetsUnlockConditions`.
- **`GeneratorType` drifts out of sync with `generators.json`.** The enum in
  `models/generators.ts` is hand-maintained and was missing `nebula_bufo` /
  `omega_bufo` / `singularity_bufo` for a whole session. Nothing in the game
  enumerates it at runtime (it's structural typing plus `debugTools`), so the
  only visible symptom was `debugTools.generator_debug.give()` refusing the
  three newest tiers - which quietly invalidates any test that seeds state
  through it. Add new generators to the enum at the same time as the JSON.
- **Result modals must survive the click that was already in flight.** The
  boss victory/defeat modals open the instant a fight ends, i.e. while the
  player is still spamming clicks, and `closeOnBackdrop: true` meant the very
  next click dismissed the popup before it could be read. They're now
  `closeOnBackdrop: false` plus `BossFight.guardAgainstStrayClicks()`, which
  puts `.modal--input-locked` (a `pointer-events: none` CSS class, see
  `modal.css`) on the close/confirm controls for 800ms. Any future modal that
  appears as a *consequence of clicking* wants the same treatment.
- **`overflow-y: auto` silently clips the *horizontal* axis too.** Per CSS,
  if one axis is not `visible` the other computes to `auto` rather than
  staying `visible`. `.column` sets only `overflow-y: auto`, so it clipped
  sideways as well - and the click effects (the "+N" label, the ripple, the
  bufo pop) were absolutely positioned children of `.frog-display` inside it,
  so clicking the right-hand edge of the bufo cut the label in half. They now
  render into `.click-effect-layer`, a `position: fixed` body-level layer with
  no clipping ancestor, positioned in viewport coordinates. Anything that
  needs to visually escape a panel wants that layer, not a `z-index` bump -
  z-index does nothing against an ancestor's overflow. The flip side is that
  nothing constrains those effects any more, so `clampToViewport()` in
  `clickArea.ts` keeps the label on screen on a narrow phone.
- **A full-screen fight overlay needs `pointer-events: auto` on itself, not
  just its children.** `.boss-fight-overlay` used to be `pointer-events: none`
  with only the sprite/HUD set to `auto` - visually it covered the screen but
  clicks fell straight through it to the bufo underneath, letting players
  farm normal click income during a "boss fight." Fixed by making the overlay
  itself swallow clicks (`pointer-events: auto`); the sprite/HUD still work
  since their own `auto` takes precedence as descendants.

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
- **UI init runs before the save loads - don't set DOM state from persisted
  data in `init()` and expect it to stick.** `initialization.ts`'s sequence
  is: init managers -> init UI (step 4) -> init game core -> **then** load
  the save (step 7). `BossFight.init()` used to set
  `document.body.dataset.bossStage` once from `getDefeatedCount()`, which ran
  against the fresh default state (0 defeated) - after a reload with
  defeated bosses, the background silently reverted to stage 0 and never
  corrected itself. Fixed by moving that read into a `syncBossStage()` method
  called both from `init()` (best-effort) and every `GAME_TICK` via the
  existing `refreshBanner()` poll, so it self-corrects within ~100ms of the
  real save data arriving instead of needing a dedicated "save loaded" event.
  Any other one-shot `init()`-time read of persisted state should either poll
  the same way or hook `GAME_STARTED` (emitted after the save load
  completes), not assume `init()` timing.
- **Boss progress is per-run, the boss multiplier is forever.**
  `state.bosses` has two fields for this: `defeated` (ids beaten in the
  current prestige run - drives the ladder in `getAvailableBoss()` and the
  `data-boss-stage` background) and `lifetimeDefeats` (a count banked from
  previous runs). `getBossMultiplier()` adds both, so `PrestigeManager.
  transcend()` can fold `defeated` into `lifetimeDefeats` and clear it: the
  ladder re-opens, the background resets to stage 0, and not one point of
  earned multiplier is taken back. Anything else that wants to "reset progress
  but keep the reward" should copy this shape rather than trying to preserve
  a derived number.
- **All clicks go through `GameCore.registerClick()`.** It owns
  `gameCore.clickCount`, `resources.clickCount` and the achievement manager's
  counter, so the three can't drift. `click()` (the main bufo, which earns
  bufos) and `BossFight.handleHit()` (boss damage, which doesn't) both call
  it - boss fights are most of the late-game clicking and used not to count
  toward the click achievements at all. `AchievementManager`'s `'click'`
  listener deliberately does *not* increment anything; it used to, on top of
  `setClickCount()`, which made every counted click worth 1.5 clicks.
- **A backgrounded tab earns nothing unless you credit it on resume.** Two
  things stop production when the tab is hidden: `requestAnimationFrame`
  (which drives `gameLoop`) doesn't fire in a background tab, and
  `initialization.ts`'s `visibilitychange` handler deliberately calls
  `getGameCore().stop()` + `getGameLoop().stop()` so a boss countdown can't
  run while nobody's watching. The pause is correct; losing the income isn't.
  The hidden branch stamps `gameSettings.lastTick` and the visible branch
  calls `applyElapsedProduction(lastTick, 0)` before restarting - same
  function `loadGame()` uses for closed-browser offline progress, just with
  the one-minute floor dropped, since a 20-second tab switch is still real
  idle time. `GameLoop.start()` resets its own frame clock, so crediting the
  gap before restarting can't double-count it.
- **Timed buffs get a countdown badge; permanent bonuses don't.** Golden
  Bufo's two frenzy buffs (`frenzyProductionMultiplier`/
  `frenzyClickMultiplier` in `resources`) are the only *temporary* multiplier
  sources - everything else (prestige, boss defeats, achievements) is
  permanent and has no "time left" to show. `GoldenBufoManager` tracks
  `productionFrenzyEndsAt`/`clickFrenzyEndsAt` wall-clock timestamps and
  exposes them via `getActiveFrenzies()`; `GoldenBufo` (the UI component)
  polls that every `GAME_TICK` and renders a `.frenzy-badge` (label +
  shrinking bar) in the top-right, same polling pattern as the boss HUD
  timer. If a third timed buff is ever added, extend `getActiveFrenzies()`'s
  return shape rather than inventing a second indicator.

## Mobile

`styles/mobile.css` is the phone/small-tablet layer and **must stay the last
`@import` in `index.css`** - it relies on source order to win, since most of
its selectors are single-class and media queries add no specificity. The base
design is a desktop three-column layout that only ever collapsed to one stack
at `<=1024px`; nothing else was sized for a phone.

The breakpoint is `max-width: 1024px`, matching the stacking breakpoint in
`layout.css` - deliberately wide enough to catch landscape phones (844x390)
and small tablets, which are stacked and touch-driven but wider than a
portrait phone. A separate `(max-height: 520px) and (orientation: landscape)`
block un-stickies the bufo and shrinks it, since a sticky 190px bufo eats
half a landscape phone's screen.

`touch-action: manipulation` is set at every width (not just on mobile) for
the bufo, boss sprite, golden bufo and buttons. The default `auto` leaves
double-tap-to-zoom on, which is precisely the gesture rapid-clicking a
clicker produces, and on some engines it also delays every click ~300ms
waiting for a possible second tap. `manipulation` kills double-tap zoom while
leaving pinch zoom alone, so it doesn't cost accessibility the way
`user-scalable=no` would. Hover styling is neutralised under `(hover: none)`
because `:hover` latches after a tap on touch devices.

Two traps worth knowing:

- **`.column` has `overflow-y: auto`**, which makes it a scroll container and
  clips anything sticky or overflowing inside it. The mobile layer sets
  `overflow: visible !important` there (the columns aren't height-limited on
  a phone, the page scrolls instead) - that's what lets the bufo be
  `position: sticky`.
- **`.buy-button` has `min-width: 120px; flex-shrink: 0`** inside a flex
  `.generator-row`. Giving it `width: 100%` makes it overlap the generator
  name rather than fill the row; the row has to `flex-wrap` and the button
  needs `flex: 1 1 100%` instead.

Verify with headless Chrome at 375/390/412/768 wide and assert, rather than
eyeballing screenshots: zero elements whose `right` exceeds `clientWidth`,
zero `button`/`.shop-item`/`.upgrade-item` under 44x44, and that the boss
sprite/banner/HUD and modals all stay inside the viewport. The page-level
`scrollWidth` is a *bad* signal on its own - the container clips, so the menu
bar overflowed to x=436 on a 375px screen (Reset was unreachable) while
`documentElement.scrollWidth` still read 375.

## Asset provenance

All bufo art (generator/upgrade/boss icons, the click-pop images) comes from
the "Bufo/Froge" Discord-emote meme set, pulled from
[knobiknows/all-the-bufo](https://github.com/knobiknows/all-the-bufo) (1700+
images, same naming convention as the images already in the repo). **That
repo states no explicit license** ("vetted to be safe for work but use your
own best judgement"). Same provenance as the project's original ~60 images -
flagging in case it matters later (e.g. if this ever needs a real
distribution license). Background photos are properly Unsplash-License (free,
no attribution required): the pond photo, the nebula/space photo (final boss
stage), and `swamp.jpg`/`storm.jpg`/`volcano.jpg`/`inferno.jpg` (boss stages
1-4, one real background-image swap per defeated boss instead of just a CSS
filter on the pond photo).

## Economy balance pass (generators.json)

Diagnosed by simulating the whole economy in Python (idle-only, "always buy
whatever has the best production-per-cost right now" greedy strategy, using
the real cost/production formulas from `generatorManager.ts`/
`models/generators.ts`), not by inspection alone - the simulator isn't
checked into the repo (throwaway, same as the Puppeteer scratch scripts).
Two structural issues showed up:

1. **Payback period (`baseCost / baseProduction`) roughly doubled every
   generator tier** (100s -> 100s -> 137s -> 255s -> 500s -> ... -> 207,000s
   at `singularity_bufo`), instead of staying roughly flat like the early
   tiers do. Each new premium tier was a worse deal than the last, which
   compounds badly by the time you reach the tiers added this session.
2. **Two "own 10 of the previous tier" unlock gates dominated their
   `totalBufos` threshold**: `cosmic_bufo` required `golden_bufo >= 10` and
   `singularity_bufo` required `omega_bufo >= 10`. At those tiers'
   `costMultiplier` (1.5-1.85), the cumulative cost of reaching the 10th unit
   dwarfed the nominal `totalBufos` gate, so the count-gate - not the headline
   number - was the actual bottleneck.

Fixed by flattening `cosmic_bufo` through `singularity_bufo`'s
`baseCost`/`baseProduction`/`costMultiplier` so payback grows ~1.55x per tier
instead of ~2x, and dropping both `>= 10` count-gates to `>= 5`. In
simulation this took `singularity_bufo`'s unlock time from ~19h50m down to
~10h39m of optimal play, and shrank the worst late-game dead zone
(omega -> singularity) from ~8h to ~23min. Did **not** touch tadpole through
golden_bufo (pre-existing, already paces well, new tier roughly every few
minutes early on) - the one remaining soft spot is the golden_bufo ->
cosmic_bufo gap, still ~2h17m in simulation. If this needs another pass,
re-derive the same way: compute `baseCost/baseProduction` per tier and check
it's not growing much faster than the tier before it, and check any
`generators`-type unlock gate's *cumulative* cost (not just its face value)
against the `totalBufos` gate it's paired with.

## Boss ladder extension (interdimensional_bufo, omniscient_bufo)

The original 5-boss ladder's click-power chain (`stronger_clicks_1` through
`quantum_click` in `upgrades.json`) tops out at a fixed native
`clickMultiplier` of 1,125,000 (750,000 from the chain documented in
`boss.ts` x 1.5 from `ribbit_resonance`) - there are no more click upgrades
past that point, so bosses gated any further out than `mega_bufo` would be
either trivial (if easy) or permanently unwinnable (if hard), since the
player's click power literally cannot grow any further. Added two new
click upgrades specifically to unstick this: `stronger_clicks_6` (10x,
gates around the nebula/omega tiers) and `omniscient_clicks` (20x, gates
after singularity unlocks), then calibrated the two new bosses' HP against
the click power those upgrades unlock (same "~120 clicks in 30s" target used
for the original ladder, computed by hand: `clickPower = clickMultiplier x
(1 + defeatedCount x 0.25)`, `HP = clickPower x 120`). If you extend the
ladder again, this is the pattern: a boss needs *both* a threshold past the
previous one *and* a fresh click-power upgrade to grind toward, or it isn't
a real checkpoint.

## Numbers that are first-pass and may need tuning

None of these have been human-playtested, only verified to be *mechanically*
correct (right math, right event flow, no crashes/errors):

- Boss HP/thresholds in `src/models/boss.ts` (7-boss ladder, gated by
  totalBufos thresholds, calibrated against the click-upgrade chain in
  `upgrades.json`). Simulated pacing for bosses 1-5 looks good (boss1@~10min,
  boss2@~2h25m, boss3@~4h29m, boss4@~5h44m, boss5@~8h38m). Bosses 6-7
  (`interdimensional_bufo` @ 50T totalBufos, `omniscient_bufo` @ 2 quadrillion)
  were added specifically to cover the late-game stretch after
  `nebula`/`omega`/`singularity` unlock, which previously had zero boss
  content - see the ladder-extension note below for how they were
  calibrated. Not wall-clock-simulated (that stretch would take many
  simulated hours); verified only by seeding `resources.clickMultiplier`
  directly in a headless-Chrome test and confirming each clears in
  ~108-114 hits against a 120-hit target, sequential unlock ordering holds,
  and `document.body.dataset.bossStage` reaches "7".
- The late upgrade tiers (`<gen>_mastery` / `<gen>_ascendancy` in
  `upgrades.json`). Every generator used to stop offering upgrades far below
  the count players actually reach - the last chromatic gate was 25 owned
  against a reachable ~55, quantum was 5 against ~22 - so buying more of
  anything eventually stopped unlocking anything.
  **Gates must come from simulated reachable counts, not from a cost
  threshold.** The first attempt derived them from "the count at which one
  more unit costs ~1e13/1e16" and 12 of 28 new upgrades turned out to be
  permanently unreachable, because a greedy buyer stops pouring bufos into a
  cheap generator long before its unit cost gets that high. Re-derive by
  re-running the greedy sim from the economy balance pass above (buy whatever
  has the best marginal production-per-cost, real cost/production formulas,
  upgrade multipliers applied as they unlock) with the new tiers *excluded*
  so it isn't circular, then place mastery near the plateau at ~1e16 lifetime
  spend and ascendancy near the plateau at ~1e18. Cost is 2x the unit that
  unlocks it - the median ratio of the pre-existing count-gated upgrades.
  Current state: every count-gated upgrade in the file unlocks by ~1e19
  lifetime spend, and nothing is dead content. That sim also caught three
  *pre-existing* upgrades that were already unreachable
  (`froglet_boost_4` gated at 150 froglets against a reachable ~118, since
  lowered to 100); check for those too if you add generators.
- New generator tier costs/production in `assets/data/generators.json`
  (`nebula_bufo`/`omega_bufo`/`singularity_bufo`) and their upgrades - see
  the balance pass above, now flattened but still first-pass/un-playtested.
- Prestige curve (`prestigePointsFor` in `src/models/prestige.ts`):
  `floor(sqrt(totalBufos / 1e9))`, +10%/point.
- Golden Bufo timing/rewards in `src/managers/goldenBufoManager.ts`
  (spawn interval, frenzy multipliers/durations, Lucky payout formula).
