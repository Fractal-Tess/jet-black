FROM oven/bun:1.3.12 AS deps

WORKDIR /app

COPY package.json bun.lock ./
COPY apps/web/package.json apps/web/package.json
COPY apps/worker/package.json apps/worker/package.json
COPY databases/convex/package.json databases/convex/package.json
COPY packages/shared/package.json packages/shared/package.json
COPY packages/typescript-config/package.json packages/typescript-config/package.json
COPY packages/ui/package.json packages/ui/package.json

RUN bun install --frozen-lockfile --ignore-scripts

FROM deps AS runner

WORKDIR /app

COPY . .

WORKDIR /app/apps/web

ENV NODE_ENV=production
ENV PORT=3000
ENV HOSTNAME=0.0.0.0

RUN bun run build

EXPOSE 3000

CMD ["node", "build"]
