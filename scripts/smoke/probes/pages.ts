import { ar, BIDI_OVERRIDE, POET_QUERY_AR, QUERY_AR } from '../checks';
import { FIXTURE_POET } from '../fixtures';
import { WEB } from '../target';

import type { Probe } from '../types';

export const pageProbes: readonly Probe[] = [
  { url: `${WEB}/`, expect: 'ok', note: 'homepage shell' },
  { url: `${WEB}/meters`, expect: 'ok', note: 'meters index' },
  { url: `${WEB}/meters/albasit`, expect: 'ok', note: 'meter detail' },
  { url: `${WEB}/rhymes/meem`, expect: 'ok', note: 'rhyme detail' },
  { url: `${WEB}/themes`, expect: 'ok', note: 'themes index' },
  { url: `${WEB}/themes/almutafarriqat`, expect: 'ok', note: 'theme detail' },
  { url: `${WEB}/feelings`, expect: 'ok', note: 'feelings wheel' },
  { url: `${WEB}/feelings/alhuzn`, expect: 'ok', note: 'feeling detail' },
  { url: `${WEB}/collections/almuallaqat`, expect: 'ok', note: 'collection detail' },
  { url: `${WEB}/poets/${FIXTURE_POET.slug}`, expect: 'ok', note: 'poet detail' },
];

export const poetsSsrProbes: readonly Probe[] = [
  { url: `${WEB}/poets`, expect: 'ok', note: 'all poets' },
  { url: `${WEB}/poets?page=2`, expect: 'ok', note: 'page 2' },
  { url: `${WEB}/poets?page=3`, expect: 'ok', note: 'page 3' },
  { url: `${WEB}/poets?era=jahili`, expect: 'ok', note: 'era filter' },
  { url: `${WEB}/poets?era=jahili&page=2`, expect: 'ok', note: 'era + page' },
  { url: `${WEB}/poets?q=${ar(POET_QUERY_AR)}`, expect: 'ok', note: 'name search' },
  { url: `${WEB}/poets?era=jahili&q=${ar(QUERY_AR)}`, expect: 'ok', note: 'era + query combo' },
  { url: `${WEB}/poets?q=${ar('zzz')}`, expect: 'ok', note: 'non-Arabic query sanitized to empty' },
  { url: `${WEB}/poets?q=${ar('   ')}`, expect: 'ok', note: 'whitespace-only query' },
  { url: `${WEB}/poets?page=1`, expect: 'not-found', note: 'page=1 canonicalizes to bare list' },
  { url: `${WEB}/poets?page=0`, expect: 'not-found', note: 'page below minimum' },
  {
    url: `${WEB}/poets?page=999999`,
    expect: 'not-found',
    note: 'page past API MAX_PAGE → 400→404',
  },
  {
    url: `${WEB}/poets?era=jahili&page=500`,
    expect: 'not-found',
    note: 'in-range page past filtered last',
  },
  { url: `${WEB}/poets?page=abc`, expect: 'not-found', note: 'non-numeric page' },
  { url: `${WEB}/poets?era=nope`, expect: 'not-found', note: 'unknown era slug' },
  {
    url: `${WEB}/poets?era=JAHILI`,
    expect: 'not-found',
    note: 'uppercase era rejected by SSR list',
  },
  { url: `${WEB}/poets?page=2&page=3`, expect: 'healthy', note: 'duplicated page param (SSR)' },
  { url: `${WEB}/poets?page=2.0`, expect: 'healthy', note: 'integer-valued float page (SSR)' },
  { url: `${WEB}/poets?era=jahili&era=abbasi`, expect: 'healthy', note: 'duplicated era param' },
  { url: `${WEB}/poets?page[]=2`, expect: 'healthy', note: 'array-shaped scalar page' },
  {
    url: `${WEB}/poets?q=${ar(BIDI_OVERRIDE)}`,
    expect: 'healthy',
    note: 'bidi-override in SSR name search',
  },
];

export const detailBoundaryProbes: readonly Probe[] = [
  { url: `${WEB}/poems/abc`, expect: 'not-found', note: 'poem slug wrong length (not 4)' },
  { url: `${WEB}/poems/ab1d`, expect: 'not-found', note: 'poem slug with non-letter' },
  { url: `${WEB}/meters/ALBASIT`, expect: 'not-found', note: 'uppercase taxonomy slug rejected' },
  { url: `${WEB}/themes/zzz`, expect: 'not-found', note: 'well-formed but absent taxonomy slug' },
  { url: `${WEB}/feelings/zzz`, expect: 'not-found', note: 'well-formed but absent feeling slug' },
  {
    url: `${WEB}/feelings/alhuzn?page=9999`,
    expect: 'not-found',
    note: 'feeling page past last',
  },
  {
    url: `${WEB}/collections/almuallaqat?page=2`,
    expect: 'not-found',
    note: 'term page past last',
  },
  { url: `${WEB}/poems/${ar('حبيب')}`, expect: 'not-found', note: 'non-ASCII poem slug' },
  {
    url: `${WEB}/poems/${'a'.repeat(8000)}`,
    expect: 'healthy',
    originOnly: true,
    note: 'absurdly long slug (URL/header limits, no 500)',
  },
  {
    url: `${WEB}/poets/%2e%2e/%2e%2e/%2e%2e/etc/passwd`,
    expect: 'healthy',
    note: 'encoded path traversal must not escape or 500',
  },
  { url: `${WEB}//poets`, expect: 'healthy', note: 'leading double-slash path' },
  { url: `${WEB}/POETS`, expect: 'healthy', note: 'uppercased route (case-sensitive)' },
];
