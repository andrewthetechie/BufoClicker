# Rust and WebAssembly implementation proposal

Draft proposal. It implements the [Rust and WASM ADR](adr/0001-port-client-to-rust-and-webassembly.md). The [task plan](rust-wasm-task-plan.md) orders the work.

## Outcome and scope

Replace the TypeScript game client with Rust compiled to WebAssembly (WASM). Keep the game on static hosting. GitHub Pages is its current host. Preserve player-facing mechanics, progress, layout, and mobile behavior. Keep CSS, HTML, images, and game content JSON as static assets. Permit only generated JavaScript in the production build.

The owner prefers Rust for future work and expects agents to maintain Rust code more effectively. Static hosting already works. This proposal excludes performance gains, new gameplay, a server, and a new visual design. The retired Explorer/RPG subsystem has no UI, so the port omits it. The new client still accepts its legacy save field.

## Current baseline

| Area | Current implementation | Rust plan |
| --- | --- | --- |
| Build and hosting | webpack builds `dist/`. GitHub Actions publishes it to Pages. Docker provides development, build, and nginx preview services. | Build Rust beside TypeScript, then switch the build at cutover. |
| Game state | `src/core/types.ts`, `src/utils/stateUtils.ts`, and `src/game/gameState.ts` share or duplicate state rules. Managers own additional transient state. | Give Rust one authoritative state model and one calculation path for values that depend on it. |
| Game content | The browser fetches `assets/data/generators.json`, `upgrades.json`, and `achievements.json`. Empty fallback values allow an incomplete game to start. | Keep runtime fetches, validate the catalogs, and stop startup on failure. |
| UI | `src/managers/UIManager.ts` replaces the placeholder markup in `index.html` and coordinates component classes. | Match the rendered UI and CSS hooks. |
| Saves | `bufo_idle_save` stores a JSON envelope. `src/game/gameSave.ts` restores from its nested `state`. Export strings use `btoa(encodeURIComponent(JSON.stringify(saveData)))`. | Test real legacy shapes, write a new schema, and retain the original value for recovery. |
| Verification | `npm test` is a stub. Existing browser probes use development console helpers. | Add native Rust tests and Rust-authored browser tests before replacing the old build. |

## Target architecture

Use a Cargo workspace with a game crate and a browser crate. The game crate has no browser dependencies. A Rust browser-test package can use the same workspace. During migration, keep the Rust HTML entry and output directory separate from the TypeScript site.

```text
Cargo.toml                 workspace
rust-toolchain.toml        pinned compiler and WASM target
Cargo.lock                 locked Rust dependencies
crates/game/               content types, rules, state, save migration
crates/web/                Leptos views and browser adapters
crates/e2e/                Rust browser tests
rust-index.html            parallel Trunk input during migration
Trunk.toml                 parallel output and public URL settings
styles/ and assets/        existing static source assets
```

The game crate owns the authoritative game state and rules. Define the durable state shape before implementing every mechanic. Include counters, owned items, achievement progress and flags, prestige, boss history, and settings. Keep active fights and temporary effects outside saves. Pass elapsed time and random values into rules so tests can reproduce timers and random events. The functions return updated state and typed outcomes for the browser crate to render.

The browser crate owns Leptos signals, DOM events, storage, fetch, animation frames, and visibility events. Keep economy rules out of view code. Leptos handles component state and cleanup. Do not port the TypeScript component base classes.

Keep content definitions separate from player state. Parse JSON with `serde` and `serde_json` into typed Rust catalogs indexed by string IDs. Validate required fields, finite numbers, effect and condition types, and references between catalogs. Use these IDs instead of a second generator enum.

Show a retry screen if content is invalid. JSON schema changes require a matching client release. Compatible value changes can deploy without recompiling WASM.

### Startup and time

1. Show a static loading shell in HTML.
2. Fetch and validate the three content files using URLs resolved against `document.baseURI`.
3. Read and validate the new save, or migrate the legacy save when no new save exists.
4. Reconstruct derived values once, then mount the game UI and start its timers.
5. Enable writes only after the content and state are valid.

Keep a fixed-step game update and publish view changes at a lower rate. The current loop uses a 60 Hz step. It updates the UI about every 100 ms. Evaluate `leptos_use::use_raf_fn` for browser scheduling and pause/resume in the pilot. Use a small `web_sys` adapter if it cannot preserve the required timing behavior. A hidden tab pauses the loop and boss countdown. It also stops temporary Golden Bufo effects.

On tab return, credit elapsed generator production before the loop resumes. On reload, credit gaps of at least one minute. Tab returns have no one-minute floor. Cap credited production at 12 hours on both paths. Treat timed buffs as temporary and do not restore them from a save.

### Player progress

The new client reads `bufo_idle_save` and writes `bufo_idle_save_v2`. A valid v2 save takes precedence. If v2 is absent, migrate the legacy save. If v2 fails validation, show recovery controls. Never select older progress without the player's choice.

Validate migration and import in memory before any write. Keep the old key unchanged for recovery. For reset, validate and serialize fresh v2 state, then write it with one `localStorage.setItem` call. Only then replace the current state and report success. A failed reset write leaves the current state and both stored keys unchanged, and shows an error. This order prevents a reload from reimporting old progress.

Recovery offers retry, backup import, restoration from the old key when present, and reset. Do not write until the player chooses an option and the chosen state passes validation.

Show a persistent warning if storage fails. After acknowledgment, the player can continue in memory. Export must work from the current state without storage. Disable durable reset while storage fails. Memory-only play must never report a reset as durable.

