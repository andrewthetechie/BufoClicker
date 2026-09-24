# Rust and WebAssembly task plan

Draft plan. It implements the [proposal](rust-wasm-proposal.md) and [ADR](adr/0001-port-client-to-rust-and-webassembly.md). Task IDs show dependencies, not calendar dates.

## Delivery rule

Keep the TypeScript site buildable and deployed through T16. Build the Rust site into a separate `dist-rust/` preview. T02 through T12 use preview state and must not write `bufo_idle_save` or `bufo_idle_save_v2`.

Serve T13 through T16's writable preview from another origin. After each task, the old build must still work and the Rust workspace must pass its current checks. T17 alone switches deployment and removes the old code.

Each gameplay task adds a usable player flow through the Rust game crate and Leptos UI. Add native rule tests and a browser test for each flow. Use current TypeScript behavior as the reference unless the proposal specifies a change.

## Dependency graph

| ID | Result | Blocked by |
| --- | --- | --- |
| T01 | Capture the current game's behavior and synthetic legacy saves. | None |
| T02 | Open a parallel Leptos pilot at root and Pages paths. | None |
| T03 | Load all three runtime catalogs or show a retry screen. | T02 |
| T04 | Click, buy generators, and watch production update. | T01, T03 |
| T05 | Unlock and buy upgrades with visible feedback. | T04 |
| T06 | Earn and view achievements. | T05 |
| T07 | Pause, resume, and credit elapsed generator production. | T04 |
| T08 | Transcend and keep permanent progress. | T05, T06 |
| T09 | Fight bosses without earning ordinary click income. | T07, T08 |
| T10 | Catch Golden Bufo and use timed rewards. | T06, T07 |
| T11 | Use statistics, reset flow, and mobile layout. | T09, T10 |
| T12 | Decode legacy saves and create v2 save envelopes. | T01, T05 |
| T13 | Load saves and recover on startup. | T11, T12 |
| T14 | Save play and reset durably. | T13 |
| T15 | Export and import across origins. | T14 |
| T16 | Pass the complete parity and browser matrix. | T15 |
| T17 | Switch the static build and remove the TypeScript toolchain. | T16 |

Start T01 and T02 first. After T04, T05 and T07 can run at the same time. Start T12 after T05 while other gameplay tasks continue. T09 and T10 can run at the same time after their dependencies. Coordinate edits to shared Rust crates.

## Task details

### T01. Capture the TypeScript baseline

- Record current flows and rules in a parity matrix under `tests/`. Capture test saves from the browser app. Use synthetic values, not user data.
- Include fresh, purchased-generator, upgraded, achievement, prestige, and boss-history saves. Add malformed JSON, missing fields, and the old encoded export string format.
- Record reference outputs for purchase costs, production, click power, unlocks, prestige points, boss scaling, number formatting, and elapsed production. Use fixed timestamps and seeded input where the old code permits it.
- Create a run record template under `tests/` for the Safari checklist below. Include browser, OS, device, URL, fixture, result, and failure evidence. Cover denied storage and quota errors with synthetic fixtures.
- Complete when each test save traces to `src/utils/saveManager.ts`, `src/game/gameSave.ts`, `src/core/types.ts`, and the relevant managers. Record nondeterministic behavior as browser cases with observable bounds.

### T02. Build a parallel Rust pilot

- Add the Cargo workspace, `rust-toolchain.toml`, lockfile, Leptos browser crate, Trunk config, and a separate Rust HTML input. Keep webpack and the current Pages workflow intact.
- Use Trunk's input/output settings to generate `dist-rust/index.html`. Link to `crates/web/Cargo.toml` with `rel="rust"`. Include `<base data-trunk-public-url/>` and Trunk copy directives for CSS and assets. Move the `styles/index.css` import block before its style rules while keeping module order. Render a click, counter, shop purchase, and modal without a custom component base class.
- Evaluate `leptos_use::use_raf_fn` for frame scheduling and pause/resume. Use a small `web_sys` adapter if the helper cannot preserve the fixed-step timing rules. Keep the accumulator in the game crate.
- Add container commands and a CI job for Rust format, lint, native tests, and a Trunk release build. Pin the toolchain and Trunk release when implementing this task.
- Complete when the pilot starts at `/` and `/BufoClicker/` and uses current CSS hooks. Keep the TypeScript production build and layout unchanged. Record startup time and bundle size. A performance gain is not a release requirement.

