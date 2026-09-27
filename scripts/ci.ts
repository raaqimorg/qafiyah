#!/usr/bin/env bun

import { selectPhases, type Phase, type Task } from './ci/phases';
import { resolveWorktreeIdentity } from './dev/worktree';

import type { Subprocess } from 'bun';

const ROOT = `${import.meta.dir}/..`;

const args = process.argv.slice(2);
const noDocker = args.includes('--no-docker');
const dockerOnly = args.includes('--docker-only');
const concurrencyArg = args.indexOf('--concurrency');
const requestedConcurrency =
  concurrencyArg >= 0 ? Number(args[concurrencyArg + 1] ?? Number.NaN) : Number.NaN;
if (concurrencyArg >= 0 && !(Number.isInteger(requestedConcurrency) && requestedConcurrency > 0)) {
  console.error(`--concurrency needs a positive integer (got "${args[concurrencyArg + 1]}")`);
  process.exit(2);
}

const phaseArg = args.indexOf('--phase');
const requestedPhase = phaseArg >= 0 ? args[phaseArg + 1] : undefined;

const supportsColor = process.stdout.isTTY && process.env['NO_COLOR'] === undefined;
const ESC = String.fromCodePoint(27);
const paint = (code: number) => (text: string) =>
  supportsColor ? `${ESC}[${code}m${text}${ESC}[0m` : text;
const dim = paint(2);
const bold = paint(1);
const green = paint(32);
const red = paint(31);
const yellow = paint(33);

const identity = await resolveWorktreeIdentity();
if (identity.isWorktree && !args.includes('--worktree')) {
  console.warn(
    yellow(
      `running in worktree '${identity.slug}' without --worktree, ports and containers will collide with the primary checkout`
    )
  );
}
const smokeArgs = identity.isWorktree ? ['--worktree'] : [];

const STATIC: Task[] = [
  { name: 'lint', cmd: ['bun', 'run', 'lint:check'] },
  { name: 'format', cmd: ['bun', 'run', 'format:check'] },
  { name: 'shellcheck', cmd: ['bun', 'run', 'check:shell'] },
  { name: 'actionlint', cmd: ['bun', 'run', 'check:workflows'] },
  { name: 'hadolint', cmd: ['bun', 'run', 'check:dockerfiles'] },
  { name: 'sql', cmd: ['bun', 'run', 'check:sql'] },
];

const CHECKS: Task[] = [
  { name: 'types', cmd: ['bun', 'run', 'types'] },
  { name: 'types-scripts', cmd: ['bun', 'run', 'types:scripts'] },
  { name: 'knip', cmd: ['bun', 'run', 'knip'] },
  { name: 'boundaries', cmd: ['bun', 'run', 'check:boundaries'] },
  { name: 'naming', cmd: ['bun', 'run', 'check:naming'] },
  { name: 'no-parent-imports', cmd: ['bun', 'run', 'check:no-parent-imports'] },
  { name: 'constants', cmd: ['bun', 'run', 'check:constants'] },
  { name: 'secrets', cmd: ['bun', 'run', 'secrets:check'] },
  { name: 'syncpack', cmd: ['bun', 'run', 'check:syncpack'] },
  { name: 'rust-toolchain', cmd: ['bun', 'run', 'check:rust-toolchain'] },
  { name: 'depcruise', cmd: ['bun', 'run', 'depcruise'] },
  { name: 'audit', cmd: ['bun', 'audit'], advisory: true },
];

const UNIT: Task[] = [
  { name: 'test', cmd: ['bun', 'run', 'test'] },
  { name: 'test-scripts', cmd: ['bun', 'run', 'test:scripts'] },
  { name: 'rust-fmt', cmd: ['bun', 'run', 'rust:fmt'] },
  { name: 'rust-lint', cmd: ['bun', 'run', 'rust:lint'] },
  { name: 'rust-test', cmd: ['bun', 'run', 'rust:test'] },
];

const CONTRACT: Task[] = [
  { name: 'openapi', cmd: ['bun', 'run', 'openapi:check'] },
  { name: 'openapi-types', cmd: ['bun', 'run', 'openapi:types:check'] },
  { name: 'well-known', cmd: ['bun', 'run', 'well-known:generate:check'] },
  { name: 'es-query', cmd: ['bun', 'run', 'es:query:check'] },
];

