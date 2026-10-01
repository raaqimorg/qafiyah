#!/usr/bin/env bun

import { DEV_API_PORT, DEV_INSPECTOR_PORT, DEV_POSTGRES_PORT, DEV_WEB_PORT } from '@qafiyah/config';

import { ensureEnvFileFrom } from './env-file';
import { detectOrbStack } from './orbstack';
import {
  cargoProgress,
  composeProgress,
  elapsed,
  fit,
  imageBuildProgress,
  indexerProgress,
  readLines,
} from './progress';
import { serviceUrls } from './service-urls';
import { primaryCheckoutRoot, resolveWorktreeIdentity, type WorktreeIdentity } from './worktree';

import type { Subprocess } from 'bun';

const ROOT = `${import.meta.dir}/../..`;
const VERBOSE = process.argv.includes('--verbose');
const WORKTREE_FLAG = process.argv.includes('--worktree');
const WITH_INSPECTOR = process.argv.includes('--inspector');

const supportsColor = process.stdout.isTTY && process.env['NO_COLOR'] === undefined;
const ESC = String.fromCodePoint(27);
const paint = (code: number) => (text: string) =>
  supportsColor ? `${ESC}[${code}m${text}${ESC}[0m` : text;
const dim = paint(2);
const bold = paint(1);
const green = paint(32);
const red = paint(31);
const yellow = paint(33);
const cyan = paint(36);
const magenta = paint(35);

const formatMs = (ms: number) => `${(ms / 1000).toFixed(1)}s`;

const SPINNER = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const SPINNER_MS = 100;
const DB_LOG_POLL_MS = 2000;

type Stage = {
  readonly detail: (text: string) => void;
  readonly end: () => void;
  readonly clear: () => void;
};

let redrawLive: (() => void) | undefined;

function say(text: string): void {
  if (redrawLive) process.stdout.write(`\r${ESC}[2K`);
  console.log(text);
  redrawLive?.();
}

function startStage(label: string, announce = true): Stage {
  const prefix = `${bold('▶')} ${label}... `;
  if (!process.stdout.isTTY) {
    if (announce) process.stdout.write(prefix);
    return { detail: () => {}, end: () => {}, clear: () => {} };
  }
  const start = Date.now();
  let frame = 0;
  let detail = '';
  const draw = () => {
    const spinner = cyan(SPINNER[frame % SPINNER.length] ?? '');
    frame += 1;
    const time = elapsed(Date.now() - start);
    const room = (process.stdout.columns || 80) - label.length - time.length - 7;
    const extra = detail ? `  ${fit(detail, room)}` : '';
    process.stdout.write(`\r${ESC}[2K${spinner} ${label}${extra}  ${dim(time)}`);
  };
  draw();
  redrawLive = draw;
  const timer = setInterval(draw, SPINNER_MS);
  const stop = () => {
    clearInterval(timer);
    redrawLive = undefined;
  };
  return {
    detail: (text) => {
      detail = text;
    },
    end: () => {
      stop();
      process.stdout.write(`\r${ESC}[2K${prefix}`);
    },
    clear: () => {
      stop();
      process.stdout.write(`\r${ESC}[2K`);
    },
  };
}

async function ensureDockerRunning(): Promise<void> {
  const stage = startStage('docker');
  const running = await Bun.spawn(['docker', 'info'], {
    cwd: ROOT,
    env: process.env,
    stdout: 'ignore',
    stderr: 'ignore',
  })
    .exited.then((code) => code === 0)
    .catch(() => false);
  stage.end();
  if (running) {
    console.log(green('✓'));
    return;
  }
  console.log(red('✗'));
  console.error(red('docker is not running, start Docker (OrbStack/Docker Desktop) and retry'));
  process.exit(1);
}

type StageOptions = {
  readonly progress?: (line: string) => string | undefined;
  readonly watch?: (onLine: (line: string) => void) => () => void;
};

async function runStage(label: string, cmd: string[], options: StageOptions = {}): Promise<void> {
  const stage = startStage(label);
  const start = Date.now();
  const proc = Bun.spawn(cmd, { cwd: ROOT, env: process.env, stdout: 'pipe', stderr: 'pipe' });
  const output: string[] = [];
  const onLine = (line: string) => {
    const detail = options.progress?.(line);
    if (detail !== undefined) stage.detail(detail);
  };
  const onOutput = (line: string) => {
    output.push(line);
    onLine(line);
  };
  const stopWatching = process.stdout.isTTY ? options.watch?.(onLine) : undefined;
  const [, , code] = await Promise.all([
    readLines(proc.stdout, onOutput),
    readLines(proc.stderr, onOutput),
    proc.exited,
  ]);
  stopWatching?.();
  stage.end();
  const ms = Date.now() - start;
  if (code !== 0) {
    console.log(`${red('✗')} ${dim(`(${formatMs(ms)})`)}`);
    const text = output.join('\n').trimEnd();
    if (text) console.error(text);
    process.exit(code || 1);
  }
  console.log(`${green('✓')} ${dim(`(${formatMs(ms)})`)}`);
}

