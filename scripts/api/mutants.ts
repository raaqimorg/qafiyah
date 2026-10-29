#!/usr/bin/env bun

import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { serviceUrls } from '../dev/service-urls';
import { ROOT } from '../lib/root';

function run(cmd: string[], extra: Record<string, string> = {}): Promise<number> {
  return Bun.spawn(cmd, {
    cwd: ROOT,
    env: { ...process.env, ...extra },
    stdout: 'inherit',
    stderr: 'inherit',
  }).exited;
}

const separator = process.argv.indexOf('--');
const passthrough = separator === -1 ? [] : process.argv.slice(separator + 1);
const all = process.argv.includes('--all');
const base =
  process.argv.find((arg) => arg.startsWith('--base='))?.slice('--base='.length) ?? 'origin/main';

if (Bun.spawnSync(['cargo', 'mutants', '--version']).exitCode !== 0) {
  console.error('cargo-mutants is not installed: cargo install cargo-mutants --locked');
  process.exit(2);
}

const dir = mkdtempSync(join(tmpdir(), 'qafiyah-mutants-'));
const scope: string[] = [];
if (!all) {
  const diff = Bun.spawnSync(['git', 'diff', base, '--', 'apps/api/src'], { cwd: ROOT });
  if (diff.exitCode !== 0) {
    console.error(`cannot diff against ${base}: ${diff.stderr.toString().trim()}`);
    process.exit(2);
  }
  if (diff.stdout.length === 0) {
    console.log(
      `no change in apps/api/src since ${base}, nothing to mutate (--all mutates everything)`
    );
    process.exit(0);
  }
  await Bun.write(join(dir, 'api.diff'), diff.stdout);
  scope.push('--in-diff', join(dir, 'api.diff'));
}

const stack = ['./scripts/dev/compose.sh', 'up', '-d', '--wait', 'db', 'elasticsearch'];
if ((await run(stack)) !== 0) process.exit(1);
const indexer = ['./scripts/dev/compose.sh', 'run', '--rm', '--no-deps', 'search-indexer'];
if ((await run(indexer)) !== 0) process.exit(1);

const urls = serviceUrls(process.env);
const code = await run(
  [
    'cargo',
    'mutants',
    '--package',
    'qafiyah-api',
    '--output',
    'reports/mutants',
    '--no-shuffle',
    ...scope,
    ...passthrough,
  ],
  {
    QAFIYAH_TEST_DATABASE_URL: urls.database,
    QAFIYAH_TEST_DATABASE_URL_ACCOUNTS: urls.accounts,
    QAFIYAH_TEST_ELASTICSEARCH_URL: urls.elasticsearch,
    QAFIYAH_TEST_ELASTICSEARCH_ADMIN_URL: urls.elasticsearchAdmin,
  }
);
rmSync(dir, { recursive: true, force: true });
process.exit(code);
