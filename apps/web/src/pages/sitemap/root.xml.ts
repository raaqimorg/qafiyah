import { SITE_URL } from '@/lib/constants/config';
import { CACHE_SITEMAP } from '@/lib/server/cache';
import { urlsetXml } from '@/lib/server/sitemap';

import type { APIRoute } from 'astro';

export const GET: APIRoute = () => {
  const locs = [`${SITE_URL}/`, `${SITE_URL}/about`, `${SITE_URL}/developers`];
  return new Response(urlsetXml(locs), {
    headers: { 'Content-Type': 'application/xml; charset=utf-8', 'Cache-Control': CACHE_SITEMAP },
  });
};
