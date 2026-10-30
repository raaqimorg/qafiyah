import { isNotFoundStatus } from './api-error';
import { apiServer } from './client';
import { apiFailure, safeCall, unwrap } from './unwrap';

import type { Ok } from './types';

export type FeelingRow = Ok<'/feelings'>['data'][number];
export type VersesPage = Ok<'/feelings/{slug}/verses'>;
export type FeelingVerseRow = VersesPage['data'][number];

export const allFeelings = (): Promise<readonly FeelingRow[]> =>
  unwrap(() => apiServer.GET('/feelings'));

export async function feelingVerses(
  slug: string,
  cursor: string | undefined
): Promise<VersesPage | null> {
  const result = await safeCall(() =>
    apiServer.GET('/feelings/{slug}/verses', {
      params: { path: { slug }, query: cursor === undefined ? {} : { cursor } },
    })
  );
  if (result.data === undefined) {
    if (isNotFoundStatus(result.status)) return null;
    throw apiFailure(result);
  }
  if (cursor !== undefined && result.data.data.length === 0) return null;
  return result.data;
}