const PHASES: readonly Phase[] = [
  { name: 'static', tasks: STATIC, kind: 'parallel' },
  { name: 'checks', tasks: CHECKS, kind: 'parallel' },
  { name: 'unit', tasks: UNIT, kind: 'parallel' },
  { name: 'contract', tasks: CONTRACT, kind: 'parallel' },
  {
    name: 'db',
    tasks: [{ name: 'db', cmd: ['bun', 'run', 'rust:test:db', ...smokeArgs] }],
    kind: 'docker',
  },
  {
    name: 'origin',
    tasks: [{ name: 'origin', cmd: ['bun', 'run', 'smoke:dev', ...smokeArgs] }],
    kind: 'docker',
  },
  {
    name: 'stack',
    tasks: [{ name: 'stack', cmd: ['bun', 'run', 'smoke:stack', ...smokeArgs] }],
    kind: 'docker',
  },
];

const formatMs = (ms: number) => `${(ms / 1000).toFixed(1)}s`;

function isTurboBoilerplate(line: string): boolean {
  const trimmed = line.trim();
  if (trimmed === '') return true;
  if (/^─+$/.test(trimmed)) return true;
  const prefixes = [
    'Attention:',
    'Turborepo',
    '•',
    'Follow @turborepo',
    'codemod@latest',
    'Tasks:',
    'Cached:',
    'Time:',
    'cache bypass',
    'cache miss',
    'cache hit',
    '$ ',
  ];
  return prefixes.some((prefix) => trimmed.startsWith(prefix));
}

function isAstroBoilerplate(content: string): boolean {
  const trimmed = content.trim();
  if (trimmed === '') return true;
  if (trimmed.startsWith('┃') || trimmed.startsWith('┏') || trimmed.startsWith('┗')) return true;
  if (/^astro\s+v[\d.]+ ready in/.test(trimmed)) return true;
  if (trimmed.includes('[vite] connected.')) return true;
  if (trimmed.includes('[types] Generated')) return true;
  if (trimmed.includes('Enabling sessions with filesystem storage')) return true;
  if (trimmed.includes('watching for file changes')) return true;
  return false;
}

const ANSI_PATTERN = new RegExp(`${ESC}\\[[0-9;]*m`, 'g');
const stripAnsi = (text: string): string => text.replaceAll(ANSI_PATTERN, '');

function isBunCliBoilerplate(line: string): boolean {
  const trimmed = stripAnsi(line).trim();
  if (/^bun \S+ v[\d.]+ \([0-9a-f]+\)$/.test(trimmed)) return true;
  if (/^\[\d+(\.\d+)?ms\] "[^"]*"$/.test(trimmed)) return true;
  return false;
}

const TASK_PREFIX = /^@[\w.-]+\/([\w.-]+):[\w-]+:\s?/;

function cleanTaskOutput(output: string): string {
  return output
    .split('\n')
    .filter((line) => !isTurboBoilerplate(line) && !isBunCliBoilerplate(line))
    .map((line) => {
      const match = line.match(TASK_PREFIX);
      if (!match) return line;
      const pkg = match[1] ?? '';
      const content = line.slice(match[0].length);
      if (
        isTurboBoilerplate(content) ||
        isAstroBoilerplate(content) ||
        isBunCliBoilerplate(content)
      )
        return undefined;
      return `${dim(pkg)} │ ${content}`;
    })
    .filter((line): line is string => line !== undefined)
    .join('\n')
    .trim();
}

async function ensureDockerRunning(): Promise<void> {
  process.stdout.write(`${bold('▶')} docker... `);
  const dockerRunning = await Bun.spawn(['docker', 'info'], {
    cwd: ROOT,
    stdout: 'ignore',
    stderr: 'ignore',
  })
    .exited.then((code) => code === 0)
    .catch(() => false);
  if (dockerRunning) {
    console.log(green('✓'));
    return;
  }
  console.log(red('✗'));
  console.error(red('docker is not running, start Docker (OrbStack/Docker Desktop) and retry'));
  process.exit(1);
}

type Result = { name: string; code: number; output: string; ms: number };

async function runSequential(tasks: Task[]): Promise<void> {
  for (const task of tasks) {
    const start = Date.now();
    process.stdout.write(`${bold('▶')} ${task.name}... `);
    const proc = Bun.spawn(task.cmd, { cwd: ROOT, stdout: 'pipe', stderr: 'pipe' });
    const [stdout, stderr, code] = await Promise.all([
      new Response(proc.stdout).text(),
      new Response(proc.stderr).text(),
      proc.exited,
    ]);
    const ms = Date.now() - start;
    if (code !== 0) {
      console.error(`\n${red('✗')} ${task.name} failed ${dim(`(${formatMs(ms)})`)}`);
      const output = cleanTaskOutput([stdout, stderr].filter(Boolean).join('\n'));
      if (output) console.error(output);
      process.exit(code || 1);
    }
    console.log(`${green('✓')} ${dim(`(${formatMs(ms)})`)}`);
  }
}

