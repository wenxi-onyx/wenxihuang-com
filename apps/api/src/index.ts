import Fastify from 'fastify';
import cookie from '@fastify/cookie';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { ensureAdminUser } from './auth.js';
import { openDb } from './db.js';
import { ApiError } from './errors.js';
import { registerAdminRoutes } from './routes/admin.js';
import { registerAuthRoutes } from './routes/auth.js';
import { registerEloRoutes } from './routes/elo.js';
import { registerMatchRoutes } from './routes/matches.js';
import { registerPlayerRoutes } from './routes/players.js';
import { registerSeasonRoutes } from './routes/seasons.js';
import { registerUserRoutes } from './routes/user.js';

const here = dirname(fileURLToPath(import.meta.url));

const PORT = Number(process.env.PORT ?? 8080);
const DATABASE_PATH = process.env.DATABASE_PATH ?? join(here, '..', 'data', 'dev.db');
// SvelteKit adapter-node build output (apps/web/build). Optional: when absent
// (local API-only dev), the API runs alone and the web app runs on Vite.
const WEB_BUILD_DIR = process.env.WEB_BUILD_DIR ?? resolve(here, '..', '..', 'web', 'build');

async function main(): Promise<void> {
  const app = Fastify({ logger: true, trustProxy: true });

  const db = openDb(DATABASE_PATH);
  app.log.info(`SQLite database at ${DATABASE_PATH}`);
  await ensureAdminUser(db, (msg) => app.log.warn(msg));

  await app.register(cookie);

  app.setErrorHandler((error: unknown, _request, reply) => {
    if (error instanceof ApiError) {
      reply.code(error.status).send({ error: error.message });
      return;
    }
    const fastifyError = error as { statusCode?: number; message?: string };
    if (fastifyError.statusCode && fastifyError.statusCode < 500) {
      reply.code(fastifyError.statusCode).send({ error: fastifyError.message ?? 'Bad request' });
      return;
    }
    app.log.error(error);
    reply.code(500).send({ error: 'Database error' });
  });

  app.get('/health', async () => ({ status: 'healthy', timestamp: new Date().toISOString() }));

  registerAuthRoutes(app, db);
  registerUserRoutes(app, db);
  registerAdminRoutes(app, db);
  registerPlayerRoutes(app, db);
  registerMatchRoutes(app, db);
  registerEloRoutes(app, db);
  registerSeasonRoutes(app, db);

  // Serve the SvelteKit app from the same process: anything that isn't /api or
  // /health is handed to the adapter-node handler.
  //
  // Silicon San Francisco hook: when SSF lands, it runs as its own process in
  // this machine and requests for its host (ssf.wenxihuang.com) get proxied
  // from here — see CLAUDE.md "Hosting silicon-sanfrancisco".
  if (existsSync(join(WEB_BUILD_DIR, 'handler.js'))) {
    const middie = (await import('@fastify/middie')).default;
    await app.register(middie);
    const { handler } = await import(pathToFileURL(join(WEB_BUILD_DIR, 'handler.js')).href);
    app.use((req, res, next) => {
      if (req.url?.startsWith('/api') || req.url === '/health') return next();
      handler(req, res, next);
    });
    app.log.info(`Serving web app from ${WEB_BUILD_DIR}`);
  } else {
    app.log.warn(`No web build found at ${WEB_BUILD_DIR}; running API only`);
  }

  await app.listen({ port: PORT, host: '0.0.0.0' });
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
