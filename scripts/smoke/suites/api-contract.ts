import { readFileSync } from 'node:fs';

import { ROOT } from '../../lib/root';
import { expectJsonObject, expectProblemJson, expectSomeResults } from '../checks/body';
import { isStatus } from '../checks/status';
import { API } from '../target';

import type { Probe } from '../types';

const ALL = ['origin', 'stack', 'prod'] as const;

const LLMS_LINK = /\]\(\{BASE\}(\/[^)]*)\)/g;

const list = (path: string, note: string): Probe => ({
  url: `${API}/v1${path}`,
  note,
  expect: 'ok',
  checks: [expectJsonObject],
  surfaces: ALL,
});

const documented = (path: string, note: string): Probe =>
  path.startsWith('/go/')
    ? { url: `${API}/v1${path}`, note, redirect: 'manual', checks: [isStatus(302)], surfaces: ALL }
    : { url: `${API}/v1${path}`, note, expect: 'ok', checks: [expectSomeResults], surfaces: ALL };

const llmsLinks = [
  ...new Set(
    [...readFileSync(`${ROOT}/well-known/llms.api.md`, 'utf8').matchAll(LLMS_LINK)].flatMap(
      (link) => (link[1] === undefined ? [] : [link[1]])
    )
  ),
];

export const apiContractProbes: readonly Probe[] = [
  list('/poems', 'poems list'),
  list('/poems/count', 'poems count'),
  list('/poets', 'poets list'),
  list('/poets/slugs', 'poets slugs'),
  list('/eras', 'eras list'),
  list('/meters', 'meters list, the developers page example'),
  list('/rhymes', 'rhymes list'),
  list('/themes', 'themes list'),
  list('/collections', 'collections list'),
  list('/poem-types', 'verse forms list'),
  {
    url: `${API}/v1/poems/not-a-real-slug`,
    note: 'malformed poem slug is 400',
    checks: [expectProblemJson('BAD_REQUEST')],
    surfaces: ALL,
  },
  {
    url: `${API}/v1/poems?page=0`,
    note: 'page zero is 400',
    checks: [expectProblemJson('BAD_REQUEST')],
    surfaces: ALL,
  },
  {
    url: `${API}/v1/search?q=%D8%AD%D8%A8&poemsPage=999999`,
    note: 'search page past window is 400',
    checks: [expectProblemJson('BAD_REQUEST')],
    surfaces: ALL,
  },
  documented('/poems/random?option=lines', 'README example: a random verse'),
  ...llmsLinks.map((path) => documented(path, `llms.txt link ${path}`)),
];
