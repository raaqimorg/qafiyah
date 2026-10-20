import { formatArabicCount, toArabicDigits } from '@/lib/arabic';
import { SITE_URL } from '@/lib/constants/config';
import { SITE_NAME_AR } from '@/lib/constants/site-meta';
import { VERSES_NOUN_FORMS } from '@/lib/constants/taxonomy-data';
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
import { sanitizeMetaText, withBrand, withPageNumber } from '@/lib/seo/meta-text';
import { feelingUrl, poemUrl } from '@/lib/urls';

import type { Feeling } from '@/lib/feelings/feeling-tree';
import type { FeelingVerseRow } from '@/lib/server/feelings';

const FEELINGS_CRUMB = 'المشاعر';
const INDEX_DESCRIPTION =
  'لا تعرف كيف تعبّر عن شعورك؟ اختر ما يقرب منه في شجرة المشاعر، واقرأ من الشعر ما يشبهه.';
const ERAS_SPAN = 'من الشعر الجاهلي إلى الأندلسي';
const ARCHIVE_TAIL = `ضمن أرشيف ${SITE_NAME_AR} الشامل للشعر العربي.`;

type FeelingSummary = Pick<Feeling, 'slug' | 'name' | 'verseCount'>;

export type FeelingsSeo<T> = {
  readonly title: string;
  readonly description: string;
  readonly jsonLd: readonly [CollectionPageDoc<T>, BreadcrumbListDoc];
};

const versesLabel = (count: number): string =>
  formatArabicCount({ count, nounForms: VERSES_NOUN_FORMS });

const indexCrumb: BreadcrumbItem = { name: FEELINGS_CRUMB, path: feelingUrl() };

export function buildFeelingsIndexSeo(
  feelings: readonly FeelingSummary[]
): FeelingsSeo<CollectionRef> {
  const collectionJsonLd = collectionPageNode({
    name: 'شجرة المشاعر',
    url: `${SITE_URL}${feelingUrl()}`,
    description: 'المشاعر في شجرة المشاعر، مع أبيات الشعر العربي التي تعبّر عن كل منها',
    isPartOf: websiteRef(),
    mainEntity: buildItemList(
      feelings.map((feeling) =>
        buildThingRef({
          '@type': 'Collection',
          name: feeling.name,
          url: `${SITE_URL}${feelingUrl(feeling.slug)}`,
          description: `أبيات في ${feeling.name} (${versesLabel(feeling.verseCount)})`,
        })
      )
    ),
  });
  return {
    title: withBrand('شجرة المشاعر في الشعر العربي'),
    description: INDEX_DESCRIPTION,
    jsonLd: [
      collectionJsonLd,
      buildBreadcrumbList([{ name: SITE_NAME_AR, path: '/' }, indexCrumb]),
    ],
  };
}

function knownPoetNames(verses: readonly FeelingVerseRow[], count: number): readonly string[] {
  const names = verses
    .filter((verse) => !verse.poet.isAnonymous)
    .map((verse) => sanitizeMetaText(verse.poet.name));
  return [...new Set(names)].slice(0, count);
}

function uniquePoems(verses: readonly FeelingVerseRow[]): readonly FeelingVerseRow['poem'][] {
  const bySlug = new Map(verses.map((verse) => [verse.poem.slug, verse.poem]));
  return [...bySlug.values()];
}

export function buildFeelingSeo(input: {
  readonly feeling: FeelingSummary;
  readonly parents: readonly FeelingSummary[];
  readonly verses: readonly FeelingVerseRow[];
  readonly page: number;
  readonly totalPages: number;
}): FeelingsSeo<CreativeWorkRef> {
  const { feeling, parents, verses, page, totalPages } = input;
  const lead = `أبيات في ${feeling.name}`;
  const count = versesLabel(feeling.verseCount);
  const poets = knownPoetNames(verses, 2);
  const poetsClause = poets.length === 0 ? '' : `، من شعرائها ${poets.join(' و')}`;

  const collectionJsonLd = collectionPageNode({
    name: lead,
    url: `${SITE_URL}${feelingUrl(feeling.slug, page)}`,
    description: `${lead} (${count}) - الصفحة ${toArabicDigits(page)} من ${toArabicDigits(totalPages)}`,
    isPartOf: websiteRef(),
    mainEntity: buildItemList(
      uniquePoems(verses).map((poem) =>
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
    indexCrumb,
    ...[...parents, feeling].map((step) => ({ name: step.name, path: feelingUrl(step.slug) })),
  ];

  return {
    title: withBrand(withPageNumber(lead, page)),
    description: `تصفح ${count} في ${feeling.name} ${ERAS_SPAN}${poetsClause}، ${ARCHIVE_TAIL}`,
    jsonLd: [collectionJsonLd, buildBreadcrumbList(crumbItems)],
  };
}
