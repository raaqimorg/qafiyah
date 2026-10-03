import { SITE_URL } from '@/lib/constants/config';
import { CACHE_SITEMAP } from '@/lib/server/cache';
import { getPoemCount } from '@/lib/server/poems';
import { shardCount, sitemapIndexXml } from '@/lib/server/sitemap';
import { SITEMAP_POEMS_PER_SHARD } from '@qafiyah/config';

import type { APIRoute } from 'astro';

export const GET: APIRoute = async () => {
  const poemShards = shardCount(await getPoemCount(), SITEMAP_POEMS_PER_SHARD);
  const paths = [
    '/sitemap/root.xml',
    ...Array.from({ length: poemShards }, (_, i) => `/sitemap/poems/${i + 1}.xml`),
    '/sitemap/poets.xml',
    '/sitemap/taxonomies.xml',
  ];
  return new Response(sitemapIndexXml(SITE_URL, paths), {
    headers: { 'Content-Type': 'application/xml; charset=utf-8', 'Cache-Control': CACHE_SITEMAP },
  });
};
