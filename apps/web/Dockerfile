FROM oven/bun:1.3.12 AS base

WORKDIR /app

FROM base AS pruner

RUN bun install --global turbo@2.9.16

COPY . .

RUN turbo prune @workspace/web --docker

FROM base AS builder

COPY --from=pruner /app/out/json/ .

RUN bun install --frozen-lockfile --ignore-scripts

COPY --from=pruner /app/out/full/ .

RUN bunx turbo run build --filter=@workspace/web

FROM base AS runner

ENV HOST=0.0.0.0
ENV NODE_ENV=production
ENV PORT=3000

COPY --from=builder /app /app

WORKDIR /app/apps/web

EXPOSE 3000

CMD ["bun", "build/index.js"]
