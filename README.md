# BufoClicker

An updated fork of [pjscheetz/BufoClicker](https://github.com/pjscheetz/BufoClicker)

An incremental / idle "clicker" game about breeding cartoon bufos (toads).
Vanilla TypeScript, bundled with webpack, deploys as a static site to GitHub
Pages.

Click the bufo to earn **bufos**, spend them on generators that produce bufos
automatically, then on upgrades that multiply that production.

---

## Requirements

You only need **Docker** (with the Compose plugin). Every Node/npm command runs
inside a container — nothing is installed on your machine.

- Docker Engine 24+ / Docker Desktop, including `docker compose`
- Ports `9000` (dev server) and `8080` (production preview) free

> Prefer a native Node toolchain? See [Running without Docker](#running-without-docker).

---

## Run it locally (development)

Hot-reloading dev server with source maps and in-browser debug tools:

```bash
docker compose up dev
```

Open <http://localhost:9000>. Edits under `src/` and `styles/` rebuild and
reload automatically. Stop with `Ctrl-C`.

The dev build exposes `window.debugTools` in the browser console
(`debugTools.help()` lists everything) — handy for granting resources, unlocking
generators, inspecting state, etc.

---

## Build the production bundle

Writes the optimised static site into `./dist/` on your host:

```bash
docker compose run --rm build
```

`./dist/` is what gets published. It is git-ignored — treat it as a build
artifact, regenerate it whenever you need it.

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

Publishing pushes the built site to the `gh-pages` branch of the `origin`
remote (GitHub Pages then serves it). The `deploy` script runs `build` first
via npm's `predeploy` hook.

```bash
docker compose run --rm \
  -v "$HOME/.ssh:/root/.ssh:ro" \
  -e GIT_AUTHOR_NAME="$(git config user.name)" \
  -e GIT_AUTHOR_EMAIL="$(git config user.email)" \
  -e GIT_COMMITTER_NAME="$(git config user.name)" \
  -e GIT_COMMITTER_EMAIL="$(git config user.email)" \
  build npm run deploy
```

Notes:

- The container needs push access to `origin`. Mounting `~/.ssh` read-only (as
  above) reuses your host SSH key; alternatively use a remote URL with a token.
- First deployment: in the GitHub repo settings, set **Pages → Build and
  deployment → Source** to **Deploy from a branch**, branch `gh-pages` / `/`
  (root).
- The live URL is `https://<owner>.github.io/BufoClicker/`.

---

## Common tasks

| Task | Command |
| --- | --- |
| Dev server | `docker compose up dev` |
| Production build → `./dist` | `docker compose run --rm build` |
| Preview `./dist` | `docker compose up site` |
| Type-check only | `docker compose run --rm build npx tsc --noEmit` |
| Dependency audit | `docker compose run --rm build npm audit` |
| Update `package-lock.json` after editing `package.json` | `docker compose run --rm build npm install` |
| Shell in the toolchain container | `docker compose run --rm build bash` |
| Rebuild the image after dependency changes | `docker compose build` |

---

## Project layout

```
src/
  core/         state manager, event bus, shared types
  game/         GameCore, game loop, save/load, top-level Game API
  managers/     generator / upgrade / achievement / explorer managers
  models/       data shapes + pure helpers (generators, upgrades, ...)
  ui/           component framework + concrete components (shop, upgrades, ...)
  utils/        number/format/storage/animation helpers, debug tools
styles/         plain CSS, copied verbatim into the build
assets/         images + JSON data (generators.json, upgrades.json, achievements.json)
webpack.config.js
```

Game content (generators, upgrades, achievements) is data-driven: it is
`fetch()`-ed at runtime from `assets/data/*.json`, so tuning numbers or adding
entries does **not** require a rebuild — just refresh.

Saves live in `localStorage` under the key `bufo_idle_save`.

---

## Running without Docker

Requires **Node.js 20+** and npm.

```bash
npm ci                 # install exact locked dependencies
npm run start          # dev server on http://localhost:9000 (opens a browser)
npm run build          # production build into ./dist
npm run serve:dist     # preview ./dist on http://localhost:8080
npm run deploy         # build + publish ./dist to the gh-pages branch
```
