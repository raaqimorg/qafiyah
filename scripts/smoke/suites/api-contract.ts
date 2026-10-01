import { expectJsonObject, expectProblemJson } from '../checks/body';
import { API } from '../target';

import type { Probe } from '../types';

const ALL = ['origin', 'stack', 'prod'] as const;

const list = (path: string, note: string): Probe => ({
  url: `${API}/v1${path}`,
  note,
  expect: 'ok',
  checks: [expectJsonObject],
  surfaces: ALL,
});

export const apiContractProbes: readonly Probe[] = [
  list('/poems', 'poems list'),
  list('/poems/count', 'poems count'),
  list('/poets', 'poets list'),
  list('/poets/slugs', 'poets slugs'),
  list('/eras', 'eras list'),
  list('/meters', 'meters list'),
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
];
