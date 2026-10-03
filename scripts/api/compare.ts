#!/usr/bin/env bun

import { writeFile } from 'node:fs/promises';

import { SQL } from 'bun';

import { serviceUrls } from '../dev/service-urls';

import {
  type Captured,
  type Difference,
  KEPT_HEADERS,
  diffCaptured,
  maskAccountBody,
  percentile,
  summarize,
  type Timing,
} from './compare-core';

type Target = {
  readonly path: string;
  readonly group: string;
  readonly statusOnly?: boolean;
};

type Page = {
  readonly data: readonly Record<string, unknown>[];
  readonly pagination?: { readonly totalPages: number };
};

const TAXONOMIES = ['meters', 'rhymes', 'eras', 'themes', 'collections', 'poem-types'] as const;
const POEM_FILTERS = [
  ['meter', 'meters'],
  ['theme', 'themes'],
  ['rhyme', 'rhymes'],
  ['era', 'eras'],
  ['collection', 'collections'],
] as const;

function flag(name: string): string | undefined {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : undefined;
}

const A = flag('--a');
const B = flag('--b');
const RUNS = Number(flag('--runs') ?? '10');
const OUT = flag('--out');
const KEY = process.env['API_KEY_INTERNAL'] ?? '';

if (A === undefined || B === undefined || !Number.isInteger(RUNS) || RUNS < 1) {
  console.error('usage: api:compare --a <base url> --b <base url> [--runs 10] [--out file.json]');
  process.exit(2);
}

async function send(
  base: string,
  path: string,
  init: RequestInit = {}
): Promise<Captured & { ms: number }> {
  const started = performance.now();
  const headers = new Headers(init.headers);
  headers.set('x-api-key', KEY);
  const response = await fetch(`${base}${path}`, { ...init, redirect: 'manual', headers });
  const body = await response.text();
  const ms = performance.now() - started;
  const kept: Record<string, string> = {};
  for (const name of KEPT_HEADERS) {
    const value = response.headers.get(name);
    if (value !== null) kept[name] = value;
  }
  return { status: response.status, headers: kept, body, ms };
}

async function page(path: string): Promise<Page> {
  const response = await fetch(`${A}${path}`, { headers: { 'x-api-key': KEY } });
  return response.ok ? ((await response.json()) as Page) : { data: [] };
}

const slugsOf = (rows: readonly Record<string, unknown>[], take: number): string[] =>
  rows.slice(0, take).map((row) => String(row['slug']));

