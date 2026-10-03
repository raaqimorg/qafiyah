import {
  MAX_FILTER_SLUGS,
  MAX_QUERY_LENGTH,
  SEARCH_POEMS_MAX_PAGE,
  SEARCH_POETS_MAX_PAGE,
  SEARCH_TYPE_VALUES,
} from '@qafiyah/config';

import {
  ar,
  BIDI_OVERRIDE,
  BOM_PREFIXED,
  COMBINING_OVERFLOW,
  EMOJI_AT_LIMIT,
  EMOJI_OVER_UTF16,
  HARAKAT_AR,
  HARAKAT_ONLY,
  NUL_QUERY,
  PHRASE_AR,
  POET_QUERY_AR,
  PRESENTATION_FORM,
  QUERY_AR,
  rawSearch,
  rawSearchWithQuery,
  searchUrl,
  TATWEEL_AR,
  ZWNJ_AR,
} from '../checks';
import { expectJsonObject, expectOnlyEra, expectPoemHits } from '../checks/body';
import { FIXTURE_POET } from '../fixtures';
import { SEARCH } from '../target';

import type { Probe } from '../types';

export const searchOkProbes: readonly Probe[] = [
  { url: searchUrl({ q: QUERY_AR }), expect: 'ok', note: 'bare query, both sections' },
  { url: searchUrl({ q: QUERY_AR, types: ['poems'] }), expect: 'ok', note: 'poems only' },
  {
    url: searchUrl({ q: 'هذا', types: ['poems'] }),
    expect: 'ok',
    checks: [expectJsonObject, expectPoemHits],
    note: 'stopword-only query still finds poems',
  },
  { url: searchUrl({ q: POET_QUERY_AR, types: ['poets'] }), expect: 'ok', note: 'poets only' },
  {
    url: searchUrl({ q: QUERY_AR, exact: 'false' }),
    expect: 'ok',
    note: 'exact=false (expanded match)',
  },
  {
    url: searchUrl({ q: QUERY_AR, exact: 'true' }),
    expect: 'ok',
    note: 'exact=true (literal match)',
  },
  {
    url: searchUrl({ q: PHRASE_AR, exact: 'true' }),
    expect: 'ok',
    note: 'multi-word literal phrase',
  },
  { url: searchUrl({ q: QUERY_AR, eraSlugs: ['jahili'] }), expect: 'ok', note: 'era filter' },
  {
    url: searchUrl({ q: QUERY_AR, eraSlugs: ['jahili', 'abbasi', 'umawi'] }),
    expect: 'ok',
    note: 'multiple eras',
  },
  {
    url: searchUrl({ q: QUERY_AR, types: ['poems'], meterSlugs: ['albasit'] }),
    expect: 'ok',
    note: 'meter filter',
  },
  {
    url: searchUrl({ q: QUERY_AR, types: ['poems'], rhymeSlugs: ['meem'] }),
    expect: 'ok',
    note: 'rhyme filter',
  },
  {
    url: searchUrl({ q: QUERY_AR, types: ['poems'], themeSlugs: ['almadih'] }),
    expect: 'ok',
    note: 'theme filter',
  },
  {
    url: searchUrl({ q: QUERY_AR, types: ['poems'], collectionSlugs: ['almuallaqat'] }),
    expect: 'ok',
    note: 'collection filter',
  },
  {
    url: searchUrl({ q: QUERY_AR, types: ['poems'], poemTypeSlugs: ['amudi'] }),
    expect: 'ok',
    note: 'verse form filter',
  },
  {
    url: searchUrl({ types: ['poems'], eraSlugs: ['jahili'], meterSlugs: ['albasit'] }),
    expect: 'ok',
    note: 'filter-only, empty query',
  },
  {
    url: searchUrl({ q: '', types: ['poems'], eraSlugs: ['jahili'] }),
    expect: 'ok',
    checks: [expectOnlyEra('poems', 'jahili')],
    note: 'era-only browse lists poems of that era',
  },
  {
    url: searchUrl({ q: '', types: ['poets'], eraSlugs: ['jahili'] }),
    expect: 'ok',
    checks: [expectOnlyEra('poets', 'jahili')],
    note: 'era-only browse lists poets of that era',
  },
  {
    url: searchUrl({
      q: QUERY_AR,
      types: ['poems'],
      exact: 'true',
      eraSlugs: ['abbasi'],
      meterSlugs: ['albasit'],
      rhymeSlugs: ['meem'],
      themeSlugs: ['almadih'],
      poemsPage: '2',
    }),
    expect: 'ok',
    note: 'kitchen-sink: query + every filter + page 2',
  },
  {
    url: searchUrl({ q: QUERY_AR, poemsPage: '3', poetsPage: '2' }),
    expect: 'ok',
    note: 'independent section pagination',
  },
  {
    url: searchUrl({ q: QUERY_AR, poemsPage: String(SEARCH_POEMS_MAX_PAGE) }),
    expect: 'ok',
    note: 'poems page at ES window boundary',
  },
  {
    url: searchUrl({ q: QUERY_AR, poetsPage: String(SEARCH_POETS_MAX_PAGE), types: ['poets'] }),
    expect: 'ok',
    note: 'poets page at ES window boundary',
  },
  {
    url: searchUrl({ q: 'ا'.repeat(MAX_QUERY_LENGTH) }),
    expect: 'ok',
    note: 'max-length query boundary',
  },
  {
    url: searchUrl({ q: QUERY_AR, extra: { foo: 'bar', injection: "' OR 1=1 --" } }),
    expect: 'ok',
    note: 'unknown params are stripped, not fatal',
  },
  { url: SEARCH, expect: 'ok', note: 'no params at all → defaults, both sections browse' },
  {
    url: searchUrl({ q: QUERY_AR, types: ['poems'], poetSlugs: [FIXTURE_POET.slug] }),
    expect: 'ok',
    note: 'poems filtered by poet',
  },
  {
    url: searchUrl({ q: POET_QUERY_AR, types: ['poets'], meterSlugs: ['albasit'] }),
    expect: 'client-error',
    note: 'poem-only filter with poets type is rejected',
  },
  {
    url: searchUrl({ types: ['poets', 'poems'], rhymeSlugs: ['meem'] }),
    expect: 'client-error',
    note: 'poem-only filter with both types (incl. poets) is rejected',
  },
  {
    url: searchUrl({ q: POET_QUERY_AR, types: ['poets'], poemTypeSlugs: ['hurr'] }),
    expect: 'client-error',
    note: 'verse form filter with poets type is rejected',
  },
  {
    url: searchUrl({ q: QUERY_AR, types: ['poems', 'poems'] }),
    expect: 'ok',
    note: 'duplicate types tolerated',
  },
  { url: searchUrl({ q: '   ' }), expect: 'ok', note: 'whitespace-only query not trimmed away' },
  {
    url: searchUrl({
      q: QUERY_AR,
      eraSlugs: Array.from({ length: MAX_FILTER_SLUGS }, () => 'jahili'),
    }),
    expect: 'ok',
    note: 'filter slugs at MAX_FILTER_SLUGS boundary',
  },
];