async function dumpStage(): Promise<void> {
  const stage = startStage('dump');
  const start = Date.now();
  const proc = Bun.spawn(['./scripts/db/resolve-dump.sh'], {
    cwd: ROOT,
    env: process.env,
    stdout: 'ignore',
    stderr: 'pipe',
  });
  const stderr = await new Response(proc.stderr).text();
  const code = await proc.exited;
  stage.end();
  const ms = Date.now() - start;
  if (code !== 0) {
    console.log(`${red('✗')} ${dim(`(${formatMs(ms)})`)}`);
    if (stderr.trim()) console.error(stderr.trim());
    process.exit(code || 1);
  }
  const usingMatch = stderr.match(/\[resolve-dump] using (\S+) \(decrypted with (\S+)\)/);
  const fallbackMatch = stderr.match(
    /\[resolve-dump] (\S+) did not decrypt (\S+), falling back to data\/db\/0000_default/
  );
  const behindMatch = stderr.match(
    /\[resolve-dump] (\d+) dump\(s\) behind latest \((\S+)\), run '([^']+)' to catch up/
  );
  if (usingMatch) {
    console.log(`${green('✓')} using ${cyan(usingMatch[1] ?? '')} ${dim(`(${formatMs(ms)})`)}`);
  } else if (fallbackMatch) {
    console.log(
      `${yellow('⚠')} ${fallbackMatch[1]} didn't work, using default sample ${dim(`(${formatMs(ms)})`)}`
    );
  } else {
    console.log(`${green('✓')} using default sample ${dim(`(${formatMs(ms)})`)}`);
  }
  if (behindMatch) {
    const [, count, latest, cmd] = behindMatch;
    const plural = count === '1' ? '' : 's';
    console.log(
      yellow(
        `  ⓘ ${count} dump${plural} behind latest (${cyan(latest ?? '')}), run '${cmd}' to catch up`
      )
    );
  }
}

async function preflightStage(target: string): Promise<void> {
  const stage = startStage('preflight');
  const start = Date.now();
  const proc = Bun.spawn(['bun', 'scripts/dev/preflight.ts', target], {
    cwd: ROOT,
    env: process.env,
    stdout: 'pipe',
    stderr: 'pipe',
  });
  const [stdout, stderr, code] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ]);
  stage.end();
  const ms = Date.now() - start;
  const output = [stdout, stderr].filter(Boolean).join('\n').trim();
  if (code !== 0) {
    console.log(`${red('✗')} ${dim(`(${formatMs(ms)})`)}`);
    if (output) console.error(output);
    process.exit(code || 1);
  }
  if (output) {
    console.log(`${yellow('⚠')} ${dim(`(${formatMs(ms)})`)}`);
    console.warn(yellow(output));
    return;
  }
  console.log(`${green('✓')} ${dim(`(${formatMs(ms)})`)}`);
}

async function ensureWebEnvFile(effectiveApiPort: number): Promise<void> {
  const envFile = `${ROOT}/apps/web/.env`;
  if (await Bun.file(envFile).exists()) return;
  await Bun.write(envFile, `PUBLIC_API_URL=http://localhost:${effectiveApiPort}\n`);
}

async function ensureRootEnvFile(identity: WorktreeIdentity): Promise<void> {
  if (!identity.isWorktree) return;
  const primaryRoot = await primaryCheckoutRoot();
  await ensureEnvFileFrom(`${ROOT}/.env`, `${primaryRoot}/.env`);
}

async function printReclaimable(): Promise<void> {
  const stage = startStage('docker disk usage', false);
  const proc = Bun.spawn(['bash', '-c', 'source scripts/lib/reclaimable.sh && print_reclaimable'], {
    cwd: ROOT,
    env: process.env,
    stdout: 'pipe',
    stderr: 'ignore',
  });
  const text = await new Response(proc.stdout).text();
  await proc.exited;
  stage.clear();
  const items = [...text.matchAll(/^\s*(Images|Containers|Build Cache):\s*(\S+)/gm)].map(
    ([, type, size]) => `${size} ${(type ?? '').toLowerCase()}`
  );
  if (items.length === 0) return;
  console.log(dim(`  ${items.join(', ')} reclaimable (docker system prune to clean up)`));
}

