# Single image serving wenxihuang.com: Fastify API (+ SQLite on the /data
# volume) and the SvelteKit site, one Node process.
FROM node:22-slim AS build
RUN corepack enable
WORKDIR /app
COPY pnpm-lock.yaml pnpm-workspace.yaml package.json ./
COPY apps/api/package.json apps/api/
COPY apps/web/package.json apps/web/
RUN pnpm install --frozen-lockfile
COPY apps ./apps
RUN pnpm --filter web build && pnpm --filter api build
RUN pnpm prune --prod

FROM node:22-slim
WORKDIR /app
ENV NODE_ENV=production
COPY --from=build /app .
EXPOSE 8080
CMD ["node", "apps/api/dist/index.js"]