export const searchSemanticEdgeProbes: readonly Probe[] = [
  {
    url: searchUrl({ q: QUERY_AR, exact: 'maybe' }),
    expect: 'client-error',
    note: 'invalid exact value rejected, not silently ignored',
  },
  {
    url: searchUrl({ q: QUERY_AR, exact: 'True' }),
    expect: 'client-error',
    note: 'uppercase exact rejected (picklist is case-sensitive)',
  },
  {
    url: searchUrl({ q: QUERY_AR, poetsPage: '2', types: ['poems'] }),
    expect: 'ok',
    note: 'valid poetsPage present but section excluded',
  },
  {
    url: searchUrl({ q: QUERY_AR, eraSlugs: ['jahili', 'jahili'] }),
    expect: 'ok',
    note: 'duplicate era slugs tolerated',
  },
];

export const searchUnicodeProbes: readonly Probe[] = [
  { url: searchUrl({ q: HARAKAT_AR }), expect: 'ok', note: 'fully-voweled query (harakat)' },
  { url: searchUrl({ q: TATWEEL_AR }), expect: 'ok', note: 'kashida/tatweel elongation' },
  { url: searchUrl({ q: ZWNJ_AR }), expect: 'ok', note: 'zero-width non-joiner inside word' },
  { url: searchUrl({ q: 'حب123abcXYZ' }), expect: 'ok', note: 'mixed Arabic/Latin/digit' },
  { url: searchUrl({ q: EMOJI_AT_LIMIT }), expect: 'ok', note: 'astral emoji exactly at cap' },
  {
    url: searchUrl({ q: HARAKAT_ONLY }),
    expect: 'healthy',
    note: 'combining marks with no base letters',
  },
  {
    url: searchUrl({ q: EMOJI_OVER_UTF16 }),
    expect: 'healthy',
    note: 'astral emoji 1 past UTF-16 cap (reveals length metric)',
  },
  {
    url: searchUrl({ q: COMBINING_OVERFLOW }),
    expect: 'healthy',
    note: 'combining-mark stack past code-point cap (1 grapheme)',
  },
  { url: searchUrl({ q: BIDI_OVERRIDE }), expect: 'healthy', note: 'RLO/PDF direction override' },
  { url: searchUrl({ q: BOM_PREFIXED }), expect: 'healthy', note: 'leading BOM / ZWNBSP' },
  { url: searchUrl({ q: PRESENTATION_FORM }), expect: 'healthy', note: 'lam-alef ligature form' },
  { url: searchUrl({ q: NUL_QUERY }), expect: 'healthy', note: 'embedded NUL byte (truncation)' },
];