The v2 envelope has a distinct `schema_version`, a save timestamp, and the authoritative game state. Store resource and click counters, owned generators, purchased upgrades, achievement progress and event flags, prestige, boss history, and settings. Recalculate multipliers, costs, and production after load. Do not save active boss fights or temporary Golden Bufo buffs.

Migration must reproduce the old loader's defaults and precedence rules. This includes click-count restoration and one-time upgrade effects. Reject non-finite values and impossible state before writing v2.

The new UI provides export and import controls. Export v2 with the existing `btoa(encodeURIComponent(JSON.stringify(...)))` format. The browser adapter uses `js_sys` URI functions and `web_sys` `btoa` and `atob` bindings for the existing codec. The game crate uses `serde_json` for decoded JSON. Select the legacy or v2 parser from `schema_version` after decoding. Test decoded legacy JSON natively and encoded string compatibility in browser tests.

A failed import leaves both stored saves unchanged. Export and import also let players move to another origin. Browser `localStorage` does not move with the static files.

### Gameplay and UI

Port player flows without copying the manager classes. These flows include clicking, generator purchases and production, upgrades, achievements, prestige, boss fights, and Golden Bufo events. Include the shop, owned generators, resource and production views, statistics, modals, notifications, menus, and mobile layout.

Keep unaffordable buttons clickable so players receive purchase feedback. Boss hits count toward click achievements but grant no ordinary click income. A boss loss clears the current bufo stash but keeps generators, upgrades, and prestige. Prestige resets run progress but keeps permanent prestige and boss rewards.

Preserve the `console_opened` achievement with Rust browser bindings. Use the current keyboard and viewport-size detection. This detection is a best-effort browser signal in the current client.

The first Leptos pilot covers a click, resource counter, shop purchase, and modal. It must use existing CSS and static assets. If the pilot needs custom UI infrastructure to match current interactions and layout, reconsider Leptos before expanding the port.

## Build and verification

Trunk builds the browser crate through a `rel="rust"` link to its manifest. Use Trunk copy directives for `styles/` and `assets/`. Move the `@import` block in `styles/index.css` before its style rules. Browsers can then apply the copied stylesheet. Keep the existing import order, image paths, and mobile override order.

Set Trunk's public URL for each host and include `<base data-trunk-public-url/>` in the HTML entry. Resolve data and image URLs against `document.baseURI`. Test both `/` and `/BufoClicker/`. The production build contains HTML, CSS, images, JSON, generated JavaScript, and WASM. A static host serves these files.

Pin the Rust toolchain, Cargo dependencies, and Trunk release in containers and continuous integration (CI). Keep container commands for development, build, and nginx preview. Run native game tests, Rust lint and format validation, browser component tests, and full browser flows. Use Rust-authored `thirtyfour` WebDriver tests against the deployed page. Automate Chrome and Firefox where CI supports them. Run the task plan's recorded Safari cases on desktop and iPhone before cutover.

Record reference cases from the TypeScript game before replacing it. Include synthetic legacy saves for fresh, purchased, prestiged, boss-progress, and malformed states. Compare exact progression values and visible outcomes. Supply fixed time and random inputs where possible.

Test missing JSON, storage failure, save recovery, reset, import and export, and both hosting paths. The [task plan](rust-wasm-task-plan.md) gives the release gates.

## Cutover and risks

Keep the TypeScript site deployed while Rust runs in a separate preview. Once Rust can write saves, serve that preview from another origin. Publish Rust only after feature parity, save migration, and browser tests pass.

At cutover, switch `dist/` and the Pages workflow. Remove TypeScript and Node build files. Update `README.md` and `CLAUDE.md`. The plan has no dual-write or reverse-migration path.

| Risk | Response |
| --- | --- |
| A save parses but loses progress through defaults or repeated effects. | Use legacy fixtures, field-level comparisons, and a separate v2 key. Block writes on restore failure. |
| Runtime JSON changes independently of the WASM client. | Validate its schema and IDs before startup. Treat incompatible schema changes as a client release. |
| A preview at `/` misses Pages path failures. | Serve and test the exact `/BufoClicker/` prefix before cutover. |
| WASM startup or frequent UI updates increase delay. | Measure pilot startup, bundle size, and update behavior against the current site. Do not assume improvement. |
| Outdated repo instructions give the wrong build commands. | Replace Node and TypeScript commands in the workflows and repo docs during cutover. |

## Primary references

- [Leptos client-side rendering and Trunk](https://book.leptos.dev/getting_started/index.html)
- [Leptos testing guidance](https://book.leptos.dev/testing.html)
- [Trunk configuration](https://trunk-rs.github.io/trunk/guide/configuration/index.html) and [asset directives](https://trunk-rs.github.io/trunk/guide/assets/index.html)
- [`serde_json`](https://docs.rs/serde_json/latest/serde_json/) and [`leptos_use::use_raf_fn`](https://docs.rs/leptos-use/latest/leptos_use/fn.use_raf_fn.html)
- [`js_sys` URI bindings](https://docs.rs/js-sys/latest/js_sys/) and [`web_sys` Window bindings](https://docs.rs/web-sys/latest/web_sys/struct.Window.html)
- [CSS Cascading and Inheritance: import placement](https://drafts.csswg.org/css-cascade-5/#at-import)
- [Thirtyfour Rust WebDriver client](https://github.com/stevepryde/thirtyfour)
- [HTML Standard local storage](https://html.spec.whatwg.org/multipage/webstorage.html)
- [MDN: `visibilitychange`](https://developer.mozilla.org/en-US/docs/Web/API/Document/visibilitychange_event) and [`beforeunload` limitations](https://developer.mozilla.org/en-US/docs/Web/API/Window/beforeunload_event)