async function specialSlugs(): Promise<Target[]> {
  const sql = new SQL(serviceUrls({ env: process.env, offset: 0 }).database);
  const column = async (query: string): Promise<string[]> => {
    const rows: readonly { readonly slug: string }[] = await sql.unsafe(query);
    return rows.map((row) => row.slug);
  };
  const found: Target[] = [];
  for (const slug of await column('SELECT slug FROM public.poem_aliases ORDER BY slug LIMIT 5')) {
    found.push({ path: `/v1/poems/${slug}`, group: 'poem alias redirect' });
  }
  for (const slug of await column('SELECT slug FROM public.poet_aliases ORDER BY slug LIMIT 5')) {
    found.push({ path: `/v1/poets/${slug}`, group: 'poet alias redirect' });
  }
  for (const slug of await column(
    'SELECT slug FROM public.poems WHERE recension_of_id IS NOT NULL AND NOT is_hidden ORDER BY id LIMIT 10'
  )) {
    found.push({ path: `/v1/poems/${slug}`, group: 'poem detail, a recension' });
  }
  for (const slug of await column(
    'SELECT p.slug FROM public.poems p WHERE EXISTS (SELECT 1 FROM public.poems r WHERE r.recension_of_id = p.id) AND NOT p.is_hidden ORDER BY p.id LIMIT 10'
  )) {
    found.push({ path: `/v1/poems/${slug}`, group: 'poem detail, has recensions' });
  }
  for (const slug of await column(
    'SELECT p.slug FROM public.poems p WHERE EXISTS (SELECT 1 FROM public.poem_relations r WHERE r.poem_id = p.id) AND NOT p.is_hidden ORDER BY p.id LIMIT 10'
  )) {
    found.push({ path: `/v1/poems/${slug}`, group: 'poem detail, has related poems' });
  }
  for (const slug of await column(
    'SELECT slug FROM public.poets WHERE is_hidden ORDER BY slug LIMIT 3'
  )) {
    found.push(
      { path: `/v1/poets/${slug}`, group: 'hidden poet' },
      { path: `/v1/poems?poet=${slug}`, group: 'hidden poet' },
      { path: `/v1/poems/facets?poet=${slug}`, group: 'hidden poet' }
    );
  }
  for (const slug of await column(
    'SELECT slug FROM public.poems WHERE is_hidden ORDER BY slug LIMIT 3'
  )) {
    found.push({ path: `/v1/poems/${slug}`, group: 'hidden poem' });
  }
  for (const slug of await column(
    'SELECT p.slug FROM public.poets p JOIN public.poet_stats s ON s.id = p.id ORDER BY s.poems_count DESC LIMIT 5'
  )) {
    const meters = (await page(`/v1/poems/facets?poet=${slug}`)) as unknown as {
      readonly data?: { readonly meters?: readonly { readonly slug: string }[] };
    };
    const meter = meters.data?.meters?.[0]?.slug;
    found.push(
      { path: `/v1/poems?poet=${slug}`, group: 'poems list: largest poets' },
      { path: `/v1/poems/facets?poet=${slug}`, group: 'poem facets: largest poets' }
    );
    if (meter !== undefined) {
      found.push(
        {
          path: `/v1/poems?poet=${slug}&meter=${meter}&page=2`,
          group: 'poems list: largest poets',
        },
        {
          path: `/v1/poems/facets?poet=${slug}&meter=${meter}`,
          group: 'poem facets: largest poets',
        }
      );
    }
  }
  await sql.close();
  return found;
}

