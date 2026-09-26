import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { EraSlug, PoetSlug } from '@/lib/api/brands';

vi.mock('./client', () => ({
  apiServer: { GET: vi.fn() },
}));

import { failure, ok } from '@/test/api-results';

import { apiServer } from './client';
import { getPoet, getPoetSlugsPage, getPoetsPage, movedPoetPath } from './poets';

const get = apiServer.GET as unknown as ReturnType<typeof vi.fn>;
const PAGINATION = { page: 1, pageSize: 30, totalPages: 1, totalItems: 1 };
const POET = { name: 'ش', slug: 'poet-x' as PoetSlug, poemsCount: 3 };

beforeEach(() => {
  get.mockReset();
});

describe('getPoetsPage', () => {
  it('maps poets and pagination', async () => {
    get.mockResolvedValue(ok({ data: [POET], pagination: PAGINATION }));
    const result = await getPoetsPage(1);
    expect(result?.poets).toHaveLength(1);
    expect(result?.pagination.totalItems).toBe(1);
  });

  it('forwards era and q filters to the API request', async () => {
    get.mockResolvedValue(ok({ data: [POET], pagination: PAGINATION }));
    await getPoetsPage(1, { era: 'jahili' as EraSlug, q: 'متنبي' });
    expect(get).toHaveBeenCalledWith('/poets', {
      params: { query: { page: '1', era: 'jahili', q: 'متنبي' } },
    });
  });

  it('returns null when the page exceeds the last page', async () => {
    get.mockResolvedValue(
      ok({ data: [], pagination: { page: 999, pageSize: 30, totalPages: 5, totalItems: 150 } })
    );
    expect(await getPoetsPage(999)).toBeNull();
  });

  it('treats page 1 of an empty collection as a valid empty page', async () => {
    get.mockResolvedValue(
      ok({ data: [], pagination: { page: 1, pageSize: 30, totalPages: 1, totalItems: 0 } })
    );
    const result = await getPoetsPage(1);
    expect(result).not.toBeNull();
    expect(result?.poets).toEqual([]);
  });
});

describe('getPoetSlugsPage', () => {
  it('maps slugs and the page count', async () => {
    get.mockResolvedValue(ok({ data: ['aBcD', 'eFgH'], pagination: PAGINATION }));
    const result = await getPoetSlugsPage(1);
    expect(result?.slugs).toEqual(['aBcD', 'eFgH']);
    expect(result?.totalPages).toBe(1);
    expect(get).toHaveBeenCalledWith('/poets/slugs', { params: { query: { page: '1' } } });
  });

  it('returns null on a 404', async () => {
    get.mockResolvedValue(failure(404));
    expect(await getPoetSlugsPage(99)).toBeNull();
  });

  it('rethrows on a 500', async () => {
    get.mockResolvedValue(failure(500));
    await expect(getPoetSlugsPage(1)).rejects.toThrow();
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

describe('movedPoetPath', () => {
  it('is null when the poet answered under the slug that was asked for', () => {
    expect(movedPoetPath('yoFB', { slug: 'yoFB' }, 1)).toBeNull();
  });

  it('is the canonical path when the API followed an alias to another slug', () => {
    expect(movedPoetPath('abCD', { slug: 'yoFB' }, 1)).toBe('/poets/yoFB');
  });

  it('keeps the page number the old URL asked for', () => {
    expect(movedPoetPath('abCD', { slug: 'yoFB' }, 3)).toBe('/poets/yoFB?page=3');
  });
});
