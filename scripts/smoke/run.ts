#!/usr/bin/env bun

import { match } from 'ts-pattern';

import { API_KEY_HEADER } from '@qafiyah/config';

import { expectJsonObject, expectMarkup, expectRateLimited } from './checks';
import { LATENCY_DISABLED, latencyCheck } from './checks/latency';
import {
  fetchWire,
  fetchWireResilient,
  pool,
  REQUEST_TIMEOUT_MS,
  SHUTDOWN_GRACE_MS,
  waitForServer,
  type ServerUnready,
} from './http';
import { observabilityChecks } from './observability';
import { conditionals, differentials } from './probes/http-shape';
import { searchBursts, sharedEntries } from './probes/page-cache';
import { hammers } from './probes/rate-limit';
import { SUITES } from './suites';
import { latencyTargets } from './suites/latency';
import {
  API,
  API_KEY,
  authHeaders,
  CONCURRENCY,
  hostHeader,
  SEARCH,
  STACK_API_KEY_FULL,
  STACK_API_KEY_INTERNAL,
  STACK_GRAFANA_PASSWORD,
  STACK_SESSION_STATE_SECRET,
  SURFACE,
  WEB,
} from './target';

import type { SurfaceName } from './surfaces';
import type { Check, Expectation, Probe } from './types';

const ROOT = `${import.meta.dir}/../..`;

const NO_COLOR = (process.env['NO_COLOR'] ?? '') !== '';
const USE_COLOR = process.stdout.isTTY && !NO_COLOR;
const paint =
  (code: string) =>
  (s: string): string =>
    USE_COLOR ? `\u001B[${code}m${s}\u001B[0m` : s;
const green = paint('32');
const red = paint('31');
const yellow = paint('33');
const cyan = paint('36');
const dim = paint('2');
const bold = paint('1');

function probeSurfaces(probe: Probe): readonly SurfaceName[] {
  if (probe.surfaces) return probe.surfaces;
  if (probe.originOnly) return ['origin'];
  if (probe.prodOnly) return ['prod'];
  return ['origin', 'stack', 'prod'];
}

function probeHost(probe: Probe): 'web' | 'api' {
  if (probe.host) return probe.host;
  if (probe.url.startsWith(`${API}/v1`) || probe.url.startsWith(`${API}/healthz`)) return 'api';
  return 'web';
}

function probeKeyHeaders(probe: Probe): Readonly<Record<string, string>> {
  if (probe.unkeyed) return {};
  if (probe.apiKey !== undefined) return { [API_KEY_HEADER]: probe.apiKey };
  if (SURFACE.name === 'stack') {
    if (probeHost(probe) === 'api' && API_KEY) return { [API_KEY_HEADER]: API_KEY };
    return {};
  }
  return authHeaders(probe.url) ?? {};
}

const withDefaultCheck = (probes: readonly Probe[]): readonly Probe[] =>
  probes.map((probe) => {
    if (probe.check || probe.checks || probe.expect !== 'ok') return probe;
    if (probe.url.startsWith(SEARCH)) return { ...probe, check: expectJsonObject };
    if (probe.url.startsWith(WEB)) return { ...probe, check: expectMarkup };
    return probe;
  });

function statusMatches(expect: Expectation, status: number): boolean {
  return match(expect)
    .with('ok', () => status === 200)
    .with('not-found', () => status === 404)
    .with('client-error', () => status >= 400 && status < 500)
    .with('healthy', () => status < 500)
    .exhaustive();
}

function describeExpectation(expect: Expectation): string {
  return match(expect)
    .with('ok', () => '200')
    .with('not-found', () => '404')
    .with('client-error', () => '4xx')
    .with('healthy', () => 'non-5xx')
    .exhaustive();
}

type Verdict = 'pass' | 'fail' | 'skip';
type Outcome = {
  readonly note: string;
  readonly url: string;
  readonly ms: number;
  readonly verdict: Verdict;
  readonly detail?: string;
};

const pass = (note: string, url: string, ms: number): Outcome => ({
  note,
  url,
  ms,
  verdict: 'pass',
});
const fail = (note: string, url: string, ms: number, detail: string): Outcome => ({
  note,
  url,
  ms,
  verdict: 'fail',
  detail,
});
const skip = (note: string, url: string, detail: string): Outcome => ({
  note,
  url,
  ms: 0,
  verdict: 'skip',
  detail,
});

