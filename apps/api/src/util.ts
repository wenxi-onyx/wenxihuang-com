import { randomUUID } from 'node:crypto';

export const uuid = (): string => randomUUID();

export const nowIso = (): string => new Date().toISOString();

/** Normalize any incoming date representation to canonical ISO-8601 UTC. */
export function toIso(value: string | Date): string {
  const d = value instanceof Date ? value : new Date(value);
  if (Number.isNaN(d.getTime())) {
    throw new Error(`Invalid date: ${value}`);
  }
  return d.toISOString();
}

/** SQLite stores booleans as 0/1; JSON responses need real booleans. */
export const asBool = (v: unknown): boolean => v === 1 || v === true;
