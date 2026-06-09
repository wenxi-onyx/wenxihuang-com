import argon2 from 'argon2';
import { randomBytes } from 'node:crypto';
import type { FastifyReply, FastifyRequest } from 'fastify';
import type { DB } from './db.js';
import { forbidden, invalidCredentials, sessionExpired, unauthorized } from './errors.js';
import { asBool, nowIso, uuid } from './util.js';

export type UserRole = 'admin' | 'user';

export interface User {
  id: string;
  username: string;
  password_hash: string;
  first_name: string | null;
  last_name: string | null;
  role: UserRole;
  created_at: string;
}

export interface UserInfo {
  id: string;
  username: string;
  first_name: string | null;
  last_name: string | null;
  role: UserRole;
}

export const toUserInfo = (u: User): UserInfo => ({
  id: u.id,
  username: u.username,
  first_name: u.first_name,
  last_name: u.last_name,
  role: u.role,
});

// ----- passwords (argon2id; verifies hashes created by the old Rust backend) -----

export const hashPassword = (password: string): Promise<string> =>
  argon2.hash(password, { type: argon2.argon2id });

export async function verifyPassword(password: string, hash: string): Promise<void> {
  let ok = false;
  try {
    ok = await argon2.verify(hash, password);
  } catch {
    ok = false;
  }
  if (!ok) throw invalidCredentials();
}

// ----- users -----

export const findUserByUsername = (db: DB, username: string): User | undefined =>
  db.prepare('SELECT * FROM users WHERE username = ?').get(username) as User | undefined;

export const findUserById = (db: DB, id: string): User | undefined =>
  db.prepare('SELECT * FROM users WHERE id = ?').get(id) as User | undefined;

export function createUser(
  db: DB,
  username: string,
  passwordHash: string,
  firstName: string | null,
  lastName: string | null,
  role: UserRole
): User {
  const id = uuid();
  db.prepare(
    `INSERT INTO users (id, username, password_hash, first_name, last_name, role, created_at)
     VALUES (?, ?, ?, ?, ?, ?, ?)`
  ).run(id, username, passwordHash, firstName, lastName, role, nowIso());
  return findUserById(db, id)!;
}

/** Create a default admin on first boot if no admin exists (parity with old backend). */
export async function ensureAdminUser(db: DB, log: (msg: string) => void): Promise<void> {
  const row = db.prepare("SELECT COUNT(*) AS n FROM users WHERE role = 'admin'").get() as { n: number };
  if (row.n > 0) return;

  const password = process.env.ADMIN_PASSWORD ?? 'admin';
  if (!process.env.ADMIN_PASSWORD) {
    log("ADMIN_PASSWORD not set, using default password 'admin' — change it immediately!");
  }
  createUser(db, 'admin', await hashPassword(password), 'Admin', 'User', 'admin');
  log('Created default admin user (username: admin)');
}

// ----- sessions -----

const SESSION_DAYS = 30;

export function createSession(db: DB, userId: string): string {
  const sessionId = randomBytes(32).toString('base64');
  const expiresAt = new Date(Date.now() + SESSION_DAYS * 24 * 60 * 60 * 1000).toISOString();
  db.prepare('INSERT INTO sessions (id, user_id, expires_at, last_accessed) VALUES (?, ?, ?, ?)').run(
    sessionId,
    userId,
    expiresAt,
    nowIso()
  );
  return sessionId;
}

export function deleteSession(db: DB, sessionId: string): void {
  db.prepare('DELETE FROM sessions WHERE id = ?').run(sessionId);
}

export function validateSession(db: DB, sessionId: string): User {
  const session = db.prepare('SELECT user_id, expires_at FROM sessions WHERE id = ?').get(sessionId) as
    | { user_id: string; expires_at: string }
    | undefined;
  if (!session) throw unauthorized();

  if (new Date(session.expires_at).getTime() < Date.now()) {
    deleteSession(db, sessionId);
    throw sessionExpired();
  }

  db.prepare('UPDATE sessions SET last_accessed = ? WHERE id = ?').run(nowIso(), sessionId);

  const user = findUserById(db, session.user_id);
  if (!user) throw unauthorized();
  return user;
}

// ----- fastify integration -----

export const SESSION_COOKIE = 'session_id';

export function sessionCookieOptions(maxAgeSeconds: number) {
  return {
    httpOnly: true,
    path: '/',
    sameSite: 'lax' as const,
    secure: process.env.NODE_ENV === 'production',
    maxAge: maxAgeSeconds,
  };
}

export const SESSION_MAX_AGE_SECONDS = SESSION_DAYS * 24 * 60 * 60;

declare module 'fastify' {
  interface FastifyRequest {
    user: User;
  }
}

export function makeAuthHooks(db: DB) {
  const requireAuth = async (request: FastifyRequest, _reply: FastifyReply) => {
    const sessionId = request.cookies[SESSION_COOKIE];
    if (!sessionId) throw unauthorized();
    request.user = validateSession(db, sessionId);
  };

  const requireAdmin = async (request: FastifyRequest, reply: FastifyReply) => {
    await requireAuth(request, reply);
    if (request.user.role !== 'admin') throw forbidden();
  };

  return { requireAuth, requireAdmin };
}

export { asBool };
