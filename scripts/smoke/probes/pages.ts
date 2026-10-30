import { ar, BIDI_OVERRIDE, cursorOf, POET_QUERY_AR, QUERY_AR } from '../checks';
import { containsText } from '../checks/body';
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

const AFTER_TOP_POET = cursorOf({ after: { poemsCount: 1_000_000, nameSort: '', id: 0 } });
const BEFORE_TOP_POET = cursorOf({ before: { poemsCount: 1_000_000, nameSort: '', id: 0 } });
const AFTER_LAST_POET = cursorOf({ after: { poemsCount: -1, nameSort: '', id: 0 } });
const AFTER_LAST_ID = cursorOf({ after: 2_147_483_647 });

export const poetsSsrProbes: readonly Probe[] = [
  {
    url: `${WEB}/poets`,
    expect: 'ok',
    checks: [containsText('/poets?cursor=')],
    note: 'all poets, linking the next page by cursor',
  },
  { url: `${WEB}/poets?cursor=${AFTER_TOP_POET}`, expect: 'ok', note: 'a cursor page' },
  { url: `${WEB}/poets?page=2`, expect: 'ok', note: 'an old page link lands on the list' },
  { url: `${WEB}/poets?era=jahili`, expect: 'ok', note: 'era filter' },
  { url: `${WEB}/poets?era=jahili&cursor=${AFTER_TOP_POET}`, expect: 'ok', note: 'era + cursor' },
  { url: `${WEB}/poets?q=${ar(POET_QUERY_AR)}`, expect: 'ok', note: 'name search' },
  { url: `${WEB}/poets?era=jahili&q=${ar(QUERY_AR)}`, expect: 'ok', note: 'era + query combo' },
  { url: `${WEB}/poets?q=${ar('zzz')}`, expect: 'ok', note: 'non-Arabic query sanitized to empty' },
  { url: `${WEB}/poets?q=${ar('   ')}`, expect: 'ok', note: 'whitespace-only query' },
  { url: `${WEB}/poets?cursor=2`, expect: 'not-found', note: 'a page number is not a cursor' },
  {
    url: `${WEB}/poets?cursor=${AFTER_LAST_ID}`,
    expect: 'not-found',
    note: 'a poems cursor on the poets list',
  },
  {
    url: `${WEB}/poets?cursor=${BEFORE_TOP_POET}`,
    expect: 'not-found',
    note: 'a cursor before the first page',
  },
  {
    url: `${WEB}/poets?era=jahili&cursor=${AFTER_LAST_POET}`,
    expect: 'not-found',
    note: 'a cursor past the filtered last page',
  },
  { url: `${WEB}/poets?era=nope`, expect: 'not-found', note: 'unknown era slug' },
  {
    url: `${WEB}/poets?era=JAHILI`,
    expect: 'not-found',
    note: 'uppercase era rejected by SSR list',
  },
  {
    url: `${WEB}/poets?cursor=${AFTER_TOP_POET}&cursor=x`,
    expect: 'healthy',
    note: 'duplicated cursor param (SSR)',
  },
  { url: `${WEB}/poets?cursor=${'A'.repeat(2048)}`, expect: 'healthy', note: 'oversized cursor' },
  { url: `${WEB}/poets?era=jahili&era=abbasi`, expect: 'healthy', note: 'duplicated era param' },
  { url: `${WEB}/poets?cursor[]=x`, expect: 'healthy', note: 'array-shaped scalar cursor' },
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
    url: `${WEB}/feelings/alhuzn?cursor=${AFTER_LAST_ID}`,
    expect: 'not-found',
    note: 'feeling cursor past last',
  },
  {
    url: `${WEB}/collections/almuallaqat?cursor=${AFTER_LAST_ID}`,
    expect: 'not-found',
    note: 'term cursor past last',
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
