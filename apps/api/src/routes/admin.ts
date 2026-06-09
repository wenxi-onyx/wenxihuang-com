import type { FastifyInstance } from 'fastify';
import type { DB } from '../db.js';
import { createUser, findUserByUsername, hashPassword, makeAuthHooks, toUserInfo, type UserRole } from '../auth.js';
import { invalidInput, usernameTaken } from '../errors.js';

interface CreateUserBody {
  username: string;
  password: string;
  first_name?: string | null;
  last_name?: string | null;
  role: UserRole;
}

export function registerAdminRoutes(app: FastifyInstance, db: DB): void {
  const { requireAdmin } = makeAuthHooks(db);

  app.post<{ Body: CreateUserBody }>('/api/admin/users', { preHandler: requireAdmin }, async (request) => {
    const { username, password, first_name = null, last_name = null, role } = request.body;

    if (username.length < 3 || username.length > 20) throw invalidInput('Username must be 3-20 characters');
    if (password.length < 6) throw invalidInput('Password must be at least 6 characters');
    for (const [name, label] of [
      [first_name, 'First name'],
      [last_name, 'Last name'],
    ] as const) {
      if (name != null) {
        if (name.trim().length === 0) throw invalidInput(`${label} cannot be empty`);
        if (name.length > 50) throw invalidInput(`${label} must be 50 characters or less`);
      }
    }
    if (findUserByUsername(db, username)) throw usernameTaken();

    const user = createUser(db, username, await hashPassword(password), first_name, last_name, role);
    return { message: 'User created successfully', user: toUserInfo(user) };
  });
}
