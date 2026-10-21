#!/usr/bin/env bun

import { DEV_API_PORT, DEV_MCP_PORT } from '@qafiyah/config';

import { checkPrerequisites } from './doctor';
import { blocksDev, formatReport } from './doctor-report';
import { dumpChoice, holdsDump, SAMPLE_DUMP } from './dump-stage';
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

import type { Subprocess } from 'bun';

const ROOT = `${import.meta.dir}/../..`;
const VERBOSE = process.argv.includes('--verbose');
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

async function prerequisitesStage(): Promise<void> {
  const stage = startStage('prerequisites');
  const start = Date.now();
  const outcomes = await checkPrerequisites(['run', 'commit']);
  stage.end();
  const time = dim(`(${formatMs(Date.now() - start)})`);
  const problems = outcomes.filter((item) => item.status === 'fail' || item.status === 'warn');
  if (problems.length === 0) {
    console.log(`${green('✓')} ${time}`);
    return;
  }
  const blocking = blocksDev(outcomes);
  console.log(`${blocking ? red('✗') : yellow('⚠')} ${time}`);
  console.log(formatReport(problems, { green, red, yellow, dim, bold }));
  if (blocking) {
    console.error(
      red('\nfix the items marked ✗, then run bun run dev again (bun run doctor shows every check)')
    );
    process.exit(1);
  }
  console.log(
    dim('bun run dev goes on, fix these when you can (bun run doctor shows every check)')
  );
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
  const choice = dumpChoice(process.argv);
  if (choice.kind === 'invalid') {
    console.error(red(choice.reason));
    process.exit(1);
  }
  if (choice.kind === 'sample') {
    console.log(`${bold('▶')} dump... ${green('✓')} the 100-poem sample (${SAMPLE_DUMP})`);
    return;
  }
  process.env['DEV_DUMP'] = choice.folder;
  const stage = startStage('dump');
  const start = Date.now();
  if (!holdsDump(`${ROOT}/data/db/${choice.folder}`)) {
    const proc = Bun.spawn(['./scripts/data/pull.sh', `db/${choice.folder}`], {
      cwd: ROOT,
      env: process.env,
      stdout: 'ignore',
      stderr: 'pipe',
    });
    const stderr = await new Response(proc.stderr).text();
    if ((await proc.exited) !== 0) {
      stage.end();
      console.log(`${red('✗')} ${dim(`(${formatMs(Date.now() - start)})`)}`);
      console.error(stderr.trim());
      process.exit(1);
    }
  }
  stage.end();
  console.log(
    `${green('✓')} ${choice.folder}, restored on a fresh database ${dim(`(${formatMs(Date.now() - start)})`)}`
  );
  console.log(
    dim(`  to switch an existing database, run bun run db:reset --dump ${choice.folder}`)
  );
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

async function ensureWebEnvFile(): Promise<void> {
  const envFile = `${ROOT}/apps/web/.env`;
  if (await Bun.file(envFile).exists()) return;
  await Bun.write(envFile, `PUBLIC_API_URL=http://localhost:${DEV_API_PORT}\n`);
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

async function tagDbContainer(): Promise<void> {
  const stage = startStage('db container tag', false);
  const psProc = Bun.spawn(['./scripts/dev/compose.sh', 'ps', '-q', 'db'], {
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

const SERVICE_FAILURE_STAGES = new Set([
  'env',
  'runtime',
  'boot_db',
  'boot_es',
  'boot_api',
  'bind',
  'bind_metrics',
  'serve',
]);

function startService(
  name: 'api' | 'mcp',
  binary: string,
  env: Record<string, string | undefined>
): Watched {
  const prefix = name === 'api' ? magenta(name) : yellow(name);
  const started = Date.now();
  const proc = Bun.spawn([binary], {
    cwd: ROOT,
    env,
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
      say(`${prefix} │ ${raw}`);
      return;
    }
    const stage = readString(parsed, 'stage');
    if (stage === 'ready') {
      const port = readNumber(parsed, 'port');
      if (port !== undefined && !readyDone) {
        readyDone = true;
        say(
          `${green('✓')} ${name} ready → ${cyan(`http://localhost:${port}`)} ${dim(`(${formatMs(Date.now() - started)})`)}`
        );
        resolveReady();
      }
      return;
    }
    if (stage === 'draining' || stage === 'stopped') return;
    if (stage !== undefined && SERVICE_FAILURE_STAGES.has(stage)) {
      say(`${red('✗')} ${name} ${stage}: ${readString(parsed, 'error') ?? 'unknown error'}`);
      return;
    }
    const kind = readString(parsed, 'kind');
    if (kind === 'completed') {
      if (VERBOSE) say(`${prefix} │ ${raw}`);
      return;
    }
    if (kind === 'completed_error') {
      const status = readNumber(parsed, 'status_code');
      const method = readString(parsed, 'method');
      const path = readString(parsed, 'path');
      const duration = readNumber(parsed, 'duration_ms');
      say(`${prefix} │ ${red(String(status))} ${method} ${path} (${duration}ms)`);
      return;
    }
    say(`${prefix} │ ${raw}`);
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
process.env['PG_MONITOR_PASSWORD'] ??= 'qafiyah-dev-pg-monitor';
process.env['ES_MONITOR_PASSWORD'] ??= 'qafiyah-dev-es-monitor';
process.env['GRAFANA_ADMIN_PASSWORD'] ??= 'qafiyah-dev-grafana';
process.env['INTERNAL_API_KEY'] = process.env['API_KEY_INTERNAL'] ?? '';

await prerequisitesStage();
await dumpStage();
function watchDbLog(onLine: (line: string) => void): () => void {
  let polling = false;
  const timer = setInterval(() => {
    if (polling) return;
    polling = true;
    const proc = Bun.spawn(
      ['./scripts/dev/compose.sh', 'logs', '--tail', '5', '--no-log-prefix', 'db'],
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
  ['./scripts/dev/compose.sh', 'up', '-d', '--wait', 'db', 'elasticsearch'],
  { progress: composeProgress(), watch: watchDbLog }
);
await tagDbContainer();
await runStage(
  'search-indexer',
  ['./scripts/dev/compose.sh', 'run', '--rm', '--no-deps', 'search-indexer'],
  { progress: (line) => indexerProgress(line) ?? imageBuildProgress(line) }
);
await ensureWebEnvFile();

const urls = serviceUrls(process.env);
process.env['ELASTICSEARCH_URL'] = urls.elasticsearch;
process.env['DATABASE_URL'] = urls.database;
process.env['DATABASE_URL_ACCOUNTS'] = urls.accounts;

await printReclaimable();
await preflightStage('api');
await preflightStage('mcp');
await runStage('api and mcp build', ['cargo', 'build', '-p', 'qafiyah-api', '-p', 'qafiyah-mcp'], {
  progress: cargoProgress(),
});

const api = startService('api', './target/debug/qafiyah-api', process.env);
await waitReady(api, 'api');
const mcp = startService('mcp', './target/debug/qafiyah-mcp', {
  ...process.env,
  INTERNAL_API_URL: `http://localhost:${DEV_API_PORT}`,
  MCP_PORT: String(DEV_MCP_PORT),
});
await waitReady(mcp, 'mcp', [api]);
const web = startWeb();
await waitReady(web, 'web', [api, mcp]);

const allProcs: { name: string; watched: Watched }[] = [
  { name: 'api', watched: api },
  { name: 'mcp', watched: mcp },
  { name: 'web', watched: web },
];

if (WITH_INSPECTOR) {
  const inspector = startInspector();
  await waitReady(inspector, 'inspector', [api, mcp, web]);
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
