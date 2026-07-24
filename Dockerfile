FROM oven/bun:1.3.14 AS web
WORKDIR /src
COPY package.json bun.lock bunfig.toml ./
COPY apps/web/package.json apps/web/package.json
COPY packages/shared/package.json packages/shared/package.json
COPY packages/typescript-config/package.json packages/typescript-config/package.json
RUN bun install --frozen-lockfile
COPY apps/web apps/web
COPY packages/shared packages/shared
COPY packages/typescript-config packages/typescript-config
RUN bun run --cwd apps/web build

FROM rust:1.86-bookworm AS rust
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY apps/jet-black apps/jet-black
COPY apps/desktop/src-tauri apps/desktop/src-tauri
COPY apps/worker apps/worker
COPY crates crates
COPY packages/shared packages/shared
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --locked --release -p jet-black -p jet-black-worker

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git \
    && rm -rf /var/lib/apt/lists/*
COPY --from=rust /src/target/release/jet-black /usr/local/bin/jet-black
COPY --from=rust /src/target/release/jet-black-worker /usr/local/bin/jet-black-worker
COPY --from=web /src/apps/web/build-client /opt/jet-black/web
ENV JET_BLACK_PROFILE=server
ENV JET_BLACK_BIND=0.0.0.0:4317
ENV JET_BLACK_STATIC_ASSETS_DIR=/opt/jet-black/web
ENV JET_BLACK_DATA_DIR=/var/lib/jet-black
EXPOSE 4317
VOLUME ["/var/lib/jet-black"]
ENTRYPOINT ["jet-black"]
