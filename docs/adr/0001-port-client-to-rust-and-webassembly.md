---
status: proposed
---

# Port the game client to Rust and WebAssembly

See the [implementation proposal](../rust-wasm-proposal.md) for the design. The [task plan](../rust-wasm-task-plan.md) gives the work order and release gates.

BufoClicker ships as a static site. Its game and UI use TypeScript, Node, and webpack. The project owner prefers Rust for future work and expects agents to work more effectively in Rust. Port the current game to Rust and WebAssembly (WASM). Preserve player progress and static hosting.

## Decision

- Put game rules and state transitions in Rust so native tests can run them. Compile the browser client for `wasm32-unknown-unknown`.
- Use [Leptos client-side rendering](https://book.leptos.dev/getting_started/index.html) for the UI. Keep the rendered layout, interactions, and CSS hooks where practical. Use Rust browser bindings for storage, fetch, timers, and animation frames. Do not create a custom UI framework.
- Keep all hand-authored game and UI behavior in Rust. Allow only generated JavaScript loader and binding code in the site output. Keep HTML, CSS, images, and JSON as static source assets.
- Use Trunk to produce `dist/` with HTML, WASM, generated loader code, CSS, images, and data. Fetch generator, upgrade, and achievement JSON at runtime. Compatible JSON value changes need a deployment but no WASM rebuild. Schema changes need a matching Rust client build.
- Make the build serve correctly at a site root or path prefix. GitHub Pages remains the current host at `/BufoClicker/`, but the site must work on other static hosts. No application server or runtime API is part of the design.
- Replace the Node and webpack steps in Docker and GitHub Actions with pinned Rust and Trunk toolchains. Keep container-based development, build, and production preview commands.

## Saves and retired code

Read legacy saves from the `localStorage` key `bufo_idle_save`. The legacy loader treats the envelope's nested `state` as authoritative. Write new saves under `bufo_idle_save_v2`. Give the new schema its own version, separate from the old application version.

Load a valid new save when it exists. If the new key is absent, migrate the legacy save. Write the new key only after a successful load. Leave the legacy key untouched for recovery.

Accept existing export strings encoded as `btoa(encodeURIComponent(JSON.stringify(saveData)))`. Add visible export and import controls so players can move saves between origins.

A corrupt new save or failed migration must show recovery choices and block automatic writes. Do not silently start a new game or load an older snapshot. Reset must validate and write a fresh v2 save before changing the current state or reporting success. If that write fails, leave the current state and stored saves unchanged, and show an error. Restore the legacy value only when the player chooses it. Do not write both formats or support migration back to TypeScript.

Represent resource quantities and multipliers with finite `f64` values to match current JavaScript numbers. Preserve large-number formatting. Remove the Explorer role-playing game (RPG) subsystem. It has no player UI but runs on each tick. Accept its field in legacy saves without restoring its behavior.

## Migration and release tests

Start with a Leptos pilot that covers a click, resource counter, shop purchase, and modal. Test the current CSS at a site root and `/BufoClicker/`. Then port the game model, UI, persistence, and development console hooks in stages. Write debug hooks in Rust and include them only in development builds.

Validate the three runtime JSON files before starting the game or writing a save. If a fetch or validation fails, show an error and retry. The current fallback content is empty, so continuing would produce an incomplete game.

Before cutover, compare the Rust build with the current game. Cover purchases, unlocks, achievements, prestige, bosses, Golden Bufo, and resource formatting. Test saves after purchases, periodic saves, `visibilitychange` to hidden, `pagehide`, export and import, and reloads from both formats. Do not depend on `beforeunload` for saving.

Test elapsed production on reload and tab return. Preserve the one-minute threshold on reload, no threshold on tab return, and the 12-hour cap. Test desktop and mobile layouts in current Chrome, Firefox, and Safari, including iPhone Safari.

Run native Rust tests for game rules and save migration. Run browser tests for UI behavior, storage, and loading at root and path-prefixed URLs. Keep the TypeScript site deployed until those tests pass. Then switch the static build and remove TypeScript, Node, webpack, and their lockfile. Update `README.md` and `CLAUDE.md` to describe the Rust workflow.

## Tradeoffs

Static hosting does not require this rewrite. Rust is the owner's preferred language for future maintenance. Porting only game rules would leave two application languages and the Node build. A Rust server would conflict with static hosting. Leptos supplies component state and lifecycle management without a project-specific UI framework.

The rewrite adds Rust browser bindings and a WASM download before the game UI starts. Generated JavaScript remains necessary. The plan assumes no speed or download-size gain. Browser storage belongs to an origin, so a host move requires export and import controls. Existing saves and player-facing behavior are release requirements.

## References

- [Rust's `wasm32-unknown-unknown` target](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html)
- [Leptos client-side deployment](https://book.leptos.dev/deployment/csr.html)
- [Trunk assets and public URL](https://trunk-rs.github.io/trunk/guide/assets/index.html)
- [HTML Standard: local storage and origins](https://html.spec.whatwg.org/multipage/webstorage.html)
- [MDN: `visibilitychange`](https://developer.mozilla.org/en-US/docs/Web/API/Document/visibilitychange_event) and [`beforeunload` limitations](https://developer.mozilla.org/en-US/docs/Web/API/Window/beforeunload_event)
