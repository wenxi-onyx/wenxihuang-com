import type { FastifyInstance } from 'fastify';
import type { DB } from '../db.js';
import {
  SESSION_COOKIE,
  SESSION_MAX_AGE_SECONDS,
  createSession,
  createUser,
  deleteSession,
  findUserByUsername,
  hashPassword,
  makeAuthHooks,
  sessionCookieOptions,
  toUserInfo,
  verifyPassword,
  type UserRole,
} from '../auth.js';
import { invalidCredentials, usernameTaken } from '../errors.js';

interface LoginBody {
  username: string;
  password: string;
}

interface RegisterBody {
  username: string;
  password: string;
  role: UserRole;
}

export function registerAuthRoutes(app: FastifyInstance, db: DB): void {
  const { requireAuth, requireAdmin } = makeAuthHooks(db);

  app.post<{ Body: LoginBody }>('/api/auth/login', async (request, reply) => {
    const { username, password } = request.body;
    const user = findUserByUsername(db, username);
    if (!user) throw invalidCredentials();
    await verifyPassword(password, user.password_hash);

    const sessionId = createSession(db, user.id);
    reply.setCookie(SESSION_COOKIE, sessionId, sessionCookieOptions(SESSION_MAX_AGE_SECONDS));
    return { user: toUserInfo(user) };
  });

  app.post('/api/auth/logout', { preHandler: requireAuth }, async (request, reply) => {
    const sessionId = request.cookies[SESSION_COOKIE];
    if (sessionId) deleteSession(db, sessionId);
    reply.setCookie(SESSION_COOKIE, '', sessionCookieOptions(0));
    return { message: 'Logged out successfully' };
  });

  app.get('/api/auth/me', { preHandler: requireAuth }, async (request) => {
    return { user: toUserInfo(request.user) };
  });

  app.post<{ Body: RegisterBody }>('/api/auth/register', { preHandler: requireAdmin }, async (request) => {
    const { username, password, role } = request.body;
    if (findUserByUsername(db, username)) throw usernameTaken();
    const user = createUser(db, username, await hashPassword(password), null, null, role);
    return { user: toUserInfo(user) };
  });
}
