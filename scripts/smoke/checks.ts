import { err, ok, type Result } from 'neverthrow';

import { MAX_QUERY_LENGTH, POEMS_PER_PAGE, SECONDS_PER_HOUR } from '@qafiyah/config';

import { SEARCH } from './target';

import type { BodyCheck } from './types';

export const ar = (s: string) => encodeURIComponent(s);

export type SearchParams = {
  readonly q?: string;
  readonly types?: readonly string[];
  readonly exact?: string;
  readonly poemsPage?: string;
  readonly poetsPage?: string;
  readonly eraSlugs?: readonly string[];
  readonly meterSlugs?: readonly string[];
  readonly rhymeSlugs?: readonly string[];
  readonly themeSlugs?: readonly string[];
  readonly poemTypeSlugs?: readonly string[];
  readonly collectionSlugs?: readonly string[];
  readonly poetSlugs?: readonly string[];
  readonly extra?: Readonly<Record<string, string>>;
};

export function searchUrl(params: SearchParams): string {
  const parts: string[] = [];
  const scalar = (key: string, value: string | undefined) => {
    if (value !== undefined) parts.push(`${key}=${encodeURIComponent(value)}`);
  };
  const list = (key: string, values: readonly string[] | undefined) => {
    if (!values) return;
    values.forEach((value, index) => {
      parts.push(`${encodeURIComponent(`${key}[${index}]`)}=${encodeURIComponent(value)}`);
    });
  };
  scalar('q', params.q);
  list('types', params.types);
  scalar('exact', params.exact);
  scalar('poemsPage', params.poemsPage);
  scalar('poetsPage', params.poetsPage);
  list('eraSlugs', params.eraSlugs);
  list('meterSlugs', params.meterSlugs);
  list('rhymeSlugs', params.rhymeSlugs);
  list('themeSlugs', params.themeSlugs);
  list('poemTypeSlugs', params.poemTypeSlugs);
  list('collectionSlugs', params.collectionSlugs);
  list('poetSlugs', params.poetSlugs);
  for (const [key, value] of Object.entries(params.extra ?? {})) {
    parts.push(`${encodeURIComponent(key)}=${encodeURIComponent(value)}`);
  }
  return `${SEARCH}?${parts.join('&')}`;
}

export const rawSearch = (queryString: string): string => `${SEARCH}?${queryString}`;
export const rawSearchWithQuery = (queryString: string): string =>
  `${SEARCH}?q=${ar(QUERY_AR)}&${queryString}`;

export const QUERY_AR = 'حب';
export const POET_QUERY_AR = 'المتنبي';
export const PHRASE_AR = 'ما بين';

export const HARAKAT_AR = 'ح\u064Fب\u0651';
export const TATWEEL_AR = `حب${'\u0640'.repeat(8)}`;
export const ZWNJ_AR = 'م\u200Cن';
export const BIDI_OVERRIDE = '\u202Eحب\u202C';
export const BOM_PREFIXED = '\uFEFFحب';
export const PRESENTATION_FORM = '\uFEFBحب';
export const NUL_QUERY = '\u0000حب';
export const HARAKAT_ONLY = '\u064B\u064C\u064D';
export const EMOJI_AT_LIMIT = '\u{1F600}'.repeat(25);
export const EMOJI_OVER_UTF16 = '\u{1F600}'.repeat(26);
export const COMBINING_OVERFLOW = `ح${'\u0651'.repeat(MAX_QUERY_LENGTH + 1)}`;

export const expectJsonObject: BodyCheck = (body, res) => {
  const contentType = res.headers.get('content-type') ?? '';
  if (!contentType.includes('json')) {
    return err(`200 but content-type is "${contentType}", expected JSON`);
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(body);
  } catch {
    return err(`200 with JSON content-type but body did not parse (${body.length} bytes)`);
  }
  if (parsed === null || typeof parsed !== 'object') {
    return err(`200 JSON body is ${parsed === null ? 'null' : typeof parsed}, expected an object`);
  }
  if ('error' in (parsed as Record<string, unknown>)) {
    return err('200 response carries a top-level `error` field');
  }
  return ok(undefined);
};

export const expectMarkup: BodyCheck = (body, res) => {
  const contentType = res.headers.get('content-type') ?? '';
  if (!contentType.includes('html')) {
    return err(`200 but content-type is "${contentType}", expected HTML`);
  }
  if (!body.includes('<')) {
    return err(`200 HTML but body carries no markup (${body.length} bytes)`);
  }
  return ok(undefined);
};