async function targets(): Promise<Target[]> {
  const out: Target[] = [];
  const add = (group: string, ...paths: string[]) => {
    for (const path of paths) out.push({ path, group });
  };
  add(
    'site',
    '/',
    '/healthz',
    '/llms.txt',
    '/robots.txt',
    '/.well-known/security.txt',
    '/favicon.ico'
  );
  add('site', '/v1/openapi.json', '/v1/docs', '/nope');
  add('go redirect', '/go/x', '/go/telegram', '/go/github', '/go/db', '/go/avatars', '/go/raaqim');
  for (const kind of TAXONOMIES) {
    const list = await page(`/v1/${kind}`);
    add('taxonomy list', `/v1/${kind}`);
    add('taxonomy detail', ...slugsOf(list.data, 100).map((slug) => `/v1/${kind}/${slug}`));
    add('taxonomy unknown', `/v1/${kind}/zzzzz`);
  }

  const poems = await page('/v1/poems');
  const lastPoems = poems.pagination?.totalPages ?? 1;
  add(
    'poems list, no filter',
    '/v1/poems',
    '/v1/poems?page=2',
    `/v1/poems?page=${Math.ceil(lastPoems / 2)}`,
    `/v1/poems?page=${lastPoems}`
  );
  add('poems count', '/v1/poems/count');
  add('poem slugs', '/v1/poems/slugs', '/v1/poems/slugs?page=2', '/v1/poems/slugs?page=8');
  add('poems list, bad input', '/v1/poems?page=0', '/v1/poems?meter=BAD', '/v1/poems?page=abc');
  for (const [param, list] of POEM_FILTERS) {
    const terms = slugsOf((await page(`/v1/${list}`)).data, 3);
    for (const term of terms) {
      const last = (await page(`/v1/poems?${param}=${term}`)).pagination?.totalPages ?? 1;
      add('poems list, one value, first page', `/v1/poems?${param}=${term}`);
      add(
        'poems list, one value, middle page',
        `/v1/poems?${param}=${term}&page=${Math.max(1, Math.ceil(last / 2))}`
      );
      add('poems list, one value, last page', `/v1/poems?${param}=${term}&page=${last}`);
    }
    const [first, second, third] = terms;
    if (first !== undefined && second !== undefined) {
      const both = `${param}=${first}&${param}=${second}`;
      const last = (await page(`/v1/poems?${both}`)).pagination?.totalPages ?? 1;
      add('poems list, two values, first page', `/v1/poems?${both}`);
      add('poems list, two values, last page', `/v1/poems?${both}&page=${last}`);
      if (third !== undefined)
        add('poems list, two values, first page', `/v1/poems?${both}&${param}=${third}`);
    }
    add(
      'poems list, matches nothing',
      `/v1/poems?${param}=zzzzz`,
      `/v1/poems?${param}=zzzzz&${param}=yyyyy`
    );
  }
  const meters = slugsOf((await page('/v1/meters')).data, 2);
  const themes = slugsOf((await page('/v1/themes')).data, 2);
  const rhymes = slugsOf((await page('/v1/rhymes')).data, 40);
  const rarest = rhymes.slice(-2);
  add(
    'poems list, several filters',
    `/v1/poems?meter=${meters[0]}&theme=${themes[0]}`,
    `/v1/poems?meter=${meters[0]}&meter=${meters[1]}&theme=${themes[0]}&theme=${themes[1]}`,
    `/v1/poems?meter=${meters[0]}&meter=${meters[1]}&rhyme=${rarest[0]}&rhyme=${rarest[1]}&theme=${themes[0]}&theme=${themes[1]}`,
    `/v1/poems?meter=${meters[0]}&rhyme=zzzzz`
  );

  const poets = await page('/v1/poets');
  const lastPoets = poets.pagination?.totalPages ?? 1;
  add('poets list', '/v1/poets', '/v1/poets?page=2', `/v1/poets?page=${lastPoets}`);
  for (const era of slugsOf((await page('/v1/eras')).data, 20))
    add('poets list', `/v1/poets?era=${era}`);
  add('poet slugs', '/v1/poets/slugs', '/v1/poets/slugs?page=2');
  add('poet unknown', '/v1/poets/zzzz');
  for (const poet of slugsOf(poets.data, 30)) {
    add('poet detail', `/v1/poets/${poet}`);
    add('poems list, one poet', `/v1/poems?poet=${poet}`);
    add('poem facets', `/v1/poems/facets?poet=${poet}`);
  }
  add(
    'poem facets, errors',
    '/v1/poems/facets',
    '/v1/poems/facets?poet=Zzzz',
    '/v1/poems/facets?poet=abc'
  );
  for (const pageNumber of [1, 2, Math.ceil(lastPoems / 2)]) {
    add(
      'poem detail',
      ...slugsOf((await page(`/v1/poems?page=${pageNumber}`)).data, 30).map(
        (slug) => `/v1/poems/${slug}`
      )
    );
  }
  add('poem unknown', '/v1/poems/zzzz');
  add(
    'search',
    '/v1/search?q=%D8%AD%D8%A8',
    '/v1/search?q=&types=poets&eraSlugs=jahili',
    '/v1/search?q=&types=poems&eraSlugs=jahili'
  );
  out.push(
    { path: '/v1/poems/random', group: 'random poem', statusOnly: true },
    { path: '/v1/poems/random?option=lines', group: 'random poem', statusOnly: true },
    ...(await specialSlugs())
  );
  const seen = new Set<string>();
  return out.filter((target) => !seen.has(target.path) && seen.add(target.path) !== undefined);
}

