import { err, ok } from 'neverthrow';

import { expectJsonObject, expectOnlyEra } from '../checks/body';
import { headerIncludes } from '../checks/headers';
import { isNoStore, isStatus } from '../checks/status';
import { FIXTURE_POET } from '../fixtures';
import { WEB } from '../target';

import type { Check, Probe } from '../types';

const ALL = ['origin', 'stack', 'prod'] as const;

const isPoemSlug: Check = {
  name: '4-letter poem slug',
  run: (body) =>
    /^[a-zA-Z]{4}$/.test(body.trim()) ? ok(undefined) : err(`body "${body}" is not a slug`),
};

export const proxyProbes: readonly Probe[] = [
  {
    url: `${WEB}/api/v1/search?q=%D8%AD%D8%A8`,
    note: 'proxy search passthrough, publicly cacheable',
    expect: 'ok',
    checks: [expectJsonObject, headerIncludes('Cache-Control', 'public')],
    surfaces: ALL,
  },
  {
    url: `${WEB}/api/v1/search?q=&types=poets&poemsPage=1&poetsPage=1&eraSlugs=jahili&exact=false`,
    note: 'proxy era-only search in the form the search island sends',
    expect: 'ok',
    checks: [expectJsonObject, expectOnlyEra('poets', 'jahili')],
    surfaces: ALL,
  },
  {
    url: `${WEB}/api/v1/poems/random`,
    note: 'proxy random poem passthrough, never cached',
    expect: 'ok',
    checks: [isPoemSlug, isNoStore],
    surfaces: ALL,
  },
  {
    url: `${WEB}/api/v1/poems?poet=${FIXTURE_POET.slug}&page=1`,
    note: "proxy one poet's poem list, as the poet page sends it",
    expect: 'ok',
    checks: [expectJsonObject],
    surfaces: ALL,
  },
  {
    url: `${WEB}/api/v1/poems?meter=altawil&meter=alkamil`,
    note: 'proxy poem list naming no poet is 404',
    checks: [isNoStore, isStatus(404)],
    surfaces: ALL,
  },
  {
    url: `${WEB}/api/v1/poems/slugs`,
    note: 'blocked proxy path is 404',
    checks: [isNoStore, isStatus(404)],
    surfaces: ALL,
  },
  {
    url: `${WEB}/api/v1/Search?q=x`,
    note: 'uppercased proxy path is blocked',
    checks: [isStatus(404)],
    surfaces: ALL,
  },
];
