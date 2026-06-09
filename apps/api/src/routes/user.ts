import type { FastifyInstance } from 'fastify';
import type { DB } from '../db.js';
import { findUserById, findUserByUsername, hashPassword, makeAuthHooks, toUserInfo, verifyPassword } from '../auth.js';
import { invalidInput, usernameTaken } from '../errors.js';

interface UpdateProfileBody {
  username: string;
  first_name?: string | null;
  last_name?: string | null;
}

interface ChangePasswordBody {
  current_password: string;
  new_password: string;
}

function validateName(name: string | null | undefined, label: string): void {
  if (name == null) return;
  if (name.trim().length === 0) throw invalidInput(`${label} cannot be empty`);
  if (name.length > 50) throw invalidInput(`${label} must be 50 characters or less`);
}

export function registerUserRoutes(app: FastifyInstance, db: DB): void {
  const { requireAuth } = makeAuthHooks(db);

  app.get('/api/user/profile', { preHandler: requireAuth }, async (request) => {
    return { user: toUserInfo(request.user) };
  });

  app.put<{ Body: UpdateProfileBody }>('/api/user/profile', { preHandler: requireAuth }, async (request) => {
    const { username, first_name = null, last_name = null } = request.body;

    if (username.length < 3 || username.length > 20) {
      throw invalidInput('Username must be 3-20 characters');
    }
    if (username !== request.user.username && findUserByUsername(db, username)) {
      throw usernameTaken();
    }
    validateName(first_name, 'First name');
    validateName(last_name, 'Last name');

    db.prepare('UPDATE users SET username = ?, first_name = ?, last_name = ? WHERE id = ?').run(
      username,
      first_name,
      last_name,
      request.user.id
    );
    return { user: toUserInfo(findUserById(db, request.user.id)!) };
  });

  app.post<{ Body: ChangePasswordBody }>(
    '/api/user/change-password',
    { preHandler: requireAuth },
    async (request) => {
      const { current_password, new_password } = request.body;
      await verifyPassword(current_password, request.user.password_hash);
      if (new_password.length < 6) throw invalidInput('Password must be at least 6 characters');

      db.prepare('UPDATE users SET password_hash = ? WHERE id = ?').run(
        await hashPassword(new_password),
        request.user.id
      );
      return { message: 'Password changed successfully' };
    }
  );
}
