# syntax=docker/dockerfile:1

##
## BufoClicker – container image
##
## Targets:
##   dev     -> hot-reloading Trunk dev server (used by `docker compose up dev`)
##   build   -> produces the static site in /app/dist
##   runtime -> tiny nginx image serving the built site (default target)
##

# ---------------------------------------------------------------------------
# Rust + WASM toolchain (pinned compiler, target, and Trunk release).
# ---------------------------------------------------------------------------
FROM rust:1.93.1-bookworm AS rust-toolchain
RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown
# Prebuilt pinned Trunk release (far faster than compiling ~300 crates).
# `uname -m` (aarch64 / x86_64) matches trunk's release asset names.
RUN curl -fsSL "https://github.com/trunk-rs/trunk/releases/download/v0.21.14/trunk-$(uname -m)-unknown-linux-gnu.tar.gz" \
    | tar xz -C /usr/local/bin
WORKDIR /app

# ---------------------------------------------------------------------------
# Development server (hot-reloading Trunk on :9000).
# ---------------------------------------------------------------------------
FROM rust-toolchain AS dev
COPY . .
EXPOSE 9000
CMD ["trunk", "serve", "--address", "0.0.0.0", "--port", "9000"]

# ---------------------------------------------------------------------------
# Production build -> /app/dist
# ---------------------------------------------------------------------------
FROM rust-toolchain AS build
COPY . .
RUN trunk build --release --public-url /BufoClicker/

# ---------------------------------------------------------------------------
# Runtime: serve the static bundle with nginx under /BufoClicker/ (the Pages
# project path), with a root redirect.
# ---------------------------------------------------------------------------
FROM nginx:1.27-alpine AS runtime
COPY --from=build /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
