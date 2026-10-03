import { API_URL } from '@/lib/constants/config';
import { canonicalPoetFilters } from '@/lib/poet-filters';
import { API_V1_PREFIX, CDN_URL } from '@qafiyah/config';

export type TaxonomySection = 'meters' | 'rhymes' | 'themes' | 'collections';

export function xProfileUrl(): string {
  return `${API_URL}${API_V1_PREFIX}/go/x`;
}

export function telegramUrl(): string {
  return `${API_URL}${API_V1_PREFIX}/go/telegram`;
}

export function githubUrl(): string {
  return `${API_URL}${API_V1_PREFIX}/go/github`;
}

export function dbDumpsUrl(): string {
  return `${API_URL}${API_V1_PREFIX}/go/db`;
}

export function avatarsUrl(): string {
  return `${API_URL}${API_V1_PREFIX}/go/avatars`;
}

export function raaqimUrl(): string {
  return `${API_URL}${API_V1_PREFIX}/go/raaqim`;
}

export function isCanonical(url: URL, canonical: string): boolean {
  return `${url.pathname}${url.search}` === canonical;
}

export function retryUrl(originPathname: string, url: URL): string {
  return `${originPathname}${url.search}`;
}

function withPage(path: string, page?: number): string {
  return page !== undefined && page > 1 ? `${path}?page=${page}` : path;
}

export function taxonomyIndexUrl(section: TaxonomySection): string {
  return `/${section}`;
}

export function taxonomyUrl(section: TaxonomySection, slug: string, page?: number): string {
  return withPage(`/${section}/${slug}`, page);
}

export function poetsUrl(opts?: {
  page?: number | undefined;
  era?: string | undefined;
  q?: string | undefined;
}): string {
  const params = new URLSearchParams();
  if (opts?.era !== undefined && opts.era !== '') params.set('era', opts.era);
  if (opts?.q !== undefined && opts.q !== '') params.set('q', opts.q);
  if (opts?.page !== undefined && opts.page > 1) params.set('page', String(opts.page));
  const query = params.toString();
  return query ? `/poets?${query}` : '/poets';
}

export type PoetUrlOptions = {
  readonly page?: number | undefined;
  readonly meter?: readonly string[] | undefined;
  readonly rhyme?: readonly string[] | undefined;
  readonly theme?: readonly string[] | undefined;
};

export function poetUrl(slug: string, opts?: PoetUrlOptions): string {
  const params = new URLSearchParams();
  const filters = canonicalPoetFilters({
    meter: opts?.meter ?? [],
    rhyme: opts?.rhyme ?? [],
    theme: opts?.theme ?? [],
  });
  for (const key of ['meter', 'rhyme', 'theme'] as const) {
    for (const value of filters[key]) params.append(key, value);
  }
  if (opts?.page !== undefined && opts.page > 1) params.set('page', String(opts.page));
  const query = params.toString();
  return query === '' ? `/poets/${slug}` : `/poets/${slug}?${query}`;
}

export function poetAvatarUrl(slug: string): string {
  return `${CDN_URL}/poets/${slug}/avatar.webp`;
}

export function poemUrl(slug: string, highlights?: readonly string[]): string {
  const base = `/poems/${slug}`;
  const terms = highlights?.map((term) => term.trim()).filter(Boolean) ?? [];
  if (terms.length === 0) return base;
  return `${base}#h=${terms.map((term) => encodeURIComponent(term)).join(',')}`;
}
