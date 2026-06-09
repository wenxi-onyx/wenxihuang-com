import type { DB } from './db.js';
import { nowIso, uuid } from './util.js';

export type JobStatus = 'pending' | 'running' | 'completed' | 'failed';

export interface Job {
  id: string;
  job_type: string;
  status: JobStatus;
  progress: number;
  total_items: number | null;
  processed_items: number;
  result_data: unknown | null;
  created_by: string | null;
  created_at: string;
  started_at: string | null;
  completed_at: string | null;
}

export function createJob(db: DB, jobType: string, createdBy: string | null): string {
  const id = uuid();
  db.prepare(
    "INSERT INTO jobs (id, job_type, status, created_by, created_at) VALUES (?, ?, 'pending', ?, ?)"
  ).run(id, jobType, createdBy, nowIso());
  return id;
}

export function markJobRunning(db: DB, jobId: string): void {
  db.prepare("UPDATE jobs SET status = 'running', started_at = ? WHERE id = ?").run(nowIso(), jobId);
}

export function markJobFinished(db: DB, jobId: string, status: 'completed' | 'failed', resultData: unknown): void {
  db.prepare('UPDATE jobs SET status = ?, completed_at = ?, progress = 100, result_data = ? WHERE id = ?').run(
    status,
    nowIso(),
    JSON.stringify(resultData ?? null),
    jobId
  );
}

export function updateJobProgressItems(db: DB, jobId: string, processed: number, total: number): void {
  const progress = Math.floor((processed / total) * 100);
  db.prepare('UPDATE jobs SET processed_items = ?, total_items = ?, progress = ? WHERE id = ?').run(
    processed,
    total,
    progress,
    jobId
  );
}

export function getJob(db: DB, jobId: string): Job | undefined {
  const row = db.prepare('SELECT * FROM jobs WHERE id = ?').get(jobId) as
    | (Omit<Job, 'result_data'> & { result_data: string | null })
    | undefined;
  if (!row) return undefined;
  return {
    ...row,
    progress: row.progress ?? 0,
    processed_items: row.processed_items ?? 0,
    result_data: row.result_data == null ? null : JSON.parse(row.result_data),
  };
}