async function runOne(task: Task, procs: Map<string, Subprocess>): Promise<Result> {
  const start = Date.now();
  const proc = Bun.spawn(task.cmd, { cwd: ROOT, stdout: 'pipe', stderr: 'pipe' });
  procs.set(task.name, proc);
  const [stdout, stderr, code] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ]);
  procs.delete(task.name);
  return {
    name: task.name,
    code,
    output: cleanTaskOutput([stdout, stderr].filter(Boolean).join('\n')),
    ms: Date.now() - start,
  };
}

async function cleanupOrphans(): Promise<void> {
  await Bun.spawn(['bun', 'run', 'clean'], {
    cwd: ROOT,
    stdout: 'ignore',
    stderr: 'ignore',
  }).exited;
}

async function runParallel(tasks: Task[], limit: number): Promise<Result[]> {
  const procs = new Map<string, Subprocess>();
  const queue = [...tasks];
  const inflight = new Map<Promise<Result>, Task>();
  const warnings: Result[] = [];
  let firstFailure: Result | null = null;

  const fill = (): void => {
    while (inflight.size < limit && queue.length > 0) {
      const task = queue.shift();
      if (!task) break;
      inflight.set(runOne(task, procs), task);
    }
  };

  fill();
  while (inflight.size > 0) {
    const settled = await Promise.race(
      [...inflight.keys()].map((promise) =>
        promise.then((settledResult) => [promise, settledResult] as const)
      )
    );
    const task = inflight.get(settled[0]);
    inflight.delete(settled[0]);
    const result = settled[1];
    if (result.code !== 0 && task?.advisory) {
      warnings.push(result);
      console.log(`${yellow('⚠')} ${result.name} ${dim(`(${formatMs(result.ms)}, non-blocking)`)}`);
    } else if (result.code === 0) {
      console.log(`${green('✓')} ${result.name} ${dim(`(${formatMs(result.ms)})`)}`);
    } else {
      firstFailure = result;
      for (const proc of procs.values()) proc.kill();
      break;
    }
    fill();
  }

  if (firstFailure) {
    await Promise.allSettled(inflight.keys());
    await cleanupOrphans();
    console.error(
      `\n${red('✗')} ${firstFailure.name} failed ${dim(`(${formatMs(firstFailure.ms)})`)}`
    );
    if (firstFailure.output) console.error(firstFailure.output);
    process.exit(firstFailure.code || 1);
  }

  return warnings;
}

const totalStart = Date.now();
const selection = selectPhases(PHASES, {
  ...(requestedPhase === undefined ? {} : { phase: requestedPhase }),
  noDocker,
  dockerOnly,
});
if (selection.isErr()) {
  console.error(selection.error);
  process.exit(2);
}
const phases = selection.value;
if (phases.some((phase) => phase.kind === 'docker')) {
  console.log(dim('── preflight ──'));
  await ensureDockerRunning();
}

const warnings: Result[] = [];
for (const phase of phases) {
  const label = { parallel: 'parallel', docker: 'docker (sequential)' }[phase.kind];
  console.log(dim(`\n── ${phase.name} (${label}) ──`));
  if (phase.kind === 'docker') {
    await runSequential([...phase.tasks]);
  } else {
    const concurrency = Number.isInteger(requestedConcurrency)
      ? requestedConcurrency
      : phase.tasks.length;
    const phaseWarnings = await runParallel([...phase.tasks], concurrency);
    warnings.push(...phaseWarnings);
  }
}

for (const warning of warnings) {
  console.log(`\n${yellow(`── ${warning.name} (advisory, did not fail ci) ──`)}`);
  if (warning.output) console.log(warning.output);
}
const warningSuffix = warnings.length > 0 ? `, ${warnings.length} advisory warning(s)` : '';
const skipped = noDocker ? ', docker tasks skipped' : '';
const only = dockerOnly ? ', docker tasks only' : '';
console.log(
  `\n${green('✓')} ci passed ${dim(`(${formatMs(Date.now() - totalStart)}${warningSuffix}${skipped}${only})`)}`
);
