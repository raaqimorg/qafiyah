import { isNotFoundStatus } from './api-error';
import { apiServer } from './client';
import { apiFailure, safeCall, unwrap } from './unwrap';

import type { Ok } from './types';

export type FeelingRow = Ok<'/feelings'>['data'][number];
type VersesPage = Ok<'/feelings/{slug}/verses'>;
export type FeelingVerseRow = VersesPage['data'][number];

export const allFeelings = (): Promise<readonly FeelingRow[]> =>
  unwrap(() => apiServer.GET('/feelings'));

export async function feelingVerses(
  slug: string,
  page: number
): Promise<{ verses: VersesPage['data']; pagination: VersesPage['pagination'] } | null> {
  const result = await safeCall(() =>
    apiServer.GET('/feelings/{slug}/verses', {
      params: { path: { slug }, query: { page: String(page) } },
    })
  );
  if (result.data === undefined) {
    if (isNotFoundStatus(result.status)) return null;
    throw apiFailure(result);
  }
  if (page > result.data.pagination.totalPages) return null;
  return { verses: result.data.data, pagination: result.data.pagination };
}