### T03. Load and parse game content

- Add `serde` and `serde_json` catalog parsing in `crates/game/` and a fetch adapter in `crates/web/`. Fetch generators, upgrades, and achievements from `assets/data/` against `document.baseURI`.
- Require valid fields, finite numbers, known effect and condition types, and IDs that refer to other catalogs. Reject content that refers to retired Explorer behavior.
- Show an error and retry control if any file is missing or invalid. Do not start the game or enable writes until all catalogs pass validation.
- Complete when the checked-in JSON loads at root and prefix paths. Each missing or malformed file produces a visible error instead of an empty shop.

### T04. Earn bufos and buy generators

- Port click registration, resource counts, generator unlocks, production, owned-generator lists, and production statistics. Calculate costs for purchases of 1, 10, 100, or the maximum affordable number.
- Define the complete durable state shape: counters, owned generators and upgrades, achievement progress and flags, prestige, boss history, and settings. Keep temporary fights and buffs outside saves. Keep mutable state separate from catalog definitions. Calculate costs and multipliers in one Rust path. Use a fixed-step game loop and lower-rate view updates.
- Keep unaffordable purchase buttons clickable so failure feedback appears. Preserve large-number formatting and the CSS click effects.
- Complete when a player can start with zero progress, click, buy a generator, and see the expected ongoing production. Compare costs and counts against T01 fixtures.

### T05. Buy and apply upgrades

- Port upgrade availability, prerequisite checks, purchase feedback, purchased state, and the current click, generator, and global multiplier effects.
- Check effect types before play so an unsupported effect cannot take currency without changing the game. Recompute derived values from canonical state after a purchase.
- Complete when upgrade cards appear at the same thresholds and unaffordable cards give feedback. Each purchased upgrade must change the expected output exactly once.

### T06. Earn and inspect achievements

- Port achievement requirements, progress, notifications, rewards, statistics, and the achievements view. Count ordinary and boss clicks through one canonical counter.
- Preserve the `console_opened` achievement with Rust browser bindings for the current keyboard-shortcut and viewport-size checks. Check it on startup, resize, and at an interval.
- Preserve one-time event flags for boss defeats and Golden Bufo catches. Later tasks add those event producers without changing the achievement save shape.
- Complete when click, generator, production, and upgrade milestones unlock once and render correctly. Check that repeated view updates do not grant a reward twice. Test console detection with a browser keyboard event and synthetic viewport dimensions in a native test.

### T07. Handle visibility and elapsed production

- Port the visibility and page lifecycle behavior from `src/initialization.ts` and elapsed-production rules from `src/game/gameSave.ts`.
- Pause active play when hidden. Credit the hidden interval before restarting. Keep the one-minute minimum for a closed-page reload, no minimum for tab return, and the 12-hour production cap.
- Use explicit time inputs in native tests. A 20-second hidden interval earns production. A 20-second reload does not. A reload after more than 12 hours credits at most 12 hours.
- Complete when a tab can hide and resume without lost or double-counted income. T13 connects the reload path to persisted timestamps.

### T08. Transcend without losing permanent rewards

- Port the prestige threshold, point award, multiplier, confirmation modal, and run reset. Clear current resources, generators, and upgrades while keeping permanent achievements and prestige.
- Carry boss-history fields in state even before T09 provides the fight UI. This keeps the final reset rule in one place.
- Complete when a qualifying player can transcend, start a new run, and receive the expected permanent click and production bonus.

### T09. Fight the boss ladder

