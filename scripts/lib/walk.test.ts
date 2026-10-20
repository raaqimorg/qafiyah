import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { walkFiles } from './walk';

let dir: string;

beforeEach(async () => {
  dir = await mkdtemp(join(tmpdir(), 'walk-test-'));
});

afterEach(async () => {
  await rm(dir, { recursive: true, force: true });
});

describe('walkFiles', () => {
  test('skips the worktrees under .worktrees', async () => {
    await Bun.write(join(dir, 'apps/web/main.ts'), '');
    await Bun.write(join(dir, '.worktrees/feature/apps/web/main.ts'), '');

    expect([...walkFiles(dir)]).toEqual([join(dir, 'apps/web/main.ts')]);
  });
});
