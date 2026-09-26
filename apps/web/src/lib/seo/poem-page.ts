import { deriveAdjacentPoems, type AdjacentPoemsView } from '@/lib/adjacent-poems';
import { formatArabicCount, stripTashkeel } from '@/lib/arabic';
import { SITE_URL } from '@/lib/constants/config';
import { POEM_LANGUAGE, SITE_NAME_AR } from '@/lib/constants/site-meta';
import { VERSES_NOUN_FORMS } from '@/lib/constants/taxonomy-data';
import { POEM_CONTEXTS, resolvePoemContext } from '@/lib/poem-context';
import {
  type BreadcrumbItem,
  buildBreadcrumbList,
  type BreadcrumbListDoc,
} from '@/lib/seo/json-ld/breadcrumb-list';
import { buildPoemArticle, type PoemArticleDoc } from '@/lib/seo/json-ld/poem-article';
import { buildThingRef } from '@/lib/seo/json-ld/thing-ref';
import { websiteRef } from '@/lib/seo/json-ld/website';
import {
  sanitizeMetaText,
  truncateMetaText,
  UNKNOWN_ENTITY_NAME,
  withBrand,
} from '@/lib/seo/meta-text';
import {
  poemUrl,
  poetsUrl,
  poetUrl,
  type TaxonomySection,
  taxonomyIndexUrl,
  taxonomyUrl,
} from '@/lib/urls';

import type { PoemSlug } from '@/lib/api/brands';
import type { Poem } from '@/lib/api/result-types';

const HEMISTICH_SEPARATOR = ' - ';
const VERSE_SEPARATOR = '\n';
const KEYWORD_SEPARATOR = ', ';

const MUALLAQA_SEARCH_TITLES: Readonly<Record<string, string>> = {
  rHUD: 'معلقة امرئ القيس',
  xjIC: 'معلقة طرفة بن العبد',
  gnNg: 'معلقة زهير بن أبي سلمى',
  DTyF: 'معلقة لبيد بن ربيعة',
  YksA: 'معلقة عمرو بن كلثوم',
  iaqM: 'معلقة عنترة بن شداد',
  xxWN: 'معلقة الحارث بن حلزة',
  JnuW: 'معلقة الأعشى',
  PDyA: 'معلقة النابغة الذبياني',
  oMec: 'معلقة عبيد بن الأبرص',
};

type PoemLayoutProps = {
  readonly title: string;
  readonly description: string;
  readonly canonical: string;
  readonly ogTitle: string;
  readonly ogDescription: string;
  readonly twitterTitle: string;
  readonly twitterDescription: string;
  readonly articleAuthor: string;
  readonly articleSection: string;
  readonly crumbItems: readonly BreadcrumbItem[];
  readonly adjacentPoems: AdjacentPoemsView;
  readonly jsonLd: readonly [PoemArticleDoc, BreadcrumbListDoc];
};

function buildCrumbItems(
  poem: Poem,
  slug: string,
  context: TaxonomySection | undefined
): readonly BreadcrumbItem[] {
  const term = context === undefined ? undefined : POEM_CONTEXTS[context].term(poem);
  const parents: readonly BreadcrumbItem[] =
    context === undefined || term === undefined
      ? [
          { name: 'الشعراء', path: poetsUrl() },
          { name: poem.poet.name, path: poetUrl(poem.poet.slug) },
        ]
      : [
          { name: POEM_CONTEXTS[context].crumbLabel, path: taxonomyIndexUrl(context) },
          { name: term.name, path: taxonomyUrl(context, term.slug) },
        ];
  return [{ name: SITE_NAME_AR, path: '/' }, ...parents, { name: poem.title, path: poemUrl(slug) }];
}

function buildPoemText(poem: Poem): string {
  return poem.verses.map((verse) => verse.join(HEMISTICH_SEPARATOR)).join(VERSE_SEPARATOR);
}

function buildPoemKeywords(poem: Poem): string {
  const poetName = poem.poet.isAnonymous ? [] : [poem.poet.name];
  return [poem.theme.name, poem.meter.name, poem.rhyme.name, poem.era.name, ...poetName]
    .map((term) => sanitizeMetaText(term))
    .filter((term) => term.length > 0 && term !== UNKNOWN_ENTITY_NAME)
    .join(KEYWORD_SEPARATOR);
}

function buildJsonLd(
  poem: Poem,
  pageUrl: string,
  description: string,
  crumbItems: readonly BreadcrumbItem[]
): readonly [PoemArticleDoc, BreadcrumbListDoc] {
  const poetHref = `${SITE_URL}${poetUrl(poem.poet.slug)}`;
  const eraHref = `${SITE_URL}${poetsUrl({ era: poem.era.slug })}`;
  const displayTitle = poem.title;
  const poetName = poem.poet.name;
  const article = buildPoemArticle({
    name: sanitizeMetaText(displayTitle),
    headline: sanitizeMetaText(`${displayTitle} - ${poetName}`),
    author: buildThingRef({ '@type': 'Person', name: poetName, url: poetHref }),
    inLanguage: POEM_LANGUAGE,
    url: pageUrl,
    isPartOf: [
      websiteRef(),
      buildThingRef({ '@type': 'Collection', name: poetName, url: poetHref }),
      buildThingRef({ '@type': 'Collection', name: poem.era.name, url: eraHref }),
    ],
    description,
    text: buildPoemText(poem),
    keywords: buildPoemKeywords(poem),
  });
  return [article, buildBreadcrumbList(crumbItems)];
}

export function buildPoemLayout(
  poem: Poem,
  slug: PoemSlug,
  from?: TaxonomySection
): PoemLayoutProps {
  const context = resolvePoemContext(poem, from);
  const displayTitle = poem.title;
  const poetName = poem.poet.name;
  const versesLabel = formatArabicCount({ count: poem.verseCount, nounForms: VERSES_NOUN_FORMS });
  const opening = stripTashkeel(poem.verses[0]?.[0] ?? '');
  const structuralPart = `قصيدة ${displayTitle} لـ${poetName}، ${versesLabel} على بحر ${poem.meter.name} وروي ${poem.rhyme.name}.`;
  const description = truncateMetaText(
    sanitizeMetaText(opening === '' ? structuralPart : `${structuralPart} مطلعها: ${opening}.`)
  );
  const pageTitle = withBrand(`${sanitizeMetaText(displayTitle)} - ${sanitizeMetaText(poetName)}`);
  const muallaqaTitle = MUALLAQA_SEARCH_TITLES[slug];
  const canonicalSlug = poem.recensionOf?.slug ?? slug;
  const pageUrl = `${SITE_URL}${poemUrl(canonicalSlug)}`;
  const crumbItems = buildCrumbItems(poem, canonicalSlug, context);
  const adjacentPoems = deriveAdjacentPoems(poem, context);
  return {
    title: muallaqaTitle === undefined ? pageTitle : withBrand(muallaqaTitle),
    description,
    canonical: poemUrl(canonicalSlug),
    ogTitle: pageTitle,
    ogDescription: description,
    twitterTitle: pageTitle,
    twitterDescription: description,
    articleAuthor: poetUrl(poem.poet.slug),
    articleSection: poem.era.name,
    crumbItems,
    adjacentPoems,
    jsonLd: buildJsonLd(poem, pageUrl, description, crumbItems),
  };
}
