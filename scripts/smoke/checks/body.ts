import { err, ok, type Result } from 'neverthrow';

import type { Check } from '../types';

function parseJsonObject(body: string): Result<Record<string, unknown>, string> {
  let parsed: unknown;
  try {
    parsed = JSON.parse(body);
  } catch {
    return err('body did not parse as JSON');
  }
  if (parsed === null || typeof parsed !== 'object') return err('body is not a JSON object');
  return ok(parsed as Record<string, unknown>);
}

export const expectJsonObject: Check = {
  name: 'JSON object',
  run: (body, res) => {
    const contentType = res.headers.get('content-type') ?? '';
    if (!contentType.includes('json'))
      return err(`content-type is "${contentType}", expected JSON`);
    const parsed = parseJsonObject(body);
    if (parsed.isErr()) return err(parsed.error);
    if ('error' in parsed.value) return err('response carries a top-level `error` field');
    return ok(undefined);
  },
};

export const expectPoemHits: Check = {
  name: 'at least one poem hit',
  run: (body) => {
    const parsed = parseJsonObject(body);
    if (parsed.isErr()) return err(parsed.error);
    const poems = parsed.value['poems'];
    const hits = typeof poems === 'object' && poems !== null && 'data' in poems ? poems.data : null;
    if (!Array.isArray(hits)) return err('poems.data is not an array');
    return hits.length > 0 ? ok(undefined) : err('no poems came back');
  },
};

function eraSlugOf(item: unknown): unknown {
  if (typeof item !== 'object' || item === null || !('era' in item)) return undefined;
  const era = item.era;
  return typeof era === 'object' && era !== null && 'slug' in era ? era.slug : undefined;
}

export const expectOnlyEra = (section: 'poems' | 'poets', era: string): Check => ({
  name: `${section} all from era ${era}`,
  run: (body) => {
    const parsed = parseJsonObject(body);
    if (parsed.isErr()) return err(parsed.error);
    const found = parsed.value[section];
    const items =
      typeof found === 'object' && found !== null && 'data' in found ? found.data : null;
    if (!Array.isArray(items)) return err(`${section}.data is not an array`);
    if (items.length === 0) return err(`no ${section} came back`);
    const strays = items.filter((item: unknown) => eraSlugOf(item) !== era).length;
    return strays === 0
      ? ok(undefined)
      : err(`${strays} of ${items.length} ${section} are not from era ${era}`);
  },
});

export const notContainsText = (needle: string): Check => ({
  name: `does not contain ${JSON.stringify(needle)}`,
  run: (body) =>
    body.includes(needle) ? err(`body contains ${JSON.stringify(needle)}`) : ok(undefined),
});

export const expectProblemJson = (code: string): Check => ({
  name: `problem+json ${code}`,
  run: (body, res) => {
    const contentType = res.headers.get('content-type') ?? '';
    if (!contentType.includes('json'))
      return err(`content-type "${contentType}" is not problem+json`);
    const parsed = parseJsonObject(body);
    if (parsed.isErr()) return err(parsed.error);
    if (parsed.value['code'] !== code)
      return err(`code is ${String(parsed.value['code'])}, wanted ${code}`);
    return ok(undefined);
  },
});

export const isWellFormedXml: Check = {
  name: 'well-formed XML',
  run: (body) => {
    const trimmed = body.trim();
    if (!trimmed.startsWith('<?xml')) return err('body does not start with an XML declaration');
    if (!trimmed.includes('<urlset') && !trimmed.includes('<sitemapindex')) {
      return err('body carries no urlset or sitemapindex element');
    }
    return ok(undefined);
  },
};
