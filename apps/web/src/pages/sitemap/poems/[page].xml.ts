import { SITE_URL } from '@/lib/constants/config';
import { parsePageParam } from '@/lib/pagination';
import { CACHE_SITEMAP } from '@/lib/server/cache';
import { getPoemSlugsPage } from '@/lib/server/poems';
import { urlsetXml } from '@/lib/server/sitemap';
import { poemUrl } from '@/lib/urls';

import type { APIRoute } from 'astro';

export const GET: APIRoute = async ({ params }) => {
  const page = parsePageParam(params['page']);
  if (page === null) return new Response('Not found', { status: 404 });
  const slugs = await getPoemSlugsPage(page);
  if (slugs === null || slugs.length === 0) return new Response('Not found', { status: 404 });
  const locs = slugs.map((slug) => `${SITE_URL}${poemUrl(slug)}`);
  return new Response(urlsetXml(locs), {
    headers: { 'Content-Type': 'application/xml; charset=utf-8', 'Cache-Control': CACHE_SITEMAP },
  });
};
