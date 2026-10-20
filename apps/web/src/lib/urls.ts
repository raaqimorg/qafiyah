import { API_URL } from '@/lib/constants/config';
import { canonicalPoetFilters } from '@/lib/poet-filters';
import {
  API_V1_PREFIX,
  CDN_URL,
  FACEBOOK_SHARE_URL,
  LINKEDIN_SHARE_URL,
  REDDIT_SHARE_URL,
  TELEGRAM_SHARE_URL,
  WHATSAPP_SHARE_URL,
  X_INTENT_TWEET_URL,
} from '@qafiyah/config';

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

export const SHARE_NETWORK_VALUES = [
  'x',
  'facebook',
  'whatsapp',
  'telegram',
  'reddit',
  'linkedin',
] as const;

export type ShareNetwork = (typeof SHARE_NETWORK_VALUES)[number];

export function shareUrl(network: ShareNetwork, path: string): string {
  return `${API_URL}${API_V1_PREFIX}/go/${network}-share?path=${encodeURIComponent(path)}`;
}

export type SharePayload = {
  readonly body: string;
  readonly text?: string;
  readonly title: string;
  readonly url: string;
};

export function shareIntent(network: ShareNetwork, payload: SharePayload): string {
  const { body, title, url } = payload;
  const text = payload.text ?? `${body}\n\n${url}`;
  switch (network) {
    case 'x':
      return `${X_INTENT_TWEET_URL}?text=${encodeURIComponent(text)}`;
    case 'whatsapp':
      return `${WHATSAPP_SHARE_URL}?text=${encodeURIComponent(text)}`;
    case 'telegram':
      return `${TELEGRAM_SHARE_URL}?url=${encodeURIComponent(url)}&text=${encodeURIComponent(body)}`;
    case 'reddit':
      return `${REDDIT_SHARE_URL}?url=${encodeURIComponent(url)}&title=${encodeURIComponent(title)}`;
    case 'facebook':
      return `${FACEBOOK_SHARE_URL}?u=${encodeURIComponent(url)}`;
    case 'linkedin':
      return `${LINKEDIN_SHARE_URL}?url=${encodeURIComponent(url)}`;
  }
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
  return poemHighlightUrl(`/poems/${slug}`, highlights ?? []);
}

export function poemHighlightUrl(url: string, highlights: readonly string[]): string {
  const base = url.split('#')[0] ?? url;
  const terms = highlights.map((term) => term.trim()).filter(Boolean);
  if (terms.length === 0) return base;
  return `${base}#h=${terms.map((term) => encodeURIComponent(term)).join(',')}`;
}
