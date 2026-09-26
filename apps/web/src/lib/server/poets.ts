import { poetUrl } from '@/lib/urls';

import { isNotFoundStatus } from './api-error';
import { apiServer } from './client';
import { apiFailure, getOrNull, safeCall } from './unwrap';

import type { Ok } from './types';
import type { EraSlug, PoetSlug } from '@/lib/api/brands';

type PoetsList = Ok<'/poets'>;
export type Poet = Ok<'/poets/{slug}'>['data'];

export async function getPoetsPage(
  page: number,
  opts?: { readonly era?: EraSlug | undefined; readonly q?: string | undefined }
): Promise<{ poets: PoetsList['data']; pagination: PoetsList['pagination'] } | null> {
  const result = await safeCall(() =>
    apiServer.GET('/poets', {
      params: {
        query: {
          page: String(page),
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
  if (page > Math.max(1, result.data.pagination.totalPages)) return null;
  return { poets: result.data.data, pagination: result.data.pagination };
}

export type PoetSlugEntry = Ok<'/poets/slugs'>['data'][number];

export async function getPoetSlugsPage(
  page: number
): Promise<{ slugs: readonly PoetSlugEntry[]; totalPages: number } | null> {
  const result = await safeCall(() =>
    apiServer.GET('/poets/slugs', { params: { query: { page: String(page) } } })
  );
  if (result.data === undefined) {
    if (isNotFoundStatus(result.status)) return null;
    throw apiFailure(result);
  }
  return { slugs: result.data.data, totalPages: result.data.pagination.totalPages };
}

export const getPoet = (slug: PoetSlug): Promise<Poet | null> =>
  getOrNull(() => apiServer.GET('/poets/{slug}', { params: { path: { slug } } }));

export function movedPoetPath(
  requested: string,
  poet: Pick<Poet, 'slug'>,
  page: number
): string | null {
  return poet.slug === requested ? null : poetUrl(poet.slug, page);
}