export const searchEncodingProbes: readonly Probe[] = [
  { url: `${SEARCH}?q=%`, expect: 'healthy', note: 'lone percent sign' },
  { url: `${SEARCH}?q=%zz`, expect: 'healthy', note: 'invalid percent hex' },
  { url: `${SEARCH}?q=%e0%a4`, expect: 'healthy', note: 'truncated UTF-8 sequence' },
  { url: `${SEARCH}?q=%0a%09%0d`, expect: 'healthy', note: 'encoded control chars in value' },
  { url: `${SEARCH}?q=%2520`, expect: 'ok', note: 'double-encoded space → literal "%20"' },
  { url: `${SEARCH}?q=a+b`, expect: 'ok', note: 'plus-as-space in query value' },
  { url: `${SEARCH}?q=`, expect: 'ok', note: 'explicit empty q → defaults' },
  { url: `${SEARCH}?`, expect: 'ok', note: 'trailing bare question mark' },
  { url: `${SEARCH}?q`, expect: 'healthy', note: 'value-less key (no =)' },
  {
    url: `${SEARCH}?q=${ar('حب')}&q=${ar('نار')}`,
    expect: 'healthy',
    note: 'repeated scalar q (first/last/array?)',
  },
  {
    url: `${SEARCH}?${ar('سؤال')}=x`,
    expect: 'healthy',
    note: 'non-ASCII param *key* (unknown, stripped)',
  },
];

export const searchArrayDecoderProbes: readonly Probe[] = [
  {
    url: rawSearchWithQuery('eraSlugs%5B1000000%5D=jahili'),
    expect: 'healthy',
    note: 'huge sparse index (no array-blowup OOM)',
  },
  {
    url: rawSearchWithQuery('types%5B1%5D=poems&types%5B0%5D=poets'),
    expect: 'healthy',
    note: 'out-of-order indices',
  },
  {
    url: rawSearchWithQuery('types%5B0%5D=poems&types%5B0%5D=poets'),
    expect: 'healthy',
    note: 'duplicate index (collision)',
  },
  {
    url: rawSearchWithQuery('types%5B0%5D=poems&types%5B5%5D=poets'),
    expect: 'healthy',
    note: 'gap → sparse length may exceed maxLength',
  },
  {
    url: rawSearchWithQuery('eraSlugs%5Bx%5D=jahili'),
    expect: 'healthy',
    note: 'non-numeric index',
  },
  { url: rawSearchWithQuery('eraSlugs%5B-1%5D=jahili'), expect: 'healthy', note: 'negative index' },
  {
    url: rawSearchWithQuery('eraSlugs%5B%5D=jahili'),
    expect: 'healthy',
    note: 'empty PHP-style brackets',
  },
  {
    url: rawSearchWithQuery('eraSlugs%5B0%5D%5B0%5D=jahili'),
    expect: 'healthy',
    note: 'nested brackets (object-in-array)',
  },
  {
    url: rawSearch(`q%5B0%5D=${ar(QUERY_AR)}`),
    expect: 'healthy',
    note: 'scalar q sent as array',
  },
];

export const searchPageCoercionProbes: readonly Probe[] = [
  { url: searchUrl({ q: QUERY_AR, poemsPage: '007' }), expect: 'healthy', note: 'leading zeros' },
  {
    url: searchUrl({ q: QUERY_AR, poemsPage: '+5' }),
    expect: 'healthy',
    note: 'leading plus sign',
  },
  {
    url: searchUrl({ q: QUERY_AR, poemsPage: '5.0' }),
    expect: 'healthy',
    note: 'integer-valued float (vs. fractional 1.5 → reject)',
  },
  {
    url: searchUrl({ q: QUERY_AR, poemsPage: '1e2' }),
    expect: 'healthy',
    note: 'scientific notation',
  },
  { url: searchUrl({ q: QUERY_AR, poemsPage: '0x10' }), expect: 'healthy', note: 'hex literal' },
  {
    url: searchUrl({ q: QUERY_AR, poemsPage: ' 5 ' }),
    expect: 'healthy',
    note: 'whitespace-padded',
  },
  {
    url: searchUrl({ q: QUERY_AR, poemsPage: '٢' }),
    expect: 'healthy',
    note: 'Arabic-Indic digit (Number()→NaN)',
  },
  { url: searchUrl({ q: QUERY_AR, poemsPage: '-0' }), expect: 'healthy', note: 'negative zero' },
  {
    url: searchUrl({ q: QUERY_AR, poemsPage: '99999999999999999999' }),
    expect: 'client-error',
    note: 'value past Number.MAX_SAFE_INTEGER and past cap',
  },
];

