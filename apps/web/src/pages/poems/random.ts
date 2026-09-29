import { fetchRandomPoemSlugWithRetry } from '@/lib/api/random-poem';
import { reportError } from '@/lib/observability/report-error';
import { INTERNAL_API_KEY, INTERNAL_API_URL } from '@/lib/server/env';
import { poemUrl } from '@/lib/urls';
import { API_KEY_HEADER } from '@qafiyah/config';

import type { APIRoute } from 'astro';

export const prerender = false;

export const GET: APIRoute = async ({ rewrite }) => {
  const result = await fetchRandomPoemSlugWithRetry(
    INTERNAL_API_URL,
    INTERNAL_API_KEY ? { headers: { [API_KEY_HEADER]: INTERNAL_API_KEY } } : {}
  );
  if (result.isErr()) {
    console.error('random poem redirect failed', result.error);
    reportError('random poem redirect failed', result.error, {
      feature: 'random-poem',
      tags: { surface: 'server', kind: result.error.kind },
    });
    return await rewrite('/500');
  }
  return new Response(null, {
    status: 302,
    headers: { Location: poemUrl(result.value), 'Cache-Control': 'no-store' },
  });
};
