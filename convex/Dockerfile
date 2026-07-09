FROM ghcr.io/get-convex/convex-backend:1bd6910bc0b4066a918809912670d42afc8fdfb0 AS convex-tools

FROM oven/bun:1.3.12 AS base

WORKDIR /app

FROM base AS pruner

RUN bun install --global turbo@2.9.16

COPY . .

RUN turbo prune @workspace/convex --docker

FROM base AS installer

COPY --from=pruner /app/out/json/ .

RUN bun install --frozen-lockfile --ignore-scripts

COPY --from=pruner /app/out/full/ .

FROM node:24-trixie-slim AS migrator

COPY --from=installer /app /app
COPY --from=convex-tools /convex/generate_key /usr/local/bin/convex-generate-key

WORKDIR /app/convex

CMD ["node", "migrator.ts"]
