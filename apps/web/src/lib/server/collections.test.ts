import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { CollectionSlug } from '@/lib/api/brands';

vi.mock('./client', () => ({
  apiServer: { GET: vi.fn() },
}));

import { failure, ok } from '@/test/api-results';

import { apiServer } from './client';
import { allCollections, getCollection } from './collections';

const get = apiServer.GET as unknown as ReturnType<typeof vi.fn>;

const COLLECTION = { name: 'المعلقات', slug: 'almuallaqat' as CollectionSlug, poemsCount: 10 };

beforeEach(() => {
  get.mockReset();
});

describe('allCollections', () => {
  it('returns the list data', async () => {
    get.mockResolvedValue(ok({ data: [COLLECTION] }));
    const collections = await allCollections();
    expect(collections).toHaveLength(1);
    expect(collections[0]?.name).toBe('المعلقات');
    expect(get).toHaveBeenCalledWith('/collections');
  });
});

describe('getCollection', () => {
  it('returns the unwrapped collection on success', async () => {
    get.mockResolvedValue(ok({ data: COLLECTION }));
    const collection = await getCollection('almuallaqat' as CollectionSlug);
    expect(collection).toEqual(COLLECTION);
    expect(get).toHaveBeenCalledWith('/collections/{slug}', {
      params: { path: { slug: 'almuallaqat' } },
    });
  });

  it('returns null on a 404', async () => {
    get.mockResolvedValue(failure(404));
    expect(await getCollection('missing' as CollectionSlug)).toBeNull();
  });
});
