#!/usr/bin/env bun

import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { ROOT } from '../lib/root';

const OASDIFF_IMAGE = 'tufin/oasdiff:v1.33.0';
const SPEC = 'apps/api/generated/openapi/openapi.json';

const base =
  process.argv.find((arg) => arg.startsWith('--base='))?.slice('--base='.length) ?? 'origin/main';

const show = Bun.spawnSync(['git', 'show', `${base}:${SPEC}`], { cwd: ROOT });
if (show.exitCode !== 0) {
  console.error(`cannot read ${SPEC} at ${base}: ${show.stderr.toString().trim()}`);
  process.exit(2);
}

const dir = mkdtempSync(join(tmpdir(), 'qafiyah-oasdiff-'));
await Bun.write(join(dir, 'base.json'), show.stdout);

console.log(`breaking changes in ${SPEC} since ${base}`);
const oasdiff = Bun.spawn(
  [
    'docker',
    'run',
    '--rm',
    '--volume',
    `${dir}/base.json:/specs/base.json:ro`,
    '--volume',
    `${ROOT}/${SPEC}:/specs/revision.json:ro`,
    OASDIFF_IMAGE,
    'breaking',
    '/specs/base.json',
    '/specs/revision.json',
    '--fail-on',
    'ERR',
  ],
  { stdout: 'inherit', stderr: 'inherit' }
);
const code = await oasdiff.exited;
rmSync(dir, { recursive: true, force: true });
process.exit(code);
