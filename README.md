# BufoClicker

An updated fork of [pjscheetz/BufoClicker](https://github.com/pjscheetz/BufoClicker)

An incremental / idle "clicker" game about breeding cartoon bufos (toads).
The game client is written in **Rust**, compiled to WebAssembly, with a
**Leptos** UI and a **Trunk** build. It deploys as a static site to GitHub
Pages.

Click the bufo to earn **bufos**, spend them on generators that produce bufos
automatically, then on upgrades that multiply that production.

---

## Requirements

You only need **Docker** (with the Compose plugin). Every Rust/cargo command
runs inside a container — nothing is installed on your machine.

- Docker Engine 24+ / Docker Desktop, including `docker compose`
- Ports `9000` (dev server) and `8080` (production preview) free

> Prefer a native Rust toolchain? See [Running without Docker](#running-without-docker).

---

## Run it locally (development)

Hot-reloading dev server (Trunk):

```bash
docker compose up dev
```

Open <http://localhost:9000>. Edits under `crates/` and `styles/` rebuild and
reload automatically. Stop with `Ctrl-C`.

---

## Build the production bundle

Writes the optimised static site into `./dist/` on your host:

```bash
docker compose run --rm build
```

`./dist/` is what gets published. It is git-ignored — treat it as a build
artifact, regenerate it whenever you need it.

Do not open `dist/index.html` with a `file://` URL. Browser security rules
block the generated JavaScript and WASM modules on that origin. The Pages
bundle also uses `/BufoClicker/` asset URLs, so it needs the matching URL path.
Use the production preview below and open `http://localhost:8080/BufoClicker/`.

---

## Preview the production build

Serve the contents of `./dist/` with nginx exactly as it would be hosted:

```bash
docker compose run --rm build      # make sure ./dist is fresh
docker compose up site
```

Open <http://localhost:8080>.

---

## Deploy to GitHub Pages

Deployment is automatic via GitHub Actions (`.github/workflows/deploy.yml`):
every push to `main` builds the site (`trunk build --release --public-url
/BufoClicker/`) and publishes `./dist` straight to GitHub Pages. There is
nothing to run locally — just merge to `main`.

Notes:

- In the repo settings, **Pages → Build and deployment → Source** must be set
  to **GitHub Actions** (not "Deploy from a branch"). Set this once per repo;
  the workflow handles every deploy after that.
- Check progress under the repo's **Actions** tab, or `gh run list` / `gh run watch`.
- The live URL is `https://<owner>.github.io/BufoClicker/`.

---

## Common tasks

| Task | Command |
| --- | --- |
| Dev server | `docker compose up dev` |
| Production build → `./dist` | `docker compose run --rm build` |
| Preview `./dist` | `docker compose up site` |
| Native tests | `docker compose run --rm build cargo test --workspace` |
| Format check | `docker compose run --rm build cargo fmt --all -- --check` |
| Lint | `docker compose run --rm build cargo clippy --workspace --all-targets -- -D warnings` |
| Shell in the toolchain container | `docker compose run --rm build bash` |
| Rebuild the image after dependency changes | `docker compose build` |

---

## Project layout

```
crates/
  game/         pure-Rust game rules: state, economy, upgrades, achievements,
                prestige, bosses, Golden Bufo, save migration — no browser deps
  web/          Leptos views + browser adapters (fetch, storage, timers, events)
styles/         plain CSS, copied verbatim into the build
assets/         images + JSON data (generators.json, upgrades.json, achievements.json)
tests/          parity matrix, synthetic save fixtures, Safari run record
Trunk.toml      Trunk build config (dist/, public URL)
rust-toolchain.toml  pinned compiler + wasm32-unknown-unknown target
```

Game content (generators, upgrades, achievements) is data-driven: it is
`fetch()`-ed at runtime from `assets/data/*.json`, so tuning numbers or adding
entries does **not** require a rebuild — just refresh.

Saves live in `localStorage`. New saves are written under `bufo_idle_save_v2`;
legacy saves under `bufo_idle_save` are migrated on first load and left
untouched. Export/import uses the existing
`btoa(encodeURIComponent(JSON.stringify(...)))` codec.
Right-click **Save** to open the export and import controls.

The [Playwright screenshot comparison](tests/parity-screenshots/README.md)
shows paired TypeScript and Rust views of the fresh game, upgrades, a purchased
Tadpole, click effects, and Stats.

---

## Running without Docker

Requires **Rust 1.93.1** (see `rust-toolchain.toml`), the
`wasm32-unknown-unknown` target, and **Trunk 0.21.14**.

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --locked --version 0.21.14

trunk serve                              # dev server on http://localhost:9000
trunk build --release --public-url /BufoClicker/   # production build into ./dist
cargo test --workspace                   # native game-rule tests
```

Deployment always runs in GitHub Actions (see above), not locally.