function skipReason(probe: Probe): string | null {
  if (!probeSurfaces(probe).includes(SURFACE.name)) return `not on the ${SURFACE.name} surface`;
  return null;
}

function printOutcome(o: Outcome): void {
  const ms = dim(`${o.ms.toFixed(0)}ms`);
  if (o.verdict === 'skip') console.log(`${yellow('SKIP')}  ${o.note}  ${dim(`(${o.detail})`)}`);
  else if (o.verdict === 'pass') console.log(`${green('OK')}    ${o.note}  ${ms}`);
  else console.error(`${red('FAIL')}  ${o.note}  ${ms}  →  ${o.detail}\n      ${dim(o.url)}`);
}

function runChecks(body: string, res: Response, checks: readonly Check[]): string | null {
  for (const check of checks) {
    const result = check.run(body, res);
    if (result.isErr()) return `${check.name}: ${result.error}`;
  }
  return null;
}

async function runProbe(probe: Probe): Promise<Outcome> {
  const started = performance.now();
  const init: RequestInit = {};
  if (probe.method) init.method = probe.method;
  if (probe.redirect) init.redirect = probe.redirect;
  const headers = { ...probeKeyHeaders(probe), ...probe.headers };
  if (SURFACE.name === 'stack') headers['Host'] = hostHeader(probeHost(probe));
  if (Object.keys(headers).length > 0) init.headers = headers;

  const result = await fetchWireResilient(probe.url, REQUEST_TIMEOUT_MS, init);
  const ms = performance.now() - started;
  if (result.isErr()) {
    return fail(probe.note, probe.url, ms, `${result.error.kind}: ${result.error.message}`);
  }
  const { status, body, res } = result.value;
  if (probe.expect !== undefined && !statusMatches(probe.expect, status)) {
    return fail(
      probe.note,
      probe.url,
      ms,
      `HTTP ${status}, wanted ${describeExpectation(probe.expect)}`
    );
  }
  if (probe.check) {
    const checked = probe.check(body, res);
    if (checked.isErr()) return fail(probe.note, probe.url, ms, `body check: ${checked.error}`);
  }
  if (probe.checks) {
    const checked = runChecks(body, res, probe.checks);
    if (checked !== null) return fail(probe.note, probe.url, ms, checked);
  }
  return pass(probe.note, probe.url, ms);
}

function hostForUrl(url: string): 'web' | 'api' {
  if (url.startsWith(`${API}/v1`) || url.startsWith(`${API}/healthz`)) return 'api';
  return 'web';
}

function smokeInit(url: string): RequestInit | undefined {
  const headers: Record<string, string> = {};
  if (API_KEY && hostForUrl(url) === 'api') headers[API_KEY_HEADER] = API_KEY;
  if (SURFACE.name === 'stack') headers['Host'] = hostHeader(hostForUrl(url));
  return Object.keys(headers).length > 0 ? { headers } : undefined;
}

async function runDifferential(diff: (typeof differentials)[number]): Promise<Outcome> {
  const url = `${diff.a}  ⟂  ${diff.b}`;
  const started = performance.now();
  const [ra, rb] = await Promise.all([
    fetchWireResilient(diff.a, REQUEST_TIMEOUT_MS, smokeInit(diff.a)),
    fetchWireResilient(diff.b, REQUEST_TIMEOUT_MS, smokeInit(diff.b)),
  ]);
  const ms = performance.now() - started;
  if (ra.isErr()) return fail(diff.note, url, ms, `A ${ra.error.kind}: ${ra.error.message}`);
  if (rb.isErr()) return fail(diff.note, url, ms, `B ${rb.error.kind}: ${rb.error.message}`);
  if (ra.value.status !== 200)
    return fail(diff.note, url, ms, `A returned HTTP ${ra.value.status}`);
  if (rb.value.status !== 200)
    return fail(diff.note, url, ms, `B returned HTTP ${rb.value.status}`);
  if (ra.value.body === rb.value.body)
    return fail(diff.note, url, ms, `A and B are byte-identical: ${diff.because}`);
  return pass(diff.note, url, ms);
}

