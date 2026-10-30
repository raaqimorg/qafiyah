import { SEARCH_POEMS_PER_PAGE } from '@qafiyah/config';

import { offsetCursor, PHRASE_AR, QUERY_AR, searchUrl } from '../checks';
import { OPENAPI_SPEC, SEARCH, WEB } from '../target';

import type { Probe } from '../types';

export const httpShapeProbes: readonly Probe[] = [
  {
    url: searchUrl({ q: QUERY_AR }),
    method: 'POST',
    expect: 'healthy',
    note: 'POST to a GET search route',
  },
  { url: SEARCH, method: 'OPTIONS', expect: 'healthy', note: 'OPTIONS (CORS preflight)' },
  { url: SEARCH, method: 'DELETE', expect: 'healthy', note: 'DELETE on read-only endpoint' },
  { url: SEARCH, method: 'PUT', expect: 'healthy', note: 'PUT on read-only endpoint' },
  { url: SEARCH, method: 'PATCH', expect: 'healthy', note: 'PATCH on read-only endpoint' },
  { url: `${WEB}/`, method: 'HEAD', expect: 'healthy', note: 'HEAD on homepage' },
  { url: `${WEB}/`, method: 'OPTIONS', expect: 'healthy', note: 'OPTIONS on web root' },
  { url: `${SEARCH}/`, expect: 'healthy', note: 'trailing slash on endpoint' },
  {
    url: `${WEB}/`,
    headers: { Range: 'bytes=0-127' },
    expect: 'healthy',
    note: 'Range request (200 or 206)',
  },
  {
    url: `${WEB}/`,
    headers: { Range: 'bytes=99999999999-' },
    expect: 'healthy',
    note: 'unsatisfiable Range (200 or 416, never 5xx)',
  },
  {
    url: `${WEB}/`,
    headers: { 'If-None-Match': '"smoke-nonexistent"' },
    expect: 'healthy',
    note: 'non-matching If-None-Match',
  },
  {
    url: searchUrl({ q: QUERY_AR }),
    headers: { 'X-Forwarded-For': '203.0.113.7, 198.51.100.9' },
    expect: 'healthy',
    note: 'spoofed X-Forwarded-For chain',
  },
  {
    url: searchUrl({ q: QUERY_AR }),
    headers: { 'X-Forwarded-Host': 'evil.example.com' },
    expect: 'healthy',
    note: 'spoofed X-Forwarded-Host',
  },
  {
    url: searchUrl({ q: QUERY_AR }),
    headers: { Accept: 'application/xml' },
    expect: 'healthy',
    note: 'unsatisfiable Accept (content negotiation)',
  },
  {
    url: searchUrl({ q: QUERY_AR }),
    headers: { 'Accept-Encoding': 'identity;q=0, *;q=0' },
    expect: 'healthy',
    note: 'Accept-Encoding rejecting every encoding',
  },
  {
    url: searchUrl({ q: QUERY_AR, extra: { padding: 'a'.repeat(20_000) } }),
    expect: 'healthy',
    originOnly: true,
    note: '~20KB unknown param (URI length / 414 handling)',
  },
];

export type Differential = {
  readonly note: string;
  readonly a: string;
  readonly b: string;
  readonly because: string;
};

export const differentials: readonly Differential[] = [
  {
    note: 'poems first page vs the next one (paging is wired)',
    a: searchUrl({ q: QUERY_AR, types: ['poems'] }),
    b: searchUrl({
      q: QUERY_AR,
      types: ['poems'],
      poemsCursor: offsetCursor(SEARCH_POEMS_PER_PAGE),
    }),
    because: 'identical bytes mean poemsCursor is parsed, validated, then dropped before ES',
  },
  {
    note: 'unfiltered vs era-filtered poems (the era filter filters)',
    a: searchUrl({ q: QUERY_AR, types: ['poems'] }),
    b: searchUrl({ q: QUERY_AR, types: ['poems'], eraSlugs: ['abbasi'] }),
    because: 'identical bytes mean eraSlugs never reached the ES query',
  },
  {
    note: 'exact=true vs exact=false on a phrase (the exact param is wired)',
    a: searchUrl({ q: PHRASE_AR, types: ['poems'], exact: 'true' }),
    b: searchUrl({ q: PHRASE_AR, types: ['poems'], exact: 'false' }),
    because: 'identical bytes mean exact is parsed then dropped before ES',
  },
];

export type Conditional = { readonly note: string; readonly url: string };

export const conditionals: readonly Conditional[] = [
  { note: 'homepage honors If-None-Match → 304', url: `${WEB}/` },
  { note: 'openapi honors If-None-Match → 304', url: OPENAPI_SPEC },
];
