#!/usr/bin/env bun

import { serviceUrls } from './service-urls';
import { resolveWorktreeIdentity } from './worktree';

const ROOT = `${import.meta.dir}/../..`;

const identity = await resolveWorktreeIdentity();
const isolating = process.argv.includes('--worktree') && identity.isWorktree;
const offset = isolating && identity.isWorktree ? identity.offset : 0;
const composeArgs = isolating ? ['--worktree'] : [];

function run(cmd: string[], extra: Record<string, string> = {}): Promise<number> {
  const proc = Bun.spawn(cmd, {
    cwd: ROOT,
    env: { ...process.env, ...extra },
    stdout: 'inherit',
    stderr: 'inherit',
  });
  return proc.exited;
}

const stack = ['./scripts/dev/compose.sh', 'up', '-d', '--wait', 'db', 'elasticsearch'];
if ((await run([...stack, ...composeArgs])) !== 0) process.exit(1);
const indexer = ['./scripts/dev/compose.sh', 'run', '--rm', '--no-deps', 'search-indexer'];
if ((await run([...indexer, ...composeArgs])) !== 0) process.exit(1);

const urls = serviceUrls({ env: process.env, offset });

const api = await run(['cargo', 'test', '-p', 'qafiyah-api', '--locked', '--test', 'db'], {
  QAFIYAH_TEST_DATABASE_URL: urls.database,
  QAFIYAH_TEST_DATABASE_URL_ACCOUNTS: urls.accounts,
  QAFIYAH_TEST_ELASTICSEARCH_URL: urls.elasticsearch,
  QAFIYAH_TEST_ELASTICSEARCH_ADMIN_URL: urls.elasticsearchAdmin,
});
if (api !== 0) process.exit(api);

process.exit(
  await run(['cargo', 'test', '-p', 'qafiyah-search-indexer', '--locked'], {
    QAFIYAH_TEST_DATABASE_URL: urls.database,
    QAFIYAH_TEST_ELASTICSEARCH_ADMIN_URL: urls.elasticsearchAdmin,
  })
);
