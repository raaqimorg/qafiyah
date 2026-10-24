import { formatArabicCount } from '@/lib/arabic';
import { SITE_URL } from '@/lib/constants/config';
import { ERROR_TEXTS } from '@/lib/constants/copy';
import { SITE_NAME_AR } from '@/lib/constants/site-meta';
import { POEMS_NOUN_FORMS } from '@/lib/constants/taxonomy-data';
import { derivePagination } from '@/lib/pagination';
import {
  type BreadcrumbItem,
  buildBreadcrumbList,
  type BreadcrumbListDoc,
} from '@/lib/seo/json-ld/breadcrumb-list';
import { collectionPageNode, type CollectionPageDoc } from '@/lib/seo/json-ld/collection-page';
import { buildItemList } from '@/lib/seo/json-ld/item-list';
import {
  buildThingRef,
  type CollectionRef,
  type CreativeWorkRef,
} from '@/lib/seo/json-ld/thing-ref';
import { websiteRef } from '@/lib/seo/json-ld/website';
import { withBrand } from '@/lib/seo/meta-text';
import { listPoems, type PoemsList } from '@/lib/server/poems';
import { poemUrl, type TaxonomySection, taxonomyIndexUrl, taxonomyUrl } from '@/lib/urls';

import {
  INDEX_PAGE_CONFIG,
  TERM_PAGE_CONFIG,
  type IndexTermData,
  type TermData,
} from './taxonomy-copy';

type ListCardItem = { readonly title: string; readonly subtitle: string; readonly href: string };

type LayoutView = {
  readonly title: string;
  readonly description: string;
  readonly canonical: string;
  readonly prevUrl?: string | undefined;
  readonly nextUrl?: string | undefined;
  readonly jsonLd: readonly [CollectionPageDoc<CreativeWorkRef | CollectionRef>, BreadcrumbListDoc];
};

export type TaxonomyTermLoad = {
  readonly term: TermData;
  readonly cursor: string | undefined;
  readonly page: PoemsList;
};

export async function loadTaxonomyTerm(
  section: TaxonomySection,
  slug: string,
  cursor: string | undefined
): Promise<TaxonomyTermLoad | null> {
  const cfg = TERM_PAGE_CONFIG[section];
  const [term, page] = await Promise.all([cfg.get(slug), listPoems(cfg.makeFilters(slug), cursor)]);
  if (!(term && page)) return null;
  return { term, cursor, page };
}

export function buildTaxonomyTermView(section: TaxonomySection, load: TaxonomyTermLoad) {
  const cfg = TERM_PAGE_CONFIG[section];
  const { term, cursor, page } = load;
  const poems = page.data;
  const { slug, name } = term;
  const poemsLabel = formatArabicCount({ count: term.poemsCount, nounForms: POEMS_NOUN_FORMS });

  const collectionJsonLd = collectionPageNode<CreativeWorkRef>({
    name: cfg.jsonLdName(name),
    url: `${SITE_URL}${taxonomyUrl(section, slug, cursor)}`,
    description: cfg.jsonLdDescLead(name),
    isPartOf: websiteRef(),
    mainEntity: buildItemList(
      poems.map((poem) =>
        buildThingRef({
          '@type': 'CreativeWork',
          name: poem.title,
          url: `${SITE_URL}${poemUrl(poem.slug)}`,
        })
      )
    ),
  });

  const crumbItems: readonly BreadcrumbItem[] = [
    { name: SITE_NAME_AR, path: '/' },
    { name: cfg.crumbLabel, path: taxonomyIndexUrl(section) },
    { name, path: taxonomyUrl(section, slug) },
  ];

  const pag = derivePagination(page, (c) => taxonomyUrl(section, slug, c));

  const items: readonly ListCardItem[] = poems.map((poem) => ({
    title: poem.title,
    subtitle: cfg.secondary(poem),
    href: poemUrl(poem.slug),
  }));

  const layout: LayoutView = {
    title: withBrand(cfg.titleLead(name)),
    description: cfg.description(name, poemsLabel, poems),
    canonical: taxonomyUrl(section, slug, cursor),
    prevUrl: pag.prevUrl,
    nextUrl: pag.nextUrl,
    jsonLd: [collectionJsonLd, buildBreadcrumbList(crumbItems)],
  };

  return {
    layout,
    body: {
      heading: cfg.heading(name, poemsLabel),
      crumbItems,
      items,
      emptyText: cfg.emptyText,
      pagination: pag,
    },
  };
}

export async function loadTaxonomyIndex(
  section: TaxonomySection
): Promise<readonly IndexTermData[]> {
  const cfg = INDEX_PAGE_CONFIG[section];
  const terms = await cfg.all();
  const predicate = cfg.filter;
  return predicate ? terms.filter((term) => predicate(term)) : terms;
}

export function buildTaxonomyIndexView(section: TaxonomySection, terms: readonly IndexTermData[]) {
  const cfg = INDEX_PAGE_CONFIG[section];
  const termsLabel = formatArabicCount({ count: terms.length, nounForms: cfg.nounForms });

  const collectionJsonLd = collectionPageNode<CollectionRef>({
    name: cfg.jsonLdName,
    url: `${SITE_URL}${taxonomyIndexUrl(section)}`,
    description: cfg.jsonLdListDescription(termsLabel),
    isPartOf: websiteRef(),
    mainEntity: buildItemList(
      terms.map((term) =>
        buildThingRef({
          '@type': 'Collection',
          name: term.name,
          url: `${SITE_URL}${taxonomyUrl(section, term.slug)}`,
          description: cfg.jsonLdItemDescription(
            term.name,
            formatArabicCount({ count: term.poemsCount, nounForms: POEMS_NOUN_FORMS })
          ),
        })
      )
    ),
  });

  const crumbsJsonLd = buildBreadcrumbList([
    { name: SITE_NAME_AR, path: '/' },
    { name: cfg.crumbLabel, path: taxonomyIndexUrl(section) },
  ]);

  const items: readonly ListCardItem[] = terms.map((term) => ({
    title: term.name,
    subtitle: cfg.subtitle(term),
    href: taxonomyUrl(section, term.slug),
  }));

  const layout: LayoutView = {
    title: cfg.metaTitle,
    description: cfg.metaDescription(terms, termsLabel),
    canonical: taxonomyIndexUrl(section),
    jsonLd: [collectionJsonLd, crumbsJsonLd],
  };

  return {
    layout,
    body: {
      heading: cfg.heading(termsLabel),
      items,
      emptyText: ERROR_TEXTS.loadFailed,
      emptyVariant: 'error' as const,
    },
  };
}