- Port boss eligibility, scaled health, click damage, 30-second countdown, retreat, win/loss modals, and stage background. A loss clears current bufos but keeps generators, upgrades, and prestige.
- Boss hits count toward click achievements but grant no normal bufo income. The overlay must intercept clicks. The result modal must survive stray clicks. Pause the countdown while hidden.
- Complete when a boss fight can be won, lost, and retreated from. Hiding a tab pauses the countdown. A transcend reopens the ladder without losing the banked boss multiplier.

### T10. Catch Golden Bufo

- Port random spawn timing, expiration, collection, lucky reward, production frenzy, click frenzy, badges, and visual placement.
- Feed deterministic random values and time into the game rules for native tests. Stop active frenzies when the game pauses, as the current client does.
- Complete when each reward changes only its intended values for the intended duration. Collecting a Golden Bufo must trigger its achievement flag.

### T11. Finish the game controls and responsive UI

- Port the remaining statistics, in-memory reset flow, confirmation dialogs, modals, notifications, tooltips, and responsive CSS behavior. T13 through T15 connect persistence and durable controls.
- Keep the existing stylesheet order, especially `styles/mobile.css` last. Test that feedback in lists causes no horizontal scrolling. Keep click effects visible near viewport edges.
- Complete when every non-persistence control on the current page has a Rust equivalent. Match the recorded desktop and phone layouts.

### T12. Define and parse save v2

- Define `schema_version: 2` for values that T13 will write under `bufo_idle_save_v2`. Use the full durable state shape defined in T04, including fields whose gameplay arrives in T06 through T10. Parse the legacy envelope from `bufo_idle_save`, using its nested `state` as the source of progress.
- Normalize legacy defaults and precedence, including click count, achievement flags, prestige, boss history, and generator state. Reapply purchased effects once. Ignore the retired Explorer field.
- Reject invalid v2 or legacy input before storage writes. Keep the legacy key untouched. Test decoded T01 JSON fixtures and round-trip a v2 save without changing any canonical field. These tests cover field preservation. T13 tests encoded strings in the browser. Later gameplay and browser tests cover restored bonuses.
- Complete when native tests prove old-to-new migration and new-to-new round trips. T13 enables the browser write path.

### T13. Load saves and recover on startup

- Read a valid v2 save before the legacy key. Migrate the legacy save only when v2 is absent. Credit reload production from the saved timestamp using T07 rules.
- Treat an invalid v2 save or failed migration as a recovery state with automatic writes blocked. Offer retry, backup import, explicit restoration from the old key, and reset. Do not silently select the old save.
- Add the browser codec for encoded backup strings. Use `js_sys` URI functions and `web_sys` `atob` and `btoa`. Pass decoded JSON to the game crate. Reuse this adapter for the normal menu controls in T15.
- Accept a recovery state only after it passes the save validator. Write v2 once, then enter play. For recovery reset, commit fresh state only after the v2 write succeeds. Leave the legacy key untouched.
- If storage fails, show a persistent warning and allow memory-only play after acknowledgment. Provide export from the warning screen. Keep durable reset disabled while storage fails.
- Complete when old saves migrate once and valid v2 takes precedence. Corrupt v2 offers each recovery choice. Failed writes preserve current state and both keys. Test denied storage and quota errors.

### T14. Save play and reset durably

- Save v2 after generator and upgrade purchases, prestige, and boss outcomes. Add manual save, the enabled 60-second autosave, and saves when `visibilitychange` reports hidden or `pagehide` fires. Keep the autosave setting in state. Do not depend on `beforeunload`.
- Keep writes blocked until startup accepts content and state. On a failed ordinary save, show the persistent storage warning and retain current in-memory play. Test tab return, reload, and `pagehide` without lost or double-counted production.
- For reset, run the save validator and serialize fresh v2 state. Then call `localStorage.setItem`. Commit the in-memory reset and show success only after that call succeeds. If it fails, keep current state and both stored keys unchanged and show an error.
- Complete when reloads retain purchases, prestige, and boss rewards. A failed reset under denied storage or quota leaves progress visible and does not restore an old snapshot on reload.

