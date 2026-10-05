#!/usr/bin/env bun

import { DEV_EDGE_PORT, DEV_ES_PORT, DEV_GRAFANA_PORT, DEV_POSTGRES_PORT } from '@qafiyah/config';

const ROOT = `${import.meta.dir}/../..`;

const INSTANCE_BUCKETS = 49;
const OFFSET_STEP = 10;

export function slugify(name: string): string {
  return name
    .toLowerCase()
    .replaceAll(/[^a-z0-9]+/g, '-')
    .replaceAll(/^-+|-+$/g, '');
}

function djb2(value: string): number {
  let hash = 5381;
  for (let i = 0; i < value.length; i++) {
    hash = ((hash << 5) + hash + (value.codePointAt(i) ?? 0)) >>> 0;
  }
  return hash;
}

export function computeOffset(slug: string): number {
  const index = (djb2(slug) % INSTANCE_BUCKETS) + 1;
  return index * OFFSET_STEP;
}

async function runGit(args: readonly string[]): Promise<string> {
  const proc = Bun.spawn(['git', ...args], { cwd: ROOT, stdout: 'pipe', stderr: 'pipe' });
  const [stdout, code] = await Promise.all([new Response(proc.stdout).text(), proc.exited]);
  if (code !== 0) {
    console.error(`[worktree] git ${args.join(' ')} failed`);
    process.exit(1);
  }
  return stdout.trim();
}

export type WorktreeIdentity =
  | { readonly isWorktree: false }
  | { readonly isWorktree: true; readonly slug: string; readonly offset: number };

export async function resolveWorktreeIdentity(): Promise<WorktreeIdentity> {
  const [gitDir, gitCommonDir] = await Promise.all([
    runGit(['rev-parse', '--path-format=absolute', '--git-dir']),
    runGit(['rev-parse', '--path-format=absolute', '--git-common-dir']),
  ]);
  if (gitDir === gitCommonDir) return { isWorktree: false };

  const toplevel = await runGit(['rev-parse', '--show-toplevel']);
  const rawName = toplevel.split('/').pop() ?? '';
  const slug = slugify(rawName) || slugify(toplevel);
  return { isWorktree: true, slug, offset: computeOffset(slug) };
}

export async function primaryCheckoutRoot(): Promise<string> {
  const gitCommonDir = await runGit(['rev-parse', '--path-format=absolute', '--git-common-dir']);
  return gitCommonDir.replace(/\/\.git$/, '');
}

if (import.meta.main) {
  const identity = await resolveWorktreeIdentity();
  if (!identity.isWorktree) {
    console.error(
      '[worktree] --worktree passed but this is the primary checkout, nothing to isolate against'
    );
    process.exit(1);
  }
  const { slug, offset } = identity;
  console.log(
    [
      `COMPOSE_PROJECT_NAME=qafiyah-dev-${slug}`,
      `DEV_SUFFIX=-${slug}`,
      `DEV_POSTGRES_PORT=${DEV_POSTGRES_PORT + offset}`,
      `DEV_ES_PORT=${DEV_ES_PORT + offset}`,
      `DEV_EDGE_PORT=${DEV_EDGE_PORT + offset}`,
      `DEV_GRAFANA_PORT=${DEV_GRAFANA_PORT + offset}`,
    ].join('\n')
  );
}
