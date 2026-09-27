import { Result } from 'neverthrow';

import type { FieldResult, Inspector, PageSubject } from '@/inspector';

export type PageMetadataReport = {
  readonly fields: readonly FieldResult[];
};

const TITLE_MAX = 80;
const DESC_MIN = 60;
const DESC_MAX = 320;

const ERROR_SHAPE = '404';
const TWITTER_CARD = 'summary';

const RX = {
  title: /<title[^>]*>([\s\S]*?)<\/title>/i,
  desc: /<meta[^>]*name=["']description["'][^>]*content=["']([^"']+)["']/i,
  canonical: /<link[^>]*rel=["']canonical["'][^>]*href=["']([^"']+)["']/i,
  hreflang:
    /<link[^>]*rel=["']alternate["'][^>]*hreflang=["']([^"']+)["'][^>]*href=["']([^"']+)["']/i,
  ogTitle: /<meta[^>]*property=["']og:title["'][^>]*content=["']([^"']+)["']/i,
  ogDescription: /<meta[^>]*property=["']og:description["'][^>]*content=["']([^"']+)["']/i,
  ogUrl: /<meta[^>]*property=["']og:url["'][^>]*content=["']([^"']+)["']/i,
  ogImage: /<meta[^>]*property=["']og:image["'][^>]*content=["']([^"']+)["']/i,
  ogType: /<meta[^>]*property=["']og:type["'][^>]*content=["']([^"']+)["']/i,
  twitterCard: /<meta[^>]*name=["']twitter:card["'][^>]*content=["']([^"']+)["']/i,
  twitterTitle: /<meta[^>]*name=["']twitter:title["'][^>]*content=["']([^"']+)["']/i,
  twitterDescription: /<meta[^>]*name=["']twitter:description["'][^>]*content=["']([^"']+)["']/i,
  twitterImage: /<meta[^>]*name=["']twitter:image["'][^>]*content=["']([^"']+)["']/i,
  h1: /<h1\b[^>]*>([\s\S]*?)<\/h1>/gi,
  jsonLd: /<script\b[^>]*application\/ld\+json[^>]*>([\s\S]*?)<\/script>/gi,
};

const safeParseJson = Result.fromThrowable(
  (raw: string): unknown => JSON.parse(raw),
  (): string => 'invalid JSON'
);

function field(label: string, value: string | undefined, ok: boolean): FieldResult {
  return { label, value: value ?? '(missing)', ok };
}

export function inspectPageMetadata(html: string, shape: string): PageMetadataReport {
  const fields: FieldResult[] = [];

  const title = RX.title.exec(html)?.[1]?.trim();
  fields.push(
    field('title', title, title !== undefined && title.length > 0 && title.length <= TITLE_MAX)
  );

  if (shape === ERROR_SHAPE) {
    return { fields };
  }

  const desc = RX.desc.exec(html)?.[1]?.trim();
  fields.push(
    field(
      'description',
      desc,
      desc !== undefined && desc.length >= DESC_MIN && desc.length <= DESC_MAX
    )
  );

  const canonical = RX.canonical.exec(html)?.[1]?.trim();
  fields.push(field('canonical', canonical, Boolean(canonical)));

  const hreflang = RX.hreflang.exec(html);
  fields.push(
    field(
      'hreflang',
      hreflang ? `${hreflang[1]} -> ${hreflang[2]}` : undefined,
      hreflang !== null && canonical !== undefined && hreflang[2]?.trim() === canonical
    )
  );

  const ogTitle = RX.ogTitle.exec(html)?.[1]?.trim();
  fields.push(field('og:title', ogTitle, Boolean(ogTitle)));

  const ogDescription = RX.ogDescription.exec(html)?.[1]?.trim();
  fields.push(field('og:description', ogDescription, Boolean(ogDescription)));

  const ogUrl = RX.ogUrl.exec(html)?.[1]?.trim();
  fields.push(field('og:url', ogUrl, Boolean(ogUrl)));

  const ogImage = RX.ogImage.exec(html)?.[1]?.trim();
  fields.push(field('og:image', ogImage, Boolean(ogImage)));

  const ogType = RX.ogType.exec(html)?.[1]?.trim();
  fields.push(field('og:type', ogType, Boolean(ogType)));

  const twitterCard = RX.twitterCard.exec(html)?.[1]?.trim();
  fields.push(field('twitter:card', twitterCard, twitterCard === TWITTER_CARD));

  const twitterTitle = RX.twitterTitle.exec(html)?.[1]?.trim();
  fields.push(field('twitter:title', twitterTitle, Boolean(twitterTitle)));

  const twitterDescription = RX.twitterDescription.exec(html)?.[1]?.trim();
  fields.push(field('twitter:description', twitterDescription, Boolean(twitterDescription)));

  const twitterImage = RX.twitterImage.exec(html)?.[1]?.trim();
  fields.push(field('twitter:image', twitterImage, Boolean(twitterImage)));

  const h1Matches = [...html.matchAll(RX.h1)];
  fields.push(field('h1 count', String(h1Matches.length), h1Matches.length === 1));

  const jsonLdBlocks = [...html.matchAll(RX.jsonLd)];
  if (jsonLdBlocks.length === 0) {
    fields.push(field('json-ld', undefined, false));
  } else {
    jsonLdBlocks.forEach((match, index) => {
      const raw = match[1] ?? '';
      const parsed = safeParseJson(raw);
      fields.push(
        field(
          `json-ld[${index}]`,
          parsed.isOk() ? JSON.stringify(parsed.value, null, 2) : raw,
          parsed.isOk()
        )
      );
    });
  }

  return { fields };
}

export const pageMetadataInspector: Inspector = {
  id: 'page-metadata',
  title: 'Page metadata',
  inspect(subject: PageSubject): readonly FieldResult[] {
    return inspectPageMetadata(subject.html, subject.shape).fields;
  },
};
