import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { EraSlug, PoetSlug } from '@/lib/api/brands';

vi.mock('./client', () => ({
  apiServer: { GET: vi.fn() },
}));

import { failure, ok } from '@/test/api-results';

import { apiServer } from './client';
import { getPoet, getPoetFacets, getPoetSlugsPage, getPoetsPage, movedPoetPath } from './poets';

const get = apiServer.GET as unknown as ReturnType<typeof vi.fn>;
const POET = { name: 'ش', slug: 'poet-x' as PoetSlug, poemsCount: 3 };

beforeEach(() => {
  get.mockReset();
});

describe('getPoetsPage', () => {
  it('returns the page with its cursors', async () => {
    const page = { data: [POET], total: 1, next: null, prev: null };
    get.mockResolvedValue(ok(page));
    expect(await getPoetsPage(undefined)).toEqual(page);
    expect(get).toHaveBeenCalledWith('/poets', { params: { query: {} } });
  });

  it('forwards the cursor and the era and q filters to the API request', async () => {
    get.mockResolvedValue(ok({ data: [POET], total: 40, next: null, prev: 'p' }));
    await getPoetsPage('c', { era: 'jahili' as EraSlug, q: 'متنبي' });
    expect(get).toHaveBeenCalledWith('/poets', {
      params: { query: { cursor: 'c', era: 'jahili', q: 'متنبي' } },
    });
  });

  it('returns null when a cursor leads past the last poet', async () => {
    get.mockResolvedValue(ok({ data: [], total: 150, next: null, prev: null }));
    expect(await getPoetsPage('c')).toBeNull();
  });

  it('returns null when the API refuses the cursor', async () => {
    get.mockResolvedValue(failure(400));
    expect(await getPoetsPage('bad')).toBeNull();
  });

  it('treats the first page of an empty list as a valid empty page', async () => {
    get.mockResolvedValue(ok({ data: [], total: 0, next: null, prev: null }));
    expect((await getPoetsPage(undefined))?.data).toEqual([]);
  });
});

describe('getPoetSlugsPage', () => {
  it('maps slugs and the next cursor', async () => {
    get.mockResolvedValue(ok({ data: ['aBcD', 'eFgH'], total: 4, next: 'n', prev: null }));
    expect(await getPoetSlugsPage(undefined)).toEqual({ slugs: ['aBcD', 'eFgH'], next: 'n' });
    expect(get).toHaveBeenCalledWith('/poets/slugs', { params: { query: {} } });
    await getPoetSlugsPage('n');
    expect(get).toHaveBeenLastCalledWith('/poets/slugs', { params: { query: { cursor: 'n' } } });
  });

  it('returns null on a 404', async () => {
    get.mockResolvedValue(failure(404));
    expect(await getPoetSlugsPage('c')).toBeNull();
  });

  it('rethrows on a 500', async () => {
    get.mockResolvedValue(failure(500));
    await expect(getPoetSlugsPage(undefined)).rejects.toThrow();
  });
});

describe('getPoet', () => {
  it('returns the unwrapped poet on success', async () => {
    get.mockResolvedValue(ok({ data: POET }));
    const result = await getPoet('poet-x' as PoetSlug);
    expect(result).toEqual(POET);
    expect(get).toHaveBeenCalledWith('/poets/{slug}', { params: { path: { slug: 'poet-x' } } });
  });

  it('returns null on a 404', async () => {
    get.mockResolvedValue(failure(404));
    expect(await getPoet('missing' as PoetSlug)).toBeNull();
  });

  it('rethrows on a 500', async () => {
    get.mockResolvedValue(failure(500));
    await expect(getPoet('boom' as PoetSlug)).rejects.toThrow();
  });
});

describe('getPoetFacets', () => {
  const FACETS = {
    meters: [{ name: 'الطويل', slug: 'altawil', poemsCount: 2 }],
    rhymes: [],
    themes: [],
  };

  it('sends the poet and each selected list as query params', async () => {
    get.mockResolvedValue(ok({ data: FACETS }));
    const result = await getPoetFacets('poet-x' as PoetSlug, {
      meter: ['altawil'],
      rhyme: ['meem', 'lam'],
      theme: [],
    });
    expect(result).toEqual(FACETS);
    expect(get).toHaveBeenCalledWith('/poems/facets', {
      params: {
        query: { poet: 'poet-x', meter: ['altawil'], rhyme: ['meem', 'lam'], theme: [] },
      },
    });
  });

  it('returns null on a 404', async () => {
    get.mockResolvedValue(failure(404));
    expect(
      await getPoetFacets('missing' as PoetSlug, { meter: [], rhyme: [], theme: [] })
    ).toBeNull();
  });

  it('rethrows on a 500', async () => {
    get.mockResolvedValue(failure(500));
    await expect(
      getPoetFacets('boom' as PoetSlug, { meter: [], rhyme: [], theme: [] })
    ).rejects.toThrow();
  });
});

describe('movedPoetPath', () => {
  it('is null when the poet answered under the slug that was asked for', () => {
    expect(movedPoetPath('yoFB', { slug: 'yoFB' }, {})).toBeNull();
  });

  it('is the canonical path when the API followed an alias to another slug', () => {
    expect(movedPoetPath('abCD', { slug: 'yoFB' }, {})).toBe('/poets/yoFB');
  });

  it('keeps the cursor the old URL asked for', () => {
    expect(movedPoetPath('abCD', { slug: 'yoFB' }, { cursor: 'c' })).toBe('/poets/yoFB?cursor=c');
  });

  it('keeps the poem filters the old URL asked for', () => {
    expect(movedPoetPath('abCD', { slug: 'yoFB' }, { meter: ['altawil'] })).toBe(
      '/poets/yoFB?meter=altawil'
    );
  });
});
