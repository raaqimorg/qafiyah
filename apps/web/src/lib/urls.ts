import { API_URL } from '@/lib/constants/config';
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

export function poetUrl(slug: string, page?: number): string {
  return withPage(`/poets/${slug}`, page);
}

export function poetAvatarUrl(slug: string): string {
  return `${CDN_URL}/poets/${slug}/avatar.webp`;
}

export function poemInSectionUrl(slug: string, section: TaxonomySection | undefined): string {
  return section === undefined ? poemUrl(slug) : `${poemUrl(slug)}?from=${section}`;
}

export function poemUrl(slug: string, highlights?: readonly string[]): string {
  const base = `/poems/${slug}`;
  const terms = highlights?.map((term) => term.trim()).filter(Boolean) ?? [];
  if (terms.length === 0) return base;
  return `${base}#h=${terms.map((term) => encodeURIComponent(term)).join(',')}`;
}
