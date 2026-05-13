# syntax=docker/dockerfile:1.7

# ===== rust-build =====
FROM rust:1-bookworm AS rust-build
WORKDIR /src
COPY backend/Cargo.toml backend/Cargo.lock ./backend/
COPY backend/migrations ./backend/migrations
COPY backend/src ./backend/src
COPY backend/tests ./backend/tests
WORKDIR /src/backend
RUN cargo build --release --bins

# ===== frontend-build =====
FROM node:22-bookworm-slim AS frontend-build
RUN npm install -g pnpm@9.12.0
WORKDIR /src/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY frontend/ ./
RUN pnpm build

# ===== runtime =====
FROM debian:bookworm-slim
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl bash bash-completion less vim-tiny git tini procps \
    debian-keyring debian-archive-keyring apt-transport-https gnupg \
    && curl -fsSL "https://dl.cloudsmith.io/public/caddy/stable/gpg.key" | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg \
    && curl -fsSL "https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt" > /etc/apt/sources.list.d/caddy-stable.list \
    && apt-get update && apt-get install -y --no-install-recommends caddy \
    && curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/* \
    && npm install -g --omit=dev @anthropic-ai/claude-code @openai/codex \
    && npm cache clean --force

COPY --from=rust-build /src/backend/target/release/hangard /usr/local/bin/hangard
COPY --from=rust-build /src/backend/target/release/hangar-supervisor /usr/local/bin/hangar-supervisor
COPY --from=frontend-build /src/frontend/build /var/www/hangar
COPY deploy/docker/Caddyfile /etc/caddy/Caddyfile
COPY deploy/docker/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Non-root user so `claude --dangerously-skip-permissions` and similar tools
# that refuse to run as root work cleanly. UID 1000 is the Linux convention.
RUN useradd -m -u 1000 -s /bin/bash hangar \
    && mkdir -p /state \
    && chown -R hangar:hangar /state /home/hangar
USER hangar
# Default cwd for spawned sessions. `/code` is where the host's
# ~/Documents/code bind mount lands, so sessions start sitting in the
# projects parent dir and the user is one `cd <project>` away.
WORKDIR /code

ENV HANGAR_STATE_DIR=/state \
    HANGAR_PORT=3000 \
    HOME=/home/hangar \
    SHELL=/bin/bash \
    TERM=xterm-256color \
    RUST_LOG=info
EXPOSE 8080
VOLUME ["/state"]

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/entrypoint.sh"]