async function runConditional(c: (typeof conditionals)[number]): Promise<Outcome> {
  const started = performance.now();
  const first = await fetchWireResilient(c.url, REQUEST_TIMEOUT_MS, smokeInit(c.url));
  if (first.isErr())
    return fail(
      c.note,
      c.url,
      performance.now() - started,
      `${first.error.kind}: ${first.error.message}`
    );
  const etag = first.value.res.headers.get('etag');
  if (!etag) return skip(c.note, c.url, 'no ETag emitted');
  const base = smokeInit(c.url)?.headers as Record<string, string> | undefined;
  const headers = { ...base, 'If-None-Match': etag };
  const second = await fetchWireResilient(c.url, REQUEST_TIMEOUT_MS, { headers });
  const ms = performance.now() - started;
  if (second.isErr())
    return fail(c.note, c.url, ms, `revalidate ${second.error.kind}: ${second.error.message}`);
  if (second.value.status !== 304)
    return fail(c.note, c.url, ms, `expected 304, got ${second.value.status}`);
  return pass(c.note, c.url, ms);
}

const SERVED_FROM_CACHE = new Set(['HIT', 'STALE', 'UPDATING', 'REVALIDATED']);

async function runSharedEntry(entry: (typeof sharedEntries)[number]): Promise<Outcome> {
  const started = performance.now();
  const warm = await fetchWireResilient(entry.url, REQUEST_TIMEOUT_MS, smokeInit(entry.url));
  if (warm.isErr())
    return fail(
      entry.note,
      entry.url,
      performance.now() - started,
      `warm-up ${warm.error.message}`
    );
  if (warm.value.status !== 200)
    return fail(
      entry.note,
      entry.url,
      performance.now() - started,
      `warm-up returned ${warm.value.status}`
    );
  const variant = `${entry.url}?smoke=${crypto.randomUUID()}`;
  const second = await fetchWireResilient(variant, REQUEST_TIMEOUT_MS, smokeInit(variant));
  const ms = performance.now() - started;
  if (second.isErr())
    return fail(entry.note, variant, ms, `${second.error.kind}: ${second.error.message}`);
  if (second.value.status !== 200)
    return fail(entry.note, variant, ms, `returned ${second.value.status}`);
  const cache = second.value.res.headers.get('x-cache-status');
  if (cache === null || !SERVED_FROM_CACHE.has(cache))
    return fail(
      entry.note,
      variant,
      ms,
      `x-cache-status ${cache ?? 'missing'}, expected the cached entry`
    );
  return pass(entry.note, variant, ms);
}

async function runBurst(burst: (typeof searchBursts)[number]): Promise<Outcome> {
  const started = performance.now();
  const results = await Promise.all(
    Array.from({ length: burst.count }, () =>
      fetchWire(burst.url, REQUEST_TIMEOUT_MS, smokeInit(burst.url))
    )
  );
  const ms = performance.now() - started;
  let limited = 0;
  for (const [index, result] of results.entries()) {
    const label = `request ${index + 1}/${burst.count}`;
    if (result.isErr())
      return fail(
        burst.note,
        burst.url,
        ms,
        `${label} ${result.error.kind}: ${result.error.message}`
      );
    const { status, res } = result.value;
    if (status >= 500) return fail(burst.note, burst.url, ms, `${label} returned ${status}`);
    if (status === 429) {
      if (res.headers.get('retry-after') === null)
        return fail(burst.note, burst.url, ms, `${label} is a 429 without Retry-After`);
      limited += 1;
    }
  }
  if (limited === 0) return fail(burst.note, burst.url, ms, `none of ${burst.count} was limited`);
  return pass(burst.note, burst.url, ms);
}

async function runHammer(hammer: (typeof hammers)[number]): Promise<Outcome> {
  const started = performance.now();
  const results = await Promise.all(
    Array.from({ length: hammer.count }, () => fetchWire(hammer.url, REQUEST_TIMEOUT_MS))
  );
  const ms = performance.now() - started;
  let lowestRemaining = Number.POSITIVE_INFINITY;
  let limit: number | undefined;
  for (const [index, result] of results.entries()) {
    const label = `request ${index + 1}/${hammer.count}`;
    if (result.isErr())
      return fail(
        hammer.note,
        hammer.url,
        ms,
        `${label} ${result.error.kind}: ${result.error.message}`
      );
    const { status, body, res } = result.value;
    if (status >= 500) return fail(hammer.note, hammer.url, ms, `${label} returned ${status}`);
    const validated = expectRateLimited(body, res);
    if (validated.isErr()) return fail(hammer.note, hammer.url, ms, `${label} ${validated.error}`);
    const remaining = Number(res.headers.get('X-RateLimit-Remaining'));
    const declared = Number(res.headers.get('X-RateLimit-Limit'));
    if (!Number.isInteger(remaining) || !Number.isInteger(declared)) {
      return fail(hammer.note, hammer.url, ms, `${label} has no usable X-RateLimit headers`);
    }
    limit = declared;
    lowestRemaining = Math.min(lowestRemaining, remaining);
  }
  if (limit === undefined || lowestRemaining > limit - hammer.count) {
    return fail(
      hammer.note,
      hammer.url,
      ms,
      `quota did not decrement: lowest remaining ${lowestRemaining} of limit ${String(limit)}`
    );
  }
  return pass(hammer.note, hammer.url, ms);
}

