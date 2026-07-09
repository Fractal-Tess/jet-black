FROM oven/bun:1.3.12 AS deps

WORKDIR /app

COPY package.json bun.lock ./
COPY apps/web/package.json apps/web/package.json
COPY apps/worker/package.json apps/worker/package.json
COPY convex/package.json convex/package.json
COPY packages/shared/package.json packages/shared/package.json
COPY packages/typescript-config/package.json packages/typescript-config/package.json
COPY packages/ui/package.json packages/ui/package.json

RUN bun install --frozen-lockfile --ignore-scripts

FROM deps AS runner

WORKDIR /app

COPY . .

WORKDIR /app/apps/worker

ENV NODE_ENV=production
ENV PORT=4000

EXPOSE 4000

CMD ["bun", "run", "index.ts"]
