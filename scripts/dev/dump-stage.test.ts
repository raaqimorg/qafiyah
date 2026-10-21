import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { dumpChoice, holdsDump } from './dump-stage';

describe('dumpChoice', () => {
  test('picks the sample when no dump is asked for', () => {
    expect(dumpChoice(['bun', 'run.ts'])).toEqual({ kind: 'sample' });
  });

  test('picks the snapshot folder after --dump', () => {
    expect(dumpChoice(['bun', 'run.ts', '--dump', '0046_15_10_2026'])).toEqual({
      kind: 'snapshot',
      folder: '0046_15_10_2026',
    });
  });

  test('picks the sample when --dump names it', () => {
    expect(dumpChoice(['bun', 'run.ts', '--dump', '0000_default'])).toEqual({ kind: 'sample' });
  });

  test('refuses --dump without a folder', () => {
    expect(dumpChoice(['bun', 'run.ts', '--dump']).kind).toBe('invalid');
    expect(dumpChoice(['bun', 'run.ts', '--dump', '--inspector']).kind).toBe('invalid');
  });

  test('refuses a name that is not a dump folder', () => {
    expect(dumpChoice(['bun', 'run.ts', '--dump', '../secrets']).kind).toBe('invalid');
  });
});

describe('holdsDump', () => {
  let dir = '';

  beforeEach(async () => {
    dir = await mkdtemp(join(tmpdir(), 'dump-stage-test-'));
  });

  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  test('finds a whole dump', async () => {
    await Bun.write(join(dir, 'qafiyah_public_20261015_040335.dump'), '');
    expect(holdsDump(dir)).toBe(true);
  });

  test('finds a split dump', async () => {
    await Bun.write(join(dir, 'qafiyah_public_20260923_102453.dump.part-aa'), '');
    expect(holdsDump(dir)).toBe(true);
  });

  test('finds nothing in a folder without a dump, or in a missing folder', async () => {
    await Bun.write(join(dir, 'CHANGES.md'), '');
    expect(holdsDump(dir)).toBe(false);
    expect(holdsDump(join(dir, 'missing'))).toBe(false);
  });
});
