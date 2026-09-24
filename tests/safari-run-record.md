# Safari / browser acceptance run record

Fill one row per browser. Use the production build (`dist/` at `/` and
`/BufoClicker/`) for load and basic-play checks, and the instrumented preview
for timed cases (elapsed production, boss countdown, frenzies). Synthetic
saves live in `tests/fixtures/`.

## Run record

| Field | Value |
|-------|-------|
| Browser | |
| Browser version | |
| OS | |
| Device | desktop / iPhone |
| URL | `/` or `/BufoClicker/` |
| Fixture | e.g. `tests/fixtures/purchased.json` |
| Result | PASS / FAIL |
| Failure evidence | screenshot / console log / devtools |

## Checklist

### Fresh load at `/` and `/BufoClicker/`
- [ ] HTML, WASM, CSS, images, and all three JSON files load (no 404s in network tab).
- [ ] Click earns bufos; counter updates.
- [ ] A generator purchase succeeds and production starts.

### Progress and refresh
- [ ] Legacy fixture (`fresh/purchased/upgraded/achievement/prestige/boss`) restores exact canonical progress.
- [ ] v2 fixture round-trips without losing purchases, prestige, or boss rewards.
- [ ] Purchases, prestige, and boss rewards survive a refresh.

### Visibility and elapsed time
- [ ] A 20-second hidden interval credits production once (no double-count on resume).
- [ ] The boss countdown pauses while hidden.
- [ ] A 20-second reload credits nothing.
- [ ] A gap over 12 hours credits at most 12 hours of production.

### Gameplay
- [ ] Upgrades unlock/purchase at the same thresholds; unaffordable cards give feedback.
- [ ] Achievements unlock once and never double-reward.
- [ ] Prestige awards points, resets the run, keeps permanent bonuses.
- [ ] Boss win and loss behave per `tests/parity-matrix.md` rows 19–23.
- [ ] Golden Bufo rewards change only their intended values for the intended duration.

### Recovery and transfer
- [ ] Corrupt v2 shows recovery choices (retry / import / restore legacy / reset).
- [ ] Reset survives reload.
- [ ] Export on one origin imports on another (legacy and v2 strings).

### Failures
- [ ] Missing content blocks startup and writes, shows retry (not an empty shop).
- [ ] Denied storage / quota error shows the warning without losing current progress.

### Phone layout and touch
- [ ] No horizontal scrolling (no element `right` exceeds `clientWidth`).
- [ ] Primary controls, menus, modals, and close buttons respond to touch and are ≥44×44.