let cleanedUp = false;
async function cleanup() {
  if (cleanedUp) return;
  cleanedUp = true;
  await Bun.spawn(['bun', 'run', 'clean'], { cwd: ROOT, stdout: 'ignore', stderr: 'ignore' })
    .exited;
}

async function shutdownDev(dev: ReturnType<typeof Bun.spawn>) {
  dev.kill();
  const outcome = await Promise.race([
    dev.exited,
    Bun.sleep(SHUTDOWN_GRACE_MS).then(() => 'timeout' as const),
  ]);
  if (outcome === 'timeout') dev.kill('SIGKILL');
  await dev.exited;
}

function describeUnready(name: string, error: ServerUnready): string {
  const hint = SURFACE.manageServer ? ' (see /tmp/dev-server.log)' : '';
  const tail = error.lastError
    ? ` last error: ${error.lastError.kind}, ${error.lastError.message}.`
    : '';
  return `${name} never came up${hint}.${tail}`;
}

const RUNNING_CHECK_MS = 10_000;
const DEV_CONTAINERS = ['db', 'elasticsearch'];

async function answers(url: string): Promise<boolean> {
  const response = await fetchWire(url, RUNNING_CHECK_MS);
  return response.isOk() && response.value.status < 500;
}

async function runningServices(): Promise<readonly string[]> {
  const ps = Bun.spawn(['./scripts/dev/compose.sh', 'ps', '--status', 'running', '--services'], {
    cwd: ROOT,
    stdout: 'pipe',
    stderr: 'ignore',
  });
  const out = await new Response(ps.stdout).text();
  await ps.exited;
  return out.split('\n').filter(Boolean);
}

type StartedSurface = { readonly stop: () => Promise<void>; readonly ownsProcesses: boolean };

async function startSurface(): Promise<StartedSurface> {
  if (SURFACE.name === 'origin') {
    if ((await answers(`${WEB}/`)) && (await answers(`${API}/`))) {
      console.log(dim('reusing the dev server that is already running, and leaving it running'));
      return { stop: async () => {}, ownsProcesses: false };
    }
    const log = Bun.file('/tmp/dev-server.log');
    const devCommand = process.argv.includes('--worktree')
      ? ['bun', 'run', 'dev', '--worktree']
      : ['bun', 'run', 'dev'];
    const dev = Bun.spawn(devCommand, { cwd: ROOT, stdout: log, stderr: log });
    return { stop: () => shutdownDev(dev), ownsProcesses: true };
  }
  if (SURFACE.name === 'stack') {
    const env = {
      ...process.env,
      ENVIRONMENT: 'production',
      ANON_REQUESTS: '40',
      API_KEY_INTERNAL: process.env['API_KEY_INTERNAL'] ?? STACK_API_KEY_INTERNAL,
      API_KEY_FULL: process.env['API_KEY_FULL'] ?? STACK_API_KEY_FULL,
      SESSION_STATE_SECRET: process.env['SESSION_STATE_SECRET'] ?? STACK_SESSION_STATE_SECRET,
      GRAFANA_ADMIN_PASSWORD: process.env['GRAFANA_ADMIN_PASSWORD'] ?? STACK_GRAFANA_PASSWORD,
    };
    const running = await runningServices();
    const restore = DEV_CONTAINERS.filter((service) => running.includes(service));
    const up = Bun.spawn(['./scripts/dev/compose.sh', 'up', '-d', '--build', '--wait'], {
      cwd: ROOT,
      stdout: 'inherit',
      stderr: 'inherit',
      env,
    });
    if ((await up.exited) !== 0) {
      console.error(red('stack did not come up'));
      process.exit(1);
    }
    return {
      stop: async () => {
        const down = Bun.spawn(['./scripts/dev/compose.sh', 'down'], {
          cwd: ROOT,
          stdout: 'ignore',
          stderr: 'ignore',
          env,
        });
        await down.exited;
        if (restore.length === 0) return;
        const back = Bun.spawn(['./scripts/dev/compose.sh', 'up', '-d', '--wait', ...restore], {
          cwd: ROOT,
          stdout: 'ignore',
          stderr: 'ignore',
        });
        await back.exited;
      },
      ownsProcesses: false,
    };
  }
  return { stop: async () => {}, ownsProcesses: false };
}

