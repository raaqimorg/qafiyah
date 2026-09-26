import { poemUrl } from '@/lib/urls';

import { isNotFoundStatus } from './api-error';
import { apiServer } from './client';
import { apiFailure, getOrNull, safeCall } from './unwrap';

import type { Ok } from './types';
import type {
  CollectionSlug,
  EraSlug,
  MeterSlug,
  PoemSlug,
  PoetSlug,
  RhymeSlug,
  ThemeSlug,
} from '@/lib/api/brands';
import type { PoemNavScope } from '@/lib/poem-context';

export type Poem = Ok<'/poems/{slug}'>['data'];

export type PoemFilters = {
  readonly poetSlugs?: readonly PoetSlug[];
  readonly eraSlugs?: readonly EraSlug[];
  readonly themeSlugs?: readonly ThemeSlug[];
  readonly meterSlugs?: readonly MeterSlug[];
  readonly rhymeSlugs?: readonly RhymeSlug[];
  readonly collectionSlugs?: readonly CollectionSlug[];
};

type PoemsList = Ok<'/poems'>;

export const getPoem = (slug: PoemSlug, by?: PoemNavScope): Promise<Poem | null> =>
  getOrNull(() =>
    apiServer.GET('/poems/{slug}', {
      params: { path: { slug }, query: by === undefined ? {} : { by } },
    })
  );

export function movedPoemPath(requested: string, poem: Pick<Poem, 'slug'>): string | null {
  return poem.slug === requested ? null : poemUrl(poem.slug);
}

export async function listPoems(
  filters: PoemFilters,
  page: number
): Promise<{ poems: PoemsList['data']; pagination: PoemsList['pagination'] } | null> {
  const result = await safeCall(() =>
    apiServer.GET('/poems', {
      params: {
        query: {
          page: String(page),
          poet: [...(filters.poetSlugs ?? [])],
          era: [...(filters.eraSlugs ?? [])],
          theme: [...(filters.themeSlugs ?? [])],
          meter: [...(filters.meterSlugs ?? [])],
          rhyme: [...(filters.rhymeSlugs ?? [])],
          collection: [...(filters.collectionSlugs ?? [])],
        },
      },
    })
  );
  if (result.data === undefined) {
    if (isNotFoundStatus(result.status)) return null;
    throw apiFailure(result);
  }
  if (page > result.data.pagination.totalPages) return null;
  return { poems: result.data.data, pagination: result.data.pagination };
}