async function tagDbContainer(psArgs: string[]): Promise<void> {
  const stage = startStage('db container tag', false);
  const psProc = Bun.spawn(['./scripts/dev/compose.sh', 'ps', '-q', 'db', ...psArgs], {
    cwd: ROOT,
    env: process.env,
    stdout: 'pipe',
    stderr: 'ignore',
  });
  const containerId = (await new Response(psProc.stdout).text()).trim();
  await psProc.exited;
  if (!containerId) {
    stage.clear();
    return;
  }

  const tagProc = Bun.spawn(
    [
      'bash',
      '-c',
      'source scripts/lib/tag-db-container.sh && tag_db_container "$1"',
      '--',
      containerId,
    ],
    { cwd: ROOT, env: process.env, stdout: 'pipe', stderr: 'pipe' }
  );
  const [stdout, stderr] = await Promise.all([
    new Response(tagProc.stdout).text(),
    new Response(tagProc.stderr).text(),
  ]);
  await tagProc.exited;
  stage.clear();
  const output = [stdout, stderr].filter(Boolean).join('\n').trim();
  if (output) console.log(dim(`  ${output}`));
}

type JsonRecord = Record<string, unknown>;

function tryParseJson(raw: string): JsonRecord | null {
  try {
    const value: unknown = JSON.parse(raw);
    return value !== null && typeof value === 'object' && !Array.isArray(value)
      ? (value as JsonRecord)
      : null;
  } catch {
    return null;
  }
}

function readString(record: JsonRecord, key: string): string | undefined {
  const value = record[key];
  return typeof value === 'string' ? value : undefined;
}

function readNumber(record: JsonRecord, key: string): number | undefined {
  const value = record[key];
  return typeof value === 'number' ? value : undefined;
}

type Watched = { proc: Subprocess; ready: Promise<void>; streaming: Promise<void> };

const API_FAILURE_STAGES = new Set(['env', 'runtime', 'boot_db', 'boot_es', 'bind', 'serve']);
const API_PREFIX = magenta('api');

function startApi(): Watched {
  const started = Date.now();
  const proc = Bun.spawn(['./target/debug/qafiyah-api'], {
    cwd: ROOT,
    env: process.env,
    stdout: 'pipe',
    stderr: 'pipe',
  });
  let readyDone = false;
  let resolveReady!: () => void;
  const ready = new Promise<void>((resolve) => {
    resolveReady = resolve;
  });
  const onLine = (raw: string): void => {
    const parsed = tryParseJson(raw);
    if (!parsed) {
      say(`${API_PREFIX} │ ${raw}`);
      return;
    }
    const stage = readString(parsed, 'stage');
    if (stage === 'ready') {
      const port = readNumber(parsed, 'port');
      if (port !== undefined && !readyDone) {
        readyDone = true;
        say(
          `${green('✓')} api ready → ${cyan(`http://localhost:${port}`)} ${dim(`(${formatMs(Date.now() - started)})`)}`
        );
        resolveReady();
      }
      return;
    }
    if (stage === 'draining' || stage === 'stopped') return;
    if (stage !== undefined && API_FAILURE_STAGES.has(stage)) {
      say(`${red('✗')} api ${stage}: ${readString(parsed, 'error') ?? 'unknown error'}`);
      return;
    }
    const kind = readString(parsed, 'kind');
    if (kind === 'completed') {
      if (VERBOSE) say(`${API_PREFIX} │ ${raw}`);
      return;
    }
    if (kind === 'completed_error') {
      const status = readNumber(parsed, 'status_code');
      const method = readString(parsed, 'method');
      const path = readString(parsed, 'path');
      const duration = readNumber(parsed, 'duration_ms');
      say(`${API_PREFIX} │ ${red(String(status))} ${method} ${path} (${duration}ms)`);
      return;
    }
    say(`${API_PREFIX} │ ${raw}`);
  };
  const streaming = Promise.all([
    readLines(proc.stdout, onLine),
    readLines(proc.stderr, onLine),
  ]).then(() => undefined);
  return { proc, ready, streaming };
}

const WEB_TASK_PREFIX = /^@qafiyah\/web:dev:\s?/;

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
  const requestMatch = /^\d{2}:\d{2}:\d{2}\s+\[(\d{3})\]/.exec(trimmed);
  if (requestMatch) return Number(requestMatch[1]) < 400;
  return false;
}

const WEB_PREFIX = cyan('web');

