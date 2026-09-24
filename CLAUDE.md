# CLAUDE.md

Working notes for Claude Code (or a human) picking this repo back up. This
file tracks decisions and non-obvious context — see `README.md` for how to
actually run/build/deploy, and `git log` for chronological history.

## Hard constraint: Docker-only

The user has no toolchain on their host machine and wants none. **Every**
`cargo`/`trunk`/`rustc` command for this repo must run inside a container —
use the committed `Dockerfile` + `docker-compose.yml`, or a one-off
`docker run --rm -v "$PWD":/app -w /app rust:1.93.1-bookworm ...`. Never
suggest installing Rust locally. `target/` and `dist/` are git-ignored and
expected to be absent/regenerable, not committed.

```
docker compose up dev site
```
`:9000` = Trunk dev server. `:8080` = production build via nginx.

`dist/` and `target/` can end up root-owned when a one-off `docker run`
writes into the bind mount as root. If a `docker compose run --rm build`
fails with `EACCES: permission denied, unlink ...`, that's why. Fix ownership
from a root container rather than `rm -rf`-ing from the host:
```
docker run --rm -v "$PWD":/app -w /app rust:1.93.1-bookworm bash -lc \
  "chown -R 1000:1000 /app/target /app/dist"
```
Only those two directories are ever affected (nothing git tracks), and both
are regenerable.

## Architecture

The game client was ported from TypeScript to Rust/WASM/Leptos (see
`docs/adr/0001-port-client-to-rust-and-webassembly.md`,
`docs/rust-wasm-proposal.md`, `docs/rust-wasm-task-plan.md`). The Rust side is
a Cargo workspace:

- `crates/game/` — **pure Rust, no browser deps.** The authoritative game
  state (`state.rs`), economy (`economy.rs`), upgrades (`upgrades.rs`),
  achievements (`achievements.rs`), prestige (`prestige.rs`), boss ladder
  (`boss.rs`), Golden Bufo (`golden.rs`), number formatting (`number.rs`),
  catalog parsing/validation (`catalog.rs`), and save v2 + legacy migration
  (`save.rs`). Everything here is testable natively with `cargo test -p game`.
- `crates/web/` — **Leptos views + browser adapters.** `main.rs` (load flow),
  `game_view.rs` (the game UI), `boss_ui.rs`, `golden_ui.rs`,
  `console_detection.rs`, `fetch.rs` (catalog fetch against `document.baseURI`),
  `storage.rs` (localStorage v2/legacy + export/import codec).

**One authoritative state model, one calculation path.** Derived values (click
power, production, generator costs) are computed on demand from canonical
state — never stored — so they cannot drift. The TS triplication
(`types.ts` / `stateUtils.ts` / `gameState.ts`) is gone.

**Generators are keyed by string id from `generators.json`, not an enum.**
The old `GeneratorType` enum drifted out of sync with the JSON and quietly
broke debug seeding. There is no enum to drift now.

## Non-obvious decisions worth knowing (now enforced by native tests)

These are the bugs/decisions from the TypeScript era that the port preserves
or deliberately changes. Each is pinned by a `#[test]` in `crates/game`.

- **Unaffordable buy buttons must stay clickable.** Setting a native `disabled`
  DOM property swallows clicks with no feedback. The Rust UI keeps buttons
  clickable and relies on the purchase outcome (success/failure) for feedback.
- **Only `clickMultiplier`, `generatorProduction`, and `globalMultiplier` are
  live upgrade effect types.** Anything else is rejected at catalog load
  (`catalog.rs`) — never a silent no-op that takes currency.
- **An unrecognised unlock-condition type must be *unmet*, not *met*.**
  `catalog.rs` rejects unknown condition/effect/requirement/reward types
  before play starts.
- **Number formatting forces `en-US` and clamps the mantissa.** `number.rs`
  ports `formatNumber`/`formatNumberWithPrecision` exactly (K/M/B/T… suffixes,
  comma grouping, no scientific-notation fallback).
- **Boss health is scaled, not fixed.** `boss.rs::get_boss_health` multiplies
  `baseHealth` by the prestige and boss multipliers so the click target stays
  invariant to prestige. Boss hits count toward click achievements but grant
  no bufo income (`register_click`).
- **Prestige (transcend) resets the run but banks boss defeats.** `prestige.rs`
  folds `defeated` into `lifetime_defeats` and clears the ladder, so the
  permanent multiplier is never taken back.
- **Achievements for one-off milestones use custom-event latches.** Beat-this-
  boss / caught-Golden-Bufo achievements gate on `customEvents` flags
  (`boss_<id>`, `golden_bufo_caught`), which are one-way and survive a
  transcend.
- **Elapsed production:** one-minute floor on reload, no floor on tab return,
  12-hour cap (`economy.rs::apply_elapsed_production`).
- **Golden Bufo rewards are transient.** Frenzy multipliers are never saved;
  `migrate` resets them to 1 on load.

## Saves

- Legacy saves live under `localStorage` key `bufo_idle_save` (a JSON envelope
  whose nested `state` is authoritative). They are **read-only** and left
  untouched.
- New saves are written under `bufo_idle_save_v2` with `schema_version: 2`.
- A valid v2 save takes precedence; the legacy save is migrated only when v2
  is absent. A corrupt v2 save is a **recovery state** (writes blocked), never
  a silent fallback to the legacy key.
- Reset validates + serializes the fresh v2 state and writes it with one
  `setItem` **before** committing the in-memory reset; a failed write leaves
  both keys and the current state unchanged.
- Export/import uses the existing `btoa(encodeURIComponent(JSON.stringify(...)))`
  codec; the parser (v2 vs legacy) is selected from `schemaVersion` after
  decoding (`save.rs::parse_any`).

## Content gate

The three runtime catalogs (`assets/data/generators.json`, `upgrades.json`,
`achievements.json`) are fetched and validated **before** the game starts or
writes a save. A missing/malformed file shows a retry screen — never an empty
shop. Schema changes require a matching client release; value-only changes
deploy without a WASM rebuild.

## Verification

No browser-test framework is configured. Verification is:

1. **Native Rust tests** — `cargo test --workspace` (55 tests at cutover)
   cover every game rule, the save migration, and the number formatting.
   `cargo fmt --all -- --check` and
   `cargo clippy --workspace --all-targets -- -D warnings` must be clean.
2. **Manual browser checklist** — `tests/safari-run-record.md` + synthetic
   fixtures in `tests/fixtures/`. The owner runs Chrome/Firefox/desktop
   Safari/iPhone Safari against the production build before merging.

`tests/parity-matrix.md` maps every player flow to its TS source (pre-port)
and the Rust test that now pins it.