async function accountFlow(base: string, tag: string): Promise<Captured[]> {
  const email = `compare-${Date.now()}-${tag}@example.test`;
  const json = (method: string, path: string, body: unknown) =>
    send(base, path, {
      method,
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(body),
    });
  const user = await json('POST', '/account/users', {
    provider: 'github',
    provider_uid: email,
    email,
    display_name: 'Compare',
    avatar_url: null,
  });
  const userId = (JSON.parse(user.body) as { id: number }).id;
  const noKeys = await send(base, `/account/users/${userId}/keys`);
  const created = await json('POST', '/account/keys', { user_id: userId, label: 'compare' });
  const listed = await send(base, `/account/users/${userId}/keys`);
  const keyId = (JSON.parse(listed.body) as { keys: { id: number }[] }).keys[0]?.id ?? 0;
  const revoked = await json('POST', '/account/keys/revoke', { user_id: userId, key_id: keyId });
  const afterRevoke = await send(base, `/account/users/${userId}/keys`);
  const session = await json('POST', '/account/sessions', { user_id: userId });
  const sessionId = (JSON.parse(session.body) as { id: string }).id;
  const resolved = await send(base, `/account/sessions/${sessionId}`);
  const deleted = await send(base, `/account/sessions/${sessionId}`, { method: 'DELETE' });
  const gone = await send(base, `/account/sessions/${sessionId}`);
  const deletedAll = await json('DELETE', '/account/sessions', { user_id: userId });
  const recased = await json('POST', '/account/users', {
    provider: 'github',
    provider_uid: email,
    email: email.toUpperCase(),
    display_name: null,
    avatar_url: null,
  });
  const steps = [
    user,
    noKeys,
    created,
    listed,
    revoked,
    afterRevoke,
    session,
    resolved,
    deleted,
    gone,
    deletedAll,
    recased,
  ];
  return steps.map((step) => ({
    status: step.status,
    headers: step.headers,
    body: maskAccountBody(step.body),
  }));
}

const list = await targets();
console.log(`${list.length} endpoints, ${RUNS} timed runs each on both builds`);
const differences: Difference[] = [];
const timings: Timing[] = [];
for (const target of list) {
  const first = await send(A, target.path);
  const second = await send(B, target.path);
  differences.push(...diffCaptured(target.path, first, second, target.statusOnly === true));
  const a: number[] = [];
  const b: number[] = [];
  for (let run = 0; run < RUNS; run += 1) {
    if (run % 2 === 0) {
      a.push((await send(A, target.path)).ms);
      b.push((await send(B, target.path)).ms);
    } else {
      b.push((await send(B, target.path)).ms);
      a.push((await send(A, target.path)).ms);
    }
  }
  timings.push({ path: target.path, group: target.group, a, b });
}

const flowA = await accountFlow(A, 'a');
const flowB = await accountFlow(B, 'b');
flowA.forEach((step, index) => {
  const other = flowB[index];
  if (other !== undefined)
    differences.push(...diffCaptured(`account step ${index + 1}`, step, other, false));
});

const groups = summarize(timings);
const ratios = timings
  .map((timing) => percentile(timing.b, 50) / Math.max(percentile(timing.a, 50), 0.001))
  .sort((x, y) => x - y);
console.log(`\n${differences.length} differences`);
for (const difference of differences.slice(0, 40)) {
  console.log(
    `  ${difference.path} [${difference.field}]\n    a: ${difference.a}\n    b: ${difference.b}`
  );
}
console.log('\n| Endpoint group | URLs | A p50 ms | B p50 ms | A p95 ms | B p95 ms | B / A |');
console.log('| --- | --- | --- | --- | --- | --- | --- |');
const fixed = (value: number) => value.toFixed(1);
for (const group of groups) {
  console.log(
    `| ${group.group} | ${group.urls} | ${fixed(group.aP50)} | ${fixed(group.bP50)} | ${fixed(group.aP95)} | ${fixed(group.bP95)} | ${group.ratio.toFixed(2)} |`
  );
}
console.log(`\nmedian per-endpoint B / A: ${percentile(ratios, 50).toFixed(2)}`);
if (OUT !== undefined) {
  await writeFile(
    OUT,
    JSON.stringify({ a: A, b: B, runs: RUNS, differences, groups, timings }, null, 1)
  );
  console.log(`wrote ${OUT}`);
}
process.exit(differences.length === 0 ? 0 : 1);