async function main() {
  const suiteArg = process.argv.indexOf('--suite');
  const suiteFilter = suiteArg >= 0 ? process.argv[suiteArg + 1] : undefined;
  console.log(`Smoke surface: ${bold(SURFACE.name)} (web ${WEB}, api ${API})`);

  let stop: (() => Promise<void>) | null = null;
  let ownsProcesses = false;
  if (SURFACE.manageServer) {
    const started = await startSurface();
    stop = started.stop;
    ownsProcesses = started.ownsProcesses;
  }

  const runStarted = performance.now();
  let toreDown = false;
  const teardown = async () => {
    if (toreDown) return;
    toreDown = true;
    if (stop) await stop();
    if (ownsProcesses) await cleanup();
  };

  const bail = async (message: string) => {
    console.error(red(message));
    await teardown();
    process.exit(1);
  };

  process.once('SIGINT', () => {
    void teardown().then(() => process.exit(130));
  });
  process.once('SIGTERM', () => {
    void teardown().then(() => process.exit(130));
  });

  if (SURFACE.name === 'origin') {
    process.stdout.write('Started smoking...');
    const [webReady, apiReady] = await Promise.all([
      waitForServer(`${WEB}/`),
      waitForServer(`${API}/`),
    ]);
    if (webReady.isErr()) return await bail(` ${describeUnready('web', webReady.error)}`);
    if (apiReady.isErr()) return await bail(` ${describeUnready('api', apiReady.error)}`);
    process.stdout.write(' running...\n');
  }

  const outcomes: Outcome[] = [];
  const record = (o: Outcome) => {
    outcomes.push(o);
    printOutcome(o);
  };

  for (const suite of SUITES) {
    if (suiteFilter !== undefined && suite.name !== suiteFilter) continue;
    const visible = suite.probes.filter((probe) => probeSurfaces(probe).includes(SURFACE.name));
    if (visible.length === 0) {
      console.log(
        `\n${cyan('[')} ${bold(suite.name)} ${cyan(']')}  ${dim(`(no probes on ${SURFACE.name})`)}`
      );
      continue;
    }
    console.log(`\n${cyan('[')} ${bold(suite.name)} ${cyan(']')}`);
    const width = suite.serial ? 1 : CONCURRENCY;
    const groupOutcomes = await pool(withDefaultCheck(visible), width, async (probe) => {
      const reason = skipReason(probe);
      return reason ? skip(probe.note, probe.url, reason) : await runProbe(probe);
    });
    for (const o of groupOutcomes) record(o);
  }

  console.log(
    `\n${cyan('[')} ${bold('differential checks (the param actually takes effect)')} ${cyan(']')}`
  );
  const diffOutcomes = await pool(
    differentials,
    Math.min(CONCURRENCY, differentials.length),
    runDifferential
  );
  for (const o of diffOutcomes) record(o);

  console.log(`\n${cyan('[')} ${bold('conditional GET (ETag → 304)')} ${cyan(']')}`);
  const condOutcomes = await pool(
    conditionals,
    Math.min(CONCURRENCY, conditionals.length),
    runConditional
  );
  for (const o of condOutcomes) record(o);

  if (SURFACE.name === 'stack') {
    console.log(
      `\n${cyan('[')} ${bold('page cache (a stray query string shares the entry)')} ${cyan(']')}`
    );
    for (const entry of sharedEntries) record(await runSharedEntry(entry));
    console.log(`\n${cyan('[')} ${bold('poet search limit (burst)')} ${cyan(']')}`);
    for (const burst of searchBursts) record(await runBurst(burst));
  } else {
    for (const entry of sharedEntries)
      record(skip(entry.note, entry.url, 'stack-only; it reads the origin nginx cache status'));
    for (const burst of searchBursts)
      record(skip(burst.note, burst.url, 'stack-only; never burst a live site'));
  }

  if (SURFACE.name === 'prod') {
    console.log(`\n${cyan('[')} ${bold('unkeyed anti-scraping (hammer)')} ${cyan(']')}`);
    for (const hammer of hammers) record(await runHammer(hammer));
  } else {
    for (const hammer of hammers)
      record(skip(hammer.note, hammer.url, 'prod-only; enforcement is off elsewhere'));
  }

  console.log(
    `\n${cyan('[')} ${bold(`latency budgets${LATENCY_DISABLED ? ' (disabled)' : ''}`)} ${cyan(']')}`
  );
  for (const target of latencyTargets) {
    const budget = target.budgets[SURFACE.name];
    await fetchWireResilient(target.url, REQUEST_TIMEOUT_MS, smokeInit(target.url));
    const samples: number[] = [];
    for (let i = 0; i < 5; i += 1) {
      const started = performance.now();
      const result = await fetchWireResilient(
        target.url,
        REQUEST_TIMEOUT_MS,
        smokeInit(target.url)
      );
      const ms = performance.now() - started;
      if (result.isErr()) {
        record(fail(target.id, target.url, ms, `${result.error.kind}: ${result.error.message}`));
        samples.push(Number.POSITIVE_INFINITY);
      } else {
        samples.push(ms);
      }
    }
    const sorted = [...samples].sort((a, b) => a - b);
    const p95 = sorted[4] ?? Number.POSITIVE_INFINITY;
    const check = latencyCheck(p95, budget);
    if (check.isErr()) record(fail(target.id, target.url, p95, check.error));
    else record(pass(target.id, target.url, p95));
  }

  if (SURFACE.grafana !== null) {
    console.log(`\n${cyan('[')} ${bold('observability')} ${cyan(']')}`);
    const password = process.env['GRAFANA_ADMIN_PASSWORD'] ?? STACK_GRAFANA_PASSWORD;
    for (const result of await observabilityChecks(SURFACE.grafana, password)) {
      record(
        result.failure === null
          ? pass(result.note, result.url, result.ms)
          : fail(result.note, result.url, result.ms, result.failure)
      );
    }
  }

  const totalMs = performance.now() - runStarted;

  let conformanceCode = 0;
  if (SURFACE.name === 'stack') {
    console.log(
      `\n${cyan('\u2014')} ${bold('openapi conformance')} ${cyan('\u2014')} ${dim('(skipped: covered by api-contract through the edge)')}`
    );
  } else {
    console.log(`\n${cyan('\u2014')} ${bold('openapi conformance')} ${cyan('\u2014')}`);
    const conformance = Bun.spawn(
      [
        'bun',
        'scripts/api/conformance.ts',
        SURFACE.name === 'prod' ? 'prod' : 'dev',
        ...(process.argv.includes('--worktree') ? ['--worktree'] : []),
      ],
      { cwd: ROOT, stdout: 'inherit', stderr: 'inherit' }
    );
    conformanceCode = await conformance.exited;
  }

  await teardown();

  if (conformanceCode !== 0) {
    console.error(red('\nopenapi conformance failed'));
    process.exit(1);
  }

  const passed = outcomes.filter((o) => o.verdict === 'pass');
  const failed = outcomes.filter((o) => o.verdict === 'fail');
  const skipped = outcomes.filter((o) => o.verdict === 'skip');
  const ran = passed.length + failed.length;

  const slowest = outcomes
    .filter((o) => o.verdict !== 'skip')
    .sort((a, b) => b.ms - a.ms)
    .slice(0, 10);
  if (slowest.length > 0) {
    console.log(`\n${bold('slowest:')}`);
    for (const o of slowest) console.log(`  ${dim(`${o.ms.toFixed(0).padStart(5)}ms`)}  ${o.note}`);
  }

  const secs = (totalMs / 1000).toFixed(1);
  const skipNote = skipped.length > 0 ? ` (${skipped.length} skipped)` : '';

  if (failed.length > 0) {
    console.error(`\n${red(bold(`${failed.length}/${ran} failed`))}${dim(skipNote)} in ${secs}s:`);
    for (const f of failed) console.error(`  ${red('•')} ${f.note}: ${f.detail}`);
    process.exit(1);
  }
  console.log(`\n${green(bold(`${ran}/${ran} passed`))}${dim(skipNote)} in ${secs}s`);
  process.exit(0);
}

await main();
