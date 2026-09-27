import { err, ok } from 'neverthrow';

import { WEB } from '../target';

import type { Check, Probe } from '../types';

const TITLE_MAX = 80;
const DESC_MIN = 60;
const DESC_MAX = 320;

const RX = {
  title: /<title[^>]*>([\s\S]*?)<\/title>/i,
  desc: /<meta[^>]*name=["']description["'][^>]*content=["']([^"']+)["']/i,
  canonical: /<link[^>]*rel=["']canonical["'][^>]*href=["']([^"']+)["']/i,
  hreflang:
    /<link[^>]*rel=["']alternate["'][^>]*hreflang=["']([^"']+)["'][^>]*href=["']([^"']+)["']/i,
  ogImage: /<meta[^>]*property=["']og:image["'][^>]*content=["']([^"']+)["']/i,
  ogType: /<meta[^>]*property=["']og:type["'][^>]*content=["']([^"']+)["']/i,
  twitterCard: /<meta[^>]*name=["']twitter:card["'][^>]*content=["']([^"']+)["']/i,
  h1: /<h1\b[^>]*>([\s\S]*?)<\/h1>/gi,
  ld: /<script\b[^>]*application\/ld\+json[^>]*>([\s\S]*?)<\/script>/gi,
};

function titleWithinLimit(): Check {
  return {
    name: 'title present and within limit',
    run: (body) => {
      const title = RX.title.exec(body)?.[1]?.trim();
      if (!title) return err('missing <title>');
      if (title.length > TITLE_MAX) return err(`title ${title.length} chars`);
      return ok(undefined);
    },
  };
}

function descriptionInRange(): Check {
  return {
    name: 'description within range',
    run: (body) => {
      const desc = RX.desc.exec(body)?.[1]?.trim();
      if (!desc) return err('missing <meta description>');
      if (desc.length < DESC_MIN || desc.length > DESC_MAX) {
        return err(`description length ${desc.length} not in [${DESC_MIN},${DESC_MAX}]`);
      }
      return ok(undefined);
    },
  };
}

const hasCanonical: Check = {
  name: 'canonical present',
  run: (body) => (RX.canonical.test(body) ? ok(undefined) : err('missing canonical')),
};

const selfReferencingHreflang: Check = {
  name: 'self-referencing hreflang',
  run: (body) => {
    const hreflang = RX.hreflang.exec(body);
    if (!hreflang) return err('missing self-referencing hreflang');
    const canonicalHref = RX.canonical.exec(body)?.[1]?.trim();
    if (hreflang[2]?.trim() !== canonicalHref) {
      return err(`hreflang href "${hreflang[2]}" does not match canonical`);
    }
    return ok(undefined);
  },
};

const hasOgTags: Check = {
  name: 'open graph tags present',
  run: (body) => {
    if (!RX.ogImage.test(body)) return err('missing og:image');
    if (!RX.ogType.test(body)) return err('missing og:type');
    return ok(undefined);
  },
};

function twitterCardIs(expected: string): Check {
  return {
    name: `twitter:card ${expected}`,
    run: (body) => {
      const card = RX.twitterCard.exec(body)?.[1]?.trim();
      return card === expected
        ? ok(undefined)
        : err(`twitter:card is "${card ?? 'missing'}", wanted ${expected}`);
    },
  };
}

const exactlyOneH1: Check = {
  name: 'exactly one h1',
  run: (body) => {
    const count = [...body.matchAll(RX.h1)].length;
    return count === 1 ? ok(undefined) : err(`expected 1 <h1>, got ${count}`);
  },
};

const jsonLdParses: Check = {
  name: 'json-ld parses',
  run: (body) => {
    for (const match of body.matchAll(RX.ld)) {
      try {
        JSON.parse(match[1] ?? '');
      } catch (cause) {
        return err(`invalid JSON-LD (${cause instanceof Error ? cause.message : String(cause)})`);
      }
    }
    return ok(undefined);
  },
};

function jsonLdHasTypes(expected: readonly string[]): Check {
  return {
    name: `json-ld has ${expected.join(', ')}`,
    run: (body) => {
      const found = new Set<string>();
      for (const match of body.matchAll(RX.ld)) {
        try {
          const parsed: unknown = JSON.parse(match[1] ?? '');
          if (parsed !== null && typeof parsed === 'object' && '@type' in parsed) {
            const typeValue = (parsed as Record<string, unknown>)['@type'];
            if (typeof typeValue === 'string') found.add(typeValue);
          }
        } catch {
          return err('invalid JSON-LD');
        }
      }
      for (const type of expected) {
        if (!found.has(type)) return err(`missing JSON-LD @type "${type}"`);
      }
      return ok(undefined);
    },
  };
}

const POEM_SLUG = process.env['SEO_POEM_SLUG'];
const ERA_SLUG = process.env['SEO_ERA_SLUG'];
const POET_SLUG = process.env['SEO_POET_SLUG'];

const EXPECTED_JSON_LD_TYPES: Record<string, readonly string[]> = {
  '/': ['WebSite', 'Organization'],
  '/poets': ['CollectionPage'],
};
if (ERA_SLUG) EXPECTED_JSON_LD_TYPES[`/poets?era=${ERA_SLUG}`] = ['CollectionPage'];
if (POET_SLUG)
  EXPECTED_JSON_LD_TYPES[`/poets/${POET_SLUG}`] = ['Person', 'ItemList', 'BreadcrumbList'];
if (POEM_SLUG) EXPECTED_JSON_LD_TYPES[`/poems/${POEM_SLUG}`] = ['CreativeWork', 'BreadcrumbList'];

const TWITTER_CARD = 'summary';

const paths = ['/', '/meters', '/rhymes', '/themes', '/collections', '/poets', '/404'];
if (ERA_SLUG) paths.push(`/poets?era=${ERA_SLUG}`);
if (POET_SLUG) paths.push(`/poets/${POET_SLUG}`);
if (POEM_SLUG) paths.push(`/poems/${POEM_SLUG}`);

function checksFor(path: string): readonly Check[] {
  const is404 = path === '/404';
  const expectedTypes = EXPECTED_JSON_LD_TYPES[path];
  const base: Check[] = [titleWithinLimit(), jsonLdParses];
  if (is404) {
    return expectedTypes ? [...base, jsonLdHasTypes(expectedTypes)] : base;
  }
  return [
    ...base,
    descriptionInRange(),
    hasCanonical,
    selfReferencingHreflang,
    hasOgTags,
    twitterCardIs(TWITTER_CARD),
    exactlyOneH1,
    ...(expectedTypes ? [jsonLdHasTypes(expectedTypes)] : []),
  ];
}

export const seoProbes: readonly Probe[] = paths.map((path) => ({
  url: `${WEB}${path}`,
  note: `seo shape for ${path}`,
  expect: path === '/404' ? 'not-found' : 'ok',
  checks: checksFor(path),
  surfaces: ['origin', 'stack', 'prod'],
}));