function startWeb(): Watched {
  const started = Date.now();
  const proc = Bun.spawn(['turbo', 'run', 'dev', '--filter=@qafiyah/web'], {
    cwd: ROOT,
    env: process.env,
    stdout: 'pipe',
    stderr: 'pipe',
  });
  let readyDone = false;
  let resolveReady!: () => void;
  const ready = new Promise<void>((resolve) => {
    resolveReady = resolve;
  });
  const onLine = (raw: string): void => {
    if (VERBOSE) {
      say(`${WEB_PREFIX} │ ${raw}`);
      return;
    }
    if (!readyDone) {
      const localMatch = raw.match(/Local\s+(\S+)/);
      if (localMatch) {
        readyDone = true;
        say(
          `${green('✓')} web ready → ${cyan(localMatch[1] ?? '')} ${dim(`(${formatMs(Date.now() - started)})`)}`
        );
        resolveReady();
        return;
      }
    }
    if (isTurboBoilerplate(raw)) return;
    const content = raw.replace(WEB_TASK_PREFIX, '');
    if (content !== raw && (isTurboBoilerplate(content) || isAstroBoilerplate(content))) return;
    say(`${WEB_PREFIX} │ ${content}`);
  };
  const streaming = Promise.all([
    readLines(proc.stdout, onLine),
    readLines(proc.stderr, onLine),
  ]).then(() => undefined);
  return { proc, ready, streaming };
}

const INSPECTOR_PREFIX = yellow('inspector');
const INSPECTOR_TASK_PREFIX = /^@qafiyah\/inspector:dev:\s?/;

function startInspector(): Watched {
  const started = Date.now();
  const proc = Bun.spawn(['turbo', 'run', 'dev', '--filter=@qafiyah/inspector'], {
    cwd: ROOT,
    env: process.env,
    stdout: 'pipe',
    stderr: 'pipe',
  });
  let readyDone = false;
  let resolveReady!: () => void;
  const ready = new Promise<void>((resolve) => {
    resolveReady = resolve;
  });
  const onLine = (raw: string): void => {
    if (!readyDone) {
      const readyMatch = raw.match(/inspector ready on (\S+)/);
      if (readyMatch) {
        readyDone = true;
        say(
          `${green('✓')} inspector ready → ${cyan(readyMatch[1] ?? '')} ${dim(`(${formatMs(Date.now() - started)})`)}`
        );
        resolveReady();
        return;
      }
    }
    if (isTurboBoilerplate(raw)) return;
    const content = raw.replace(INSPECTOR_TASK_PREFIX, '');
    if (content !== raw && isTurboBoilerplate(content)) return;
    say(`${INSPECTOR_PREFIX} │ ${content}`);
  };
  const streaming = Promise.all([
    readLines(proc.stdout, onLine),
    readLines(proc.stderr, onLine),
  ]).then(() => undefined);
  return { proc, ready, streaming };
}

async function waitReady(watched: Watched, label: string, cleanup: Watched[] = []): Promise<void> {
  const stage = startStage(`${label} starting`, false);
  const outcome = await Promise.race([watched.ready.then(() => null), watched.proc.exited]);
  stage.clear();
  if (outcome !== null) {
    console.log(`${red('✗')} ${label} exited before starting (code ${outcome})`);
    if (cleanup.length > 0) await shutdown(cleanup);
    process.exit(outcome || 1);
  }
}

async function shutdown(children: Watched[]): Promise<void> {
  console.log(`\n${bold('▶')} stopping...`);
  for (const { proc } of children) proc.kill('SIGTERM');
  await Promise.race([
    Promise.all(children.map((child) => child.proc.exited)),
    new Promise((resolve) => {
      setTimeout(resolve, 5000);
    }),
  ]);
  for (const { proc } of children) {
    if (proc.exitCode === null) proc.kill('SIGKILL');
  }
  console.log(green('✓ stopped'));
}

process.env['LANG'] = 'en_US.UTF-8';
process.env['POSTGRES_PASSWORD'] ??= 'qafiyah';
process.env['ELASTIC_PASSWORD'] ??= 'qafiyah-dev-es';
process.env['ES_READER_PASSWORD'] ??= 'qafiyah-dev-reader';
process.env['PG_READER_PASSWORD'] ??= 'qafiyah-dev-pg-reader';
process.env['PG_ACCOUNTS_PASSWORD'] ??= 'qafiyah-dev-pg-accounts';
process.env['INTERNAL_API_KEY'] = process.env['API_KEY_INTERNAL'] ?? '';