export function parseJsonObject(body: string): Result<Record<string, unknown>, string> {
  let parsed: unknown;
  try {
    parsed = JSON.parse(body);
  } catch {
    return err('body did not parse as JSON');
  }
  if (parsed === null || typeof parsed !== 'object') return err('body is not a JSON object');
  return ok(parsed as Record<string, unknown>);
}

function numberField(obj: Record<string, unknown>, path: readonly string[]): number | undefined {
  let cursor: unknown = obj;
  for (const key of path) {
    if (cursor === null || typeof cursor !== 'object' || !(key in cursor)) return undefined;
    cursor = (cursor as Record<string, unknown>)[key];
  }
  return typeof cursor === 'number' ? cursor : undefined;
}

export const expectRealPage: BodyCheck = (body) => {
  const parsed = parseJsonObject(body);
  if (parsed.isErr()) return err(parsed.error);
  const pageSize = numberField(parsed.value, ['pagination', 'pageSize']);
  if (pageSize === undefined) return err('response has no pagination.pageSize');
  if (pageSize !== POEMS_PER_PAGE) {
    return err(`pageSize ${pageSize}, expected the full ${POEMS_PER_PAGE}`);
  }
  return ok(undefined);
};

function checkRetryAfter(res: Response): Result<void, string> {
  const cacheControl = res.headers.get('Cache-Control') ?? '';
  if (!cacheControl.includes('no-store')) {
    return err(`429 Cache-Control "${cacheControl}" is not no-store`);
  }
  const retryAfter = res.headers.get('Retry-After');
  if (retryAfter === null) return err('429 is missing Retry-After');
  const seconds = Number(retryAfter);
  if (!Number.isInteger(seconds)) return err(`429 Retry-After "${retryAfter}" is not an integer`);
  if (seconds < 1 || seconds > SECONDS_PER_HOUR) {
    return err(`429 Retry-After ${seconds}s outside [1, ${SECONDS_PER_HOUR}]`);
  }
  return ok(undefined);
}

export const expectRateLimited: BodyCheck = (body, res) => {
  if (res.status === 429) {
    const retry = checkRetryAfter(res);
    if (retry.isErr()) return retry;
    const parsed = parseJsonObject(body);
    if (parsed.isErr()) return err(parsed.error);
    const code = parsed.value['code'];
    if (code !== 'TOO_MANY_REQUESTS') return err(`429 body code is ${String(code)}`);
    return ok(undefined);
  }
  if (res.status !== 200) return err(`status ${res.status}, expected 200 or 429`);
  return ok(undefined);
};

export const expectOpenApiDocuments429: BodyCheck = (body) => {
  const parsed = parseJsonObject(body);
  if (parsed.isErr()) return err(parsed.error);
  const paths = parsed.value['paths'];
  if (paths === null || typeof paths !== 'object') return err('openapi spec has no paths object');
  const documents429 = (path: string): boolean => {
    const entry = (paths as Record<string, unknown>)[path];
    const get =
      entry && typeof entry === 'object' ? (entry as Record<string, unknown>)['get'] : undefined;
    const responses =
      get && typeof get === 'object' ? (get as Record<string, unknown>)['responses'] : undefined;
    return responses !== null && typeof responses === 'object' && '429' in responses;
  };
  if (!documents429('/poems')) return err('openapi spec does not document 429 on GET /poems');
  if (!documents429('/search')) return err('openapi spec does not document 429 on GET /search');
  return ok(undefined);
};

export const headerIncludes =
  (name: string, needle: string): BodyCheck =>
  (_body, res) => {
    const value = res.headers.get(name) ?? '';
    return value.includes(needle)
      ? ok(undefined)
      : err(`${name} "${value}" does not include "${needle}"`);
  };

export const headerPresent =
  (name: string): BodyCheck =>
  (_body, res) =>
    res.headers.get(name) === null ? err(`${name} is missing`) : ok(undefined);

export const allChecks =
  (...checks: readonly BodyCheck[]): BodyCheck =>
  (body, res) => {
    for (const check of checks) {
      const result = check(body, res);
      if (result.isErr()) return result;
    }
    return ok(undefined);
  };