### T15. Export and import across origins

- Add visible export and import controls to the normal menu. Export v2 through the codec from T13 and accept both legacy and v2 encoded strings. Select the parser from `schema_version` after decoding.
- Accept imported state only after it passes the save validator. Write v2 once, then replace in-memory state. If parsing or writing fails, leave current state and both stored keys unchanged and show an error.
- Complete when a player exports on one origin and imports on another. Include legacy export fixtures, v2 round trips, malformed strings, and denied storage in browser tests.

### T16. Run the release parity matrix

- Expand Rust-authored `thirtyfour` browser tests across fresh and migrated games, purchases, achievements, prestige, bosses, and Golden Bufo. Include recovery and failed content loads. Use development-only Rust hooks in an instrumented preview for deterministic cases.
- Run the production build under `/` and `/BufoClicker/`. Check that WASM, CSS, image, and JSON requests resolve under both paths.
- Run automated Chrome and Firefox flows. Run the Safari checklist below on desktop Safari and iPhone Safari. Use the production build for load and smoke cases, and the instrumented preview for timed cases. Record browser, OS, device, URL, fixture, result, and failures. Use `safaridriver` on macOS when available. Use a device session for iPhone checks.
- Complete when native tests, browser tests, the T01 matrix, and recorded Safari checks pass. Resolve all save-loss and broken-control cases.

### T17. Cut over the static site

- Switch Trunk output to `dist/` and update `.github/workflows/ci.yml`, `.github/workflows/deploy.yml`, `Dockerfile`, and `docker-compose.yml` for Rust.
- Remove TypeScript source, including `styles/styleLoader.ts`, webpack and Node configuration, `package.json`, and `package-lock.json`. Replace the old `index.html` with the Trunk entry. Keep static styles, assets, and the Rust legacy save decoder.
- Update `README.md`, `CLAUDE.md`, and `.gitignore`. Replace old commands and TypeScript architecture notes with the Rust layout, debug hooks, test commands, and save policy.
- Complete when container development, build, and nginx preview work. CI builds `dist/`, and the Pages build passes root and prefix tests. Inspect the shipped build for authored JavaScript. Keep the old `localStorage` value untouched in users' browsers.

## Safari acceptance checklist

Run these checks on desktop Safari and iPhone Safari against the release candidate. Record a result for each browser, including its browser and OS version, device, URL, fixture, and failure evidence. Use synthetic saves and an instrumented preview for timed events. Use the production build for load and basic play.

| Check | Expected result |
| --- | --- |
| Fresh load at `/` and `/BufoClicker/` | HTML, WASM, CSS, images, and all three JSON files load. Click and purchase work. |
| Progress and refresh | Legacy and v2 fixtures restore exact canonical progress. Purchases, prestige, and boss rewards survive refresh. |
| Visibility and elapsed time | A 20-second hidden interval credits production once. The boss timer pauses. A 20-second reload credits nothing. A gap over 12 hours credits at most 12 hours. |
| Gameplay | Upgrades, achievements, prestige, boss win and loss, and Golden Bufo rewards match T01 cases. |
| Recovery and transfer | Corrupt v2 shows recovery choices. Reset survives reload. Export and import work across origins. |
| Failures | Missing content blocks startup and writes. Denied storage and quota errors show the warning without losing current progress. |
| Phone layout and touch | No horizontal scrolling. Primary controls, menus, modals, and close buttons respond to touch. |

## Release gate

T17 may start when all of these are true:

1. Every player-facing flow in T01 has a working Rust equivalent or an explicit, approved bug fix.
2. Legacy and v2 save fixtures round-trip without lost canonical progress. Recovery and reset never select old progress silently.
3. Missing content and storage failures show usable errors and do not overwrite saves.
4. Production assets and data load from both a root and the Pages project path.
5. Chrome, Firefox, desktop Safari, and iPhone Safari pass the recorded browser checks.

T17 finishes when Rust replaces Node in the container workflow and CI. The Pages build passes the same tests, and repo instructions match the shipped build.
