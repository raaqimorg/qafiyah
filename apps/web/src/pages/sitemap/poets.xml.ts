import { SITE_URL } from '@/lib/constants/config';
import { CACHE_SITEMAP } from '@/lib/server/cache';
import { getPoetSlugsPage } from '@/lib/server/poets';
import { collectPagedSlugs, type SitemapEntry, urlsetXml } from '@/lib/server/sitemap';
import { poetAvatarUrl, poetsUrl, poetUrl } from '@/lib/urls';

import type { APIRoute } from 'astro';

export const GET: APIRoute = async () => {
  const poets = await collectPagedSlugs(getPoetSlugsPage);
  const entries: readonly SitemapEntry[] = [
    { loc: `${SITE_URL}${poetsUrl()}` },
    ...poets.map((poet) => ({
      loc: `${SITE_URL}${poetUrl(poet.slug)}`,
      ...(poet.hasAvatar ? { imageLoc: poetAvatarUrl(poet.slug) } : {}),
    })),
  ];
  return new Response(urlsetXml(entries), {
    headers: { 'Content-Type': 'application/xml; charset=utf-8', 'Cache-Control': CACHE_SITEMAP },
  });
};
