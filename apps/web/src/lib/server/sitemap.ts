const POSITIVE_INTEGER_RE = /^\d+$/;

export function parsePageParam(raw: string | undefined): number | null {
  if (raw === undefined || !POSITIVE_INTEGER_RE.test(raw)) return null;
  const page = Number(raw);
  return Number.isInteger(page) && page >= 1 ? page : null;
}

export function shardCount(total: number, perShard: number): number {
  return Math.max(1, Math.ceil(total / perShard));
}

type SlugPage<T> = { readonly slugs: readonly T[]; readonly next: string | null };

type FetchSlugPage<T> = (cursor: string | undefined) => Promise<SlugPage<T> | null>;

export async function nthSlugPage<T>(
  fetchPage: FetchSlugPage<T>,
  n: number
): Promise<SlugPage<T> | null> {
  let page = await fetchPage(undefined);
  for (let index = 1; index < n && page !== null; index += 1) {
    page = page.next === null ? null : await fetchPage(page.next);
  }
  return page;
}

export async function collectPagedSlugs<T>(fetchPage: FetchSlugPage<T>): Promise<T[]> {
  const slugs: T[] = [];
  let page = await fetchPage(undefined);
  while (page !== null) {
    slugs.push(...page.slugs);
    page = page.next === null ? null : await fetchPage(page.next);
  }
  return slugs;
}

function escapeXml(value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&apos;');
}

export type SitemapEntry = { readonly loc: string; readonly imageLoc?: string | undefined };

const IMAGE_NAMESPACE = ' xmlns:image="http://www.google.com/schemas/sitemap-image/1.1"';

function entryXml(entry: SitemapEntry): string {
  const image =
    entry.imageLoc === undefined
      ? ''
      : `<image:image><image:loc>${escapeXml(entry.imageLoc)}</image:loc></image:image>`;
  return `<url><loc>${escapeXml(entry.loc)}</loc>${image}</url>`;
}

export function urlsetXml(entries: readonly (string | SitemapEntry)[]): string {
  const normalized = entries.map((entry) => (typeof entry === 'string' ? { loc: entry } : entry));
  const namespace = normalized.some((entry) => entry.imageLoc !== undefined) ? IMAGE_NAMESPACE : '';
  const body = normalized.map((entry) => entryXml(entry)).join('');
  return `<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"${namespace}>${body}</urlset>`;
}

export function sitemapIndexXml(baseUrl: string, sitemapPaths: readonly string[]): string {
  const body = sitemapPaths
    .map((path) => `<sitemap><loc>${escapeXml(`${baseUrl}${path}`)}</loc></sitemap>`)
    .join('');
  return `<?xml version="1.0" encoding="UTF-8"?><sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">${body}</sitemapindex>`;
}
