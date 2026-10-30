import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { PoemSlug, PoetSlug } from '@/lib/api/brands';

vi.mock('./client', () => ({
  apiServer: { GET: vi.fn() },
}));

import { failure, ok } from '@/test/api-results';

import { apiServer } from './client';
import { getPoem, listPoems, movedPoemPath } from './poems';

const get = apiServer.GET as unknown as ReturnType<typeof vi.fn>;

const POEM = {
  title: 'قصيدة',
  slug: 'a-poem' as PoemSlug,
  verses: [['شطر', 'شطر']],
  verseCount: 1,
  sample: 'شطر',
  keywords: 'k',
  poet: { name: 'شاعر', slug: 'poet-x' },
  era: { name: 'الجاهلي', slug: 'jahili' },
  meter: { name: 'الطويل', slug: 'altaweel' },
  theme: { name: 'مدح', slug: 'theme-1' },
  relatedPoems: [],
};

const POEM_ROW = {
  title: 'ت',
  slug: 'p1',
  poet: { name: 'ش', slug: 'poet-x' },
  meter: { name: 'م', slug: 'meter-x' },
};

describe('getPoem', () => {
  beforeEach(() => {
    get.mockReset();
  });

  it('returns the unwrapped poem on success', async () => {
    get.mockResolvedValue(ok({ data: POEM }));
    const result = await getPoem('a-poem' as PoemSlug);
    expect(result).toEqual(POEM);
    expect(get).toHaveBeenCalledWith('/poems/{slug}', { params: { path: { slug: 'a-poem' } } });
  });

  it('returns null on a 404', async () => {
    get.mockResolvedValue(failure(404));
    expect(await getPoem('missing' as PoemSlug)).toBeNull();
  });

  it('returns null on a 400, which can never resolve either', async () => {
    get.mockResolvedValue(failure(400));
    expect(await getPoem('bad' as PoemSlug)).toBeNull();
  });

  it('rethrows on a 500 (a genuine server error is not a missing poem)', async () => {
    get.mockResolvedValue(failure(500));
    await expect(getPoem('boom' as PoemSlug)).rejects.toThrow();
  });

  it('rethrows when the request never got an answer', async () => {
    get.mockRejectedValue(new Error('network down'));
    await expect(getPoem('boom' as PoemSlug)).rejects.toThrow('network down');
  });
});

describe('listPoems', () => {
  beforeEach(() => {
    get.mockReset();
  });

  it('returns the page with its cursors on success', async () => {
    const page = { data: [POEM_ROW], total: 31, next: 'n', prev: 'p' };
    get.mockResolvedValue(ok(page));
    expect(await listPoems({ poetSlugs: ['poet-x' as PoetSlug] }, 'c')).toEqual(page);
    expect(get).toHaveBeenCalledWith('/poems', {
      params: {
        query: {
          cursor: 'c',
          poet: ['poet-x'],
          era: [],
          theme: [],
          meter: [],
          rhyme: [],
          collection: [],
        },
      },
    });
  });

  it('sends no cursor for the first page', async () => {
    get.mockResolvedValue(ok({ data: [], total: 0, next: null, prev: null }));
    expect((await listPoems({}, undefined))?.data).toEqual([]);
    expect(get.mock.calls[0]?.[1].params.query).not.toHaveProperty('cursor');
  });

  it('returns null when a cursor leads past the last poem, or the API refuses it', async () => {
    get.mockResolvedValue(ok({ data: [], total: 1, next: null, prev: null }));
    expect(await listPoems({}, 'c')).toBeNull();
    get.mockResolvedValue(failure(400));
    expect(await listPoems({}, 'bad')).toBeNull();
  });

  it('rethrows on a 500', async () => {
    get.mockResolvedValue(failure(500));
    await expect(listPoems({}, undefined)).rejects.toThrow();
  });
});

describe('movedPoemPath', () => {
  it('is null when the poem answered under the slug that was asked for', () => {
    expect(movedPoemPath('TnKK', { slug: 'TnKK' })).toBeNull();
  });

  it('is the canonical path when the API followed an alias to another slug', () => {
    expect(movedPoemPath('abCD', { slug: 'TnKK' })).toBe('/poems/TnKK');
  });
});
