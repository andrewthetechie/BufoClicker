# syntax=docker/dockerfile:1

##
## BufoClicker – container image
##
## Targets:
##   dev     -> hot-reloading webpack-dev-server (used by `docker compose up dev`)
##   build   -> produces the static site in /app/dist
##   runtime -> tiny nginx image serving the built site (default target)
##

# ---------------------------------------------------------------------------
# Shared base with dependencies installed
# ---------------------------------------------------------------------------
FROM node:22-bookworm-slim AS deps
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --no-audit --no-fund

# ---------------------------------------------------------------------------
# Toolchain image (dev server + one-off build/lint/audit commands).
# Runs as the unprivileged `node` user (uid 1000) so files written back to a
# bind-mounted host directory (e.g. ./dist) stay owned by a normal user.
# ---------------------------------------------------------------------------
FROM deps AS dev
WORKDIR /app
COPY . .
RUN chown -R node:node /app
USER node
EXPOSE 9000
CMD ["npm", "run", "dev"]

# ---------------------------------------------------------------------------
# Production build -> /app/dist
# ---------------------------------------------------------------------------
FROM deps AS build
WORKDIR /app
COPY . .
RUN npm run build

# ---------------------------------------------------------------------------
# Runtime: serve the static bundle with nginx
# ---------------------------------------------------------------------------
FROM nginx:1.27-alpine AS runtime
COPY --from=build /app/dist /usr/share/nginx/html
EXPOSE 80
# nginx's default config already serves /usr/share/nginx/html with index.html
