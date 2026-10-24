import { describe, expect, it } from 'vitest';

import { SITEMAP_POEMS_PER_SHARD } from '@qafiyah/config';

import {
  collectPagedSlugs,
  nthSlugPage,
  parsePageParam,
  shardCount,
  sitemapIndexXml,
  urlsetXml,
} from './sitemap';

describe('urlsetXml', () => {
  it('wraps locs in a urlset', () => {
    const xml = urlsetXml(['https://qafiyah.com/poems/a', 'https://qafiyah.com/poems/b']);
    expect(xml).toContain('<?xml version="1.0" encoding="UTF-8"?>');
    expect(xml).toContain('<urlset');
    expect(xml).toContain('<url><loc>https://qafiyah.com/poems/a</loc></url>');
    expect(xml).toContain('<url><loc>https://qafiyah.com/poems/b</loc></url>');
  });

  it('omits the image namespace when no entry carries an image', () => {
    const xml = urlsetXml(['https://qafiyah.com/poems/a', { loc: 'https://qafiyah.com/poems/b' }]);
    expect(xml).not.toContain('xmlns:image');
    expect(xml).not.toContain('<image:image>');
  });

  it('declares the image namespace and nests the avatar when an entry carries one', () => {
    const xml = urlsetXml([
      {
        loc: 'https://qafiyah.com/poets/GuRy',
        imageLoc: 'https://cdn.qafiyah.com/poets/GuRy/avatar.webp',
      },
    ]);
    expect(xml).toContain('xmlns:image="http://www.google.com/schemas/sitemap-image/1.1"');
    expect(xml).toContain(
      '<url><loc>https://qafiyah.com/poets/GuRy</loc><image:image><image:loc>https://cdn.qafiyah.com/poets/GuRy/avatar.webp</image:loc></image:image></url>'
    );
  });

  it('leaves avatar-less entries as a bare loc alongside ones that have images', () => {
    const xml = urlsetXml([
      {
        loc: 'https://qafiyah.com/poets/GuRy',
        imageLoc: 'https://cdn.qafiyah.com/poets/GuRy/avatar.webp',
      },
      { loc: 'https://qafiyah.com/poets/tpep' },
    ]);
    expect(xml).toContain('<url><loc>https://qafiyah.com/poets/tpep</loc></url>');
    expect(xml.match(/<image:image>/g)).toHaveLength(1);
  });

  it('escapes an image loc', () => {
    const xml = urlsetXml([
      { loc: 'https://qafiyah.com/p', imageLoc: 'https://cdn.qafiyah.com/a?x=1&y=2' },
    ]);
    expect(xml).toContain('<image:loc>https://cdn.qafiyah.com/a?x=1&amp;y=2</image:loc>');
  });
});

describe('sitemapIndexXml', () => {
  it('lists child sitemap URLs under the given base URL', () => {
    const xml = sitemapIndexXml('https://qafiyah.com', [
      '/sitemap/poems/1.xml',
      '/sitemap/poets.xml',
    ]);
    expect(xml).toContain('<sitemapindex');
    expect(xml).toContain('<sitemap><loc>https://qafiyah.com/sitemap/poems/1.xml</loc></sitemap>');
    expect(xml).toContain('<sitemap><loc>https://qafiyah.com/sitemap/poets.xml</loc></sitemap>');
  });
  it('uses whatever base URL it is given (env-aware caller)', () => {
    const xml = sitemapIndexXml('http://localhost:4321', ['/sitemap/poets.xml']);
    expect(xml).toContain('<sitemap><loc>http://localhost:4321/sitemap/poets.xml</loc></sitemap>');
  });
});

describe('shardCount', () => {
  it('is at least 1', () => expect(shardCount(0, SITEMAP_POEMS_PER_SHARD)).toBe(1));
  it('ceils to the shard size', () => {
    expect(shardCount(SITEMAP_POEMS_PER_SHARD, SITEMAP_POEMS_PER_SHARD)).toBe(1);
    expect(shardCount(SITEMAP_POEMS_PER_SHARD + 1, SITEMAP_POEMS_PER_SHARD)).toBe(2);
  });
});

describe('parsePageParam', () => {
  it('parses a positive integer string', () => {
    expect(parsePageParam('1')).toBe(1);
    expect(parsePageParam('42')).toBe(42);
  });
  it('returns null for undefined, non-numeric, zero, negative, or decimal', () => {
    expect(parsePageParam(undefined)).toBeNull();
    expect(parsePageParam('abc')).toBeNull();
    expect(parsePageParam('0')).toBeNull();
    expect(parsePageParam('-3')).toBeNull();
    expect(parsePageParam('1.5')).toBeNull();
  });
});

const PAGES: Record<string, { slugs: string[]; next: string | null }> = {
  first: { slugs: ['a', 'b'], next: 'c2' },
  c2: { slugs: ['c', 'd'], next: 'c3' },
  c3: { slugs: ['e'], next: null },
};

const fetchPage = (cursor: string | undefined) => Promise.resolve(PAGES[cursor ?? 'first'] ?? null);

describe('collectPagedSlugs', () => {
  it('follows next through every page, in page order', async () => {
    expect(await collectPagedSlugs(fetchPage)).toEqual(['a', 'b', 'c', 'd', 'e']);
  });

  it('returns an empty list when the first page is missing', async () => {
    expect(await collectPagedSlugs(() => Promise.resolve(null))).toEqual([]);
  });
});

describe('nthSlugPage', () => {
  it('walks next to the page asked for', async () => {
    expect((await nthSlugPage(fetchPage, 1))?.slugs).toEqual(['a', 'b']);
    expect((await nthSlugPage(fetchPage, 3))?.slugs).toEqual(['e']);
  });

  it('is null past the last page', async () => {
    expect(await nthSlugPage(fetchPage, 4)).toBeNull();
  });
});
