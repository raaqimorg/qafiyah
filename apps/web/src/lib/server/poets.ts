import { poetUrl, type PoetUrlOptions } from '@/lib/urls';

import { isNotFoundStatus } from './api-error';
import { apiServer } from './client';
import { apiFailure, getOrNull, safeCall } from './unwrap';

import type { Ok } from './types';
import type { EraSlug, PoetSlug } from '@/lib/api/brands';
import type { PoetFilterSelection } from '@/lib/poet-filters';

export type PoetsList = Ok<'/poets'>;
export type Poet = Ok<'/poets/{slug}'>['data'];

export async function getPoetsPage(
  cursor: string | undefined,
  opts?: { readonly era?: EraSlug | undefined; readonly q?: string | undefined }
): Promise<PoetsList | null> {
  const result = await safeCall(() =>
    apiServer.GET('/poets', {
      params: {
        query: {
          ...(cursor === undefined ? {} : { cursor }),
          ...(opts?.era !== undefined && opts.era !== '' ? { era: opts.era } : {}),
          ...(opts?.q !== undefined && opts.q !== '' ? { q: opts.q } : {}),
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

export type PoetSlugEntry = Ok<'/poets/slugs'>['data'][number];

export async function getPoetSlugsPage(
  cursor: string | undefined
): Promise<{ slugs: readonly PoetSlugEntry[]; next: string | null } | null> {
  const result = await safeCall(() =>
    apiServer.GET('/poets/slugs', { params: { query: cursor === undefined ? {} : { cursor } } })
  );
  if (result.data === undefined) {
    if (isNotFoundStatus(result.status)) return null;
    throw apiFailure(result);
  }
  return { slugs: result.data.data, next: result.data.next };
}

export const getPoet = (slug: PoetSlug): Promise<Poet | null> =>
  getOrNull(() => apiServer.GET('/poets/{slug}', { params: { path: { slug } } }));

export type PoetFacets = Ok<'/poems/facets'>['data'];

export const getPoetFacets = (
  slug: PoetSlug,
  selection: PoetFilterSelection
): Promise<PoetFacets | null> =>
  getOrNull(() =>
    apiServer.GET('/poems/facets', {
      params: {
        query: {
          poet: slug,
          meter: [...selection.meter],
          rhyme: [...selection.rhyme],
          theme: [...selection.theme],
        },
      },
    })
  );

export function movedPoetPath(
  requested: string,
  poet: Pick<Poet, 'slug'>,
  opts: PoetUrlOptions
): string | null {
  return poet.slug === requested ? null : poetUrl(poet.slug, opts);
}
