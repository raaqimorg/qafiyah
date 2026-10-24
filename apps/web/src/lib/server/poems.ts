import { poemUrl } from '@/lib/urls';

import { isNotFoundStatus } from './api-error';
import { apiServer } from './client';
import { apiFailure, getOrNull, safeCall, unwrap } from './unwrap';

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

export type Poem = Ok<'/poems/{slug}'>['data'];

export type PoemFilters = {
  readonly poetSlugs?: readonly PoetSlug[];
  readonly eraSlugs?: readonly EraSlug[];
  readonly themeSlugs?: readonly ThemeSlug[];
  readonly meterSlugs?: readonly MeterSlug[];
  readonly rhymeSlugs?: readonly RhymeSlug[];
  readonly collectionSlugs?: readonly CollectionSlug[];
};

export type PoemsList = Ok<'/poems'>;

export const getPoemCount = async (): Promise<number> =>
  (await unwrap(() => apiServer.GET('/poems/count'))).total;

export async function getPoemSlugsPage(
  cursor: string | undefined
): Promise<{ slugs: readonly string[]; next: string | null } | null> {
  const result = await safeCall(() =>
    apiServer.GET('/poems/slugs', { params: { query: cursor === undefined ? {} : { cursor } } })
  );
  if (result.data === undefined) {
    if (isNotFoundStatus(result.status)) return null;
    throw apiFailure(result);
  }
  return { slugs: result.data.data, next: result.data.next };
}

export const getPoem = (slug: PoemSlug): Promise<Poem | null> =>
  getOrNull(() => apiServer.GET('/poems/{slug}', { params: { path: { slug } } }));

export function movedPoemPath(requested: string, poem: Pick<Poem, 'slug'>): string | null {
  return poem.slug === requested ? null : poemUrl(poem.slug);
}

export async function listPoems(
  filters: PoemFilters,
  cursor: string | undefined
): Promise<PoemsList | null> {
  const result = await safeCall(() =>
    apiServer.GET('/poems', {
      params: {
        query: {
          ...(cursor === undefined ? {} : { cursor }),
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
  if (cursor !== undefined && result.data.data.length === 0) return null;
  return result.data;
}