export const searchAbsentSlugProbes: readonly Probe[] = [
  {
    url: searchUrl({ q: QUERY_AR, types: ['poems'], meterSlugs: ['zzzzz'] }),
    expect: 'ok',
    note: 'well-formed nonexistent slug → empty, not 4xx',
  },
  { url: searchUrl({ q: QUERY_AR, eraSlugs: ['a'] }), expect: 'ok', note: 'single-letter slug' },
  {
    url: searchUrl({ q: QUERY_AR, eraSlugs: ['a--b'] }),
    expect: 'ok',
    note: 'double hyphen allowed by regex',
  },
  {
    url: searchUrl({ q: QUERY_AR, eraSlugs: ['jahili-'] }),
    expect: 'ok',
    note: 'trailing hyphen allowed by regex',
  },
  {
    url: searchUrl({ q: QUERY_AR, themeSlugs: [''] }),
    expect: 'client-error',
    note: 'empty-string slug fails ^[a-z]',
  },
  {
    url: searchUrl({ q: QUERY_AR, eraSlugs: ['jahili', 'JAHILI'] }),
    expect: 'client-error',
    note: 'one bad slug fails the whole array',
  },
];

export const searchRejectProbes: readonly Probe[] = [
  {
    url: searchUrl({ q: QUERY_AR, poemsPage: '0' }),
    expect: 'client-error',
    note: 'page below min',
  },
  {
    url: searchUrl({ q: QUERY_AR, poemsPage: String(SEARCH_POEMS_MAX_PAGE + 1) }),
    expect: 'client-error',
    note: 'poems page past ES window (would 500 ES)',
  },
  {
    url: searchUrl({ q: QUERY_AR, poetsPage: String(SEARCH_POETS_MAX_PAGE + 1), types: ['poets'] }),
    expect: 'client-error',
    note: 'poets page past ES window (would 500 ES)',
  },
  {
    url: searchUrl({ q: QUERY_AR, poemsPage: 'abc' }),
    expect: 'client-error',
    note: 'non-numeric page',
  },
  {
    url: searchUrl({ q: QUERY_AR, poemsPage: '1.5' }),
    expect: 'client-error',
    note: 'fractional page',
  },
  {
    url: searchUrl({ q: QUERY_AR, poetsPage: '-5' }),
    expect: 'client-error',
    note: 'negative page',
  },
  {
    url: searchUrl({ q: QUERY_AR, types: ['banana'] }),
    expect: 'client-error',
    note: 'unknown type',
  },
  {
    url: searchUrl({ q: QUERY_AR, types: [...SEARCH_TYPE_VALUES, 'poems'] }),
    expect: 'client-error',
    note: 'more types than allowed',
  },
  {
    url: searchUrl({ q: QUERY_AR, eraSlugs: ['JAHILI'] }),
    expect: 'client-error',
    note: 'uppercase slug rejected',
  },
  {
    url: searchUrl({ q: QUERY_AR, meterSlugs: ['meter1'] }),
    expect: 'client-error',
    note: 'slug with digit rejected',
  },
  {
    url: searchUrl({ q: 'ا'.repeat(MAX_QUERY_LENGTH + 1) }),
    expect: 'client-error',
    note: 'query past max length',
  },
  {
    url: searchUrl({
      q: QUERY_AR,
      types: ['poets'],
      poemsPage: String(SEARCH_POEMS_MAX_PAGE + 1),
    }),
    expect: 'client-error',
    note: 'unused section page still validated',
  },
  {
    url: searchUrl({
      q: QUERY_AR,
      eraSlugs: Array.from({ length: MAX_FILTER_SLUGS + 1 }, () => 'jahili'),
    }),
    expect: 'client-error',
    note: 'filter slugs past MAX_FILTER_SLUGS',
  },
  {
    url: searchUrl({ q: QUERY_AR, eraSlugs: ['-jahili'] }),
    expect: 'client-error',
    note: 'slug with leading hyphen rejected',
  },
];