const identity = await resolveWorktreeIdentity();
if (WORKTREE_FLAG && !identity.isWorktree) {
  console.error(
    red('--worktree passed but this is the primary checkout, nothing to isolate against')
  );
  process.exit(1);
}
if (identity.isWorktree && !WORKTREE_FLAG) {
  console.warn(
    yellow(
      `running in worktree '${identity.slug}' without --worktree, ports and containers will collide with the primary checkout`
    )
  );
}
const isolating = WORKTREE_FLAG && identity.isWorktree;
const offset = isolating && identity.isWorktree ? identity.offset : 0;
const suffix = isolating && identity.isWorktree ? `-${identity.slug}` : '';

if (isolating && identity.isWorktree) {
  console.log(`${bold('▶')} worktree: ${identity.slug} ${dim(`(ports +${offset})`)}`);
  process.env['PORT'] = String(DEV_API_PORT + offset);
  process.env['WEB_PORT'] = String(DEV_WEB_PORT + offset);
  process.env['INSPECTOR_PORT'] = String(DEV_INSPECTOR_PORT + offset);
  process.env['DEV_POSTGRES_PORT'] = String(DEV_POSTGRES_PORT + offset);
  process.env['WEB_BASE_URL'] = `http://localhost:${DEV_WEB_PORT + offset}`;
  process.env['INTERNAL_API_URL'] = `http://localhost:${DEV_API_PORT + offset}`;
}

await ensureRootEnvFile(identity);

const withWorktreeFlag = (cmd: string[]): string[] => (isolating ? [...cmd, '--worktree'] : cmd);

await ensureDockerRunning();
await dumpStage();
function watchDbLog(onLine: (line: string) => void): () => void {
  let polling = false;
  const timer = setInterval(() => {
    if (polling) return;
    polling = true;
    const proc = Bun.spawn(
      withWorktreeFlag([
        './scripts/dev/compose.sh',
        'logs',
        '--tail',
        '5',
        '--no-log-prefix',
        'db',
      ]),
      { cwd: ROOT, env: process.env, stdout: 'pipe', stderr: 'ignore' }
    );
    void readLines(proc.stdout, onLine).finally(() => {
      polling = false;
    });
  }, DB_LOG_POLL_MS);
  return () => clearInterval(timer);
}

await runStage(
  'docker (db, elasticsearch)',
  withWorktreeFlag(['./scripts/dev/compose.sh', 'up', '-d', '--wait', 'db', 'elasticsearch']),
  { progress: composeProgress(), watch: watchDbLog }
);
await tagDbContainer(withWorktreeFlag([]));
await runStage(
  'search-indexer',
  withWorktreeFlag(['./scripts/dev/compose.sh', 'run', '--rm', '--no-deps', 'search-indexer']),
  { progress: (line) => indexerProgress(line) ?? imageBuildProgress(line) }
);
await ensureWebEnvFile(DEV_API_PORT + offset);

const urls = serviceUrls({
  env: process.env,
  offset,
  orbstack: await detectOrbStack(ROOT),
  suffix,
});
process.env['ELASTICSEARCH_URL'] = urls.elasticsearch;
process.env['DATABASE_URL'] = urls.database;
process.env['DATABASE_URL_ACCOUNTS'] = urls.accounts;

await printReclaimable();
await preflightStage('api');
await runStage('api build', ['cargo', 'build', '-p', 'qafiyah-api'], {
  progress: cargoProgress(),
});

const api = startApi();
await waitReady(api, 'api');
const web = startWeb();
await waitReady(web, 'web', [api]);

const allProcs: { name: string; watched: Watched }[] = [
  { name: 'api', watched: api },
  { name: 'web', watched: web },
];

if (WITH_INSPECTOR) {
  const inspector = startInspector();
  await waitReady(inspector, 'inspector', [api, web]);
  allProcs.push({ name: 'inspector', watched: inspector });
}

let stopping = false;

const onSignal = async (): Promise<void> => {
  if (stopping) return;
  stopping = true;
  await shutdown(allProcs.map(({ watched }) => watched));
  process.exit(0);
};
process.once('SIGINT', () => {
  void onSignal();
});
process.once('SIGTERM', () => {
  void onSignal();
});

const [crashed, exitCode] = await Promise.race(
  allProcs.map(({ name, watched }) => watched.proc.exited.then((code) => [name, code] as const))
);
if (!stopping) {
  stopping = true;
  console.log(`\n${red('✗')} ${crashed} exited unexpectedly (code ${exitCode})`);
  const survivors = allProcs.filter(({ name }) => name !== crashed).map(({ watched }) => watched);
  await shutdown(survivors);
  process.exit(exitCode || 1);
}
