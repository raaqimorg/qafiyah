#!/usr/bin/env bun

import { existsSync } from 'node:fs';
import { tmpdir } from 'node:os';

import { type CheckOutcome, type DoctorEnv, runChecks } from './doctor-checks';
import { formatReport, hasFailure } from './doctor-report';

const ROOT = `${import.meta.dir}/../..`;
const PROBE_TIMEOUT_MS = 10_000;

const supportsColor = process.stdout.isTTY && process.env['NO_COLOR'] === undefined;
const ESC = String.fromCodePoint(27);
const paint = (code: number) => (text: string) =>
  supportsColor ? `${ESC}[${code}m${text}${ESC}[0m` : text;
const STYLE = {
  green: paint(32),
  red: paint(31),
  yellow: paint(33),
  dim: paint(2),
  bold: paint(1),
};

async function probe(command: readonly string[]): ReturnType<DoctorEnv['probe']> {
  const [name, ...args] = command;
  if (name === undefined || Bun.which(name) === null) return undefined;
  const proc = Bun.spawn([name, ...args], {
    cwd: tmpdir(),
    stdout: 'pipe',
    stderr: 'pipe',
    timeout: PROBE_TIMEOUT_MS,
  });
  const [stdout, stderr, code] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ]);
  return { code, output: `${stdout}${stderr}` };
}

function stringAt(value: unknown, path: readonly string[]): string | undefined {
  let current: unknown = value;
  for (const key of path) {
    if (typeof current !== 'object' || current === null) return undefined;
    current = Reflect.get(current, key);
  }
  return typeof current === 'string' ? current : undefined;
}

async function readToml(path: string): Promise<unknown> {
  const file = Bun.file(path);
  return (await file.exists()) ? Bun.TOML.parse(await file.text()) : undefined;
}

async function readEnv(): Promise<DoctorEnv> {
  const packageJson: unknown = await Bun.file(`${ROOT}/package.json`).json();
  const toolchain = await readToml(`${ROOT}/rust-toolchain.toml`);
  return {
    platform: process.platform,
    bunVersion: Bun.version,
    bunMinimum: stringAt(packageJson, ['engines', 'bun']),
    rustChannel: stringAt(toolchain, ['toolchain', 'channel']),
    hasNodeModules: existsSync(`${ROOT}/node_modules`),
    which: (name) => Bun.which(name) !== null,
    probe,
  };
}

export async function checkPrerequisites(
  groups: readonly CheckOutcome['group'][]
): Promise<readonly CheckOutcome[]> {
  return await runChecks(await readEnv(), groups);
}

if (import.meta.main) {
  const outcomes = await checkPrerequisites(['run', 'commit', 'optional']);
  console.log(formatReport(outcomes, STYLE));
  if (hasFailure(outcomes)) {
    console.log(`\n${STYLE.red('✗')} fix the items marked ✗, then run bun run doctor again`);
    process.exit(1);
  }
  console.log(`\n${STYLE.green('✓')} ready for bun run dev`);
}
