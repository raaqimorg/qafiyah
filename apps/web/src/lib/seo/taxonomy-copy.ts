import { type ArabicNounForms, formatArabicCount } from '@/lib/arabic';
import { SITE_NAME_AR } from '@/lib/constants/site-meta';
import {
  COLLECTIONS_NOUN_FORMS,
  LETTERS_NOUN_FORMS,
  METERS_NOUN_FORMS,
  POEMS_NOUN_FORMS,
  THEMES_NOUN_FORMS,
} from '@/lib/constants/taxonomy-data';
import { UNKNOWN_ENTITY_NAME, withBrand } from '@/lib/seo/meta-text';
import { allCollections, getCollection } from '@/lib/server/collections';
import {
  allMeters,
  allRhymes,
  allThemes,
  getMeter,
  getRhyme,
  getTheme,
} from '@/lib/server/taxonomies';

import type { CollectionSlug, MeterSlug, RhymeSlug, ThemeSlug } from '@/lib/api/brands';
import type { PoemFilters } from '@/lib/server/poems';
import type { Ok } from '@/lib/server/types';
import type { TaxonomySection } from '@/lib/urls';

type PoemRow = Ok<'/poems'>['data'][number];

function distinctPoetNames(poems: readonly PoemRow[], count: number): readonly string[] {
  const seen = new Set<string>();
  for (const poem of poems) {
    if (poem.poet.isAnonymous) continue;
    seen.add(poem.poet.name);
    if (seen.size >= count) break;
  }
  return [...seen];
}

function firstAttributedPoem(poems: readonly PoemRow[]): PoemRow | undefined {
  return poems.find((poem) => !poem.poet.isAnonymous);
}

const ARCHIVE_TAIL = 'ضمن أرشيف قافية الشامل للشعر العربي.';

export type TermData = {
  readonly name: string;
  readonly slug: string;
  readonly poemsCount: number;
};
export type IndexTermData = TermData & { readonly poetsCount?: number };

export type TermPageConfig = {
  readonly get: (slug: string) => Promise<TermData | null>;
  readonly makeFilters: (slug: string) => PoemFilters;
  readonly crumbLabel: string;
  readonly titleLead: (name: string) => string;
  readonly description: (name: string, poemsLabel: string, poems: readonly PoemRow[]) => string;
  readonly jsonLdName: (name: string) => string;
  readonly jsonLdDescLead: (name: string) => string;
  readonly heading: (name: string, poemsLabel: string) => string;
  readonly emptyText: string;
  readonly secondary: (poem: PoemRow) => string;
};

// oxlint-disable typescript/no-unsafe-type-assertion -- the route validates each slug before the lookup builds its brand
export const TERM_PAGE_CONFIG: Record<TaxonomySection, TermPageConfig> = {
  meters: {
    get: (slug) => getMeter(slug as MeterSlug),
    makeFilters: (slug) => ({ meterSlugs: [slug as MeterSlug] }),
    crumbLabel: 'البحور',
    titleLead: (name) => `بحر ${name}: قصائده وشعراؤه`,
    description: (name, poemsLabel, poems) => {
      const poets = distinctPoetNames(poems, 2);
      const poetsPart = poets.length > 0 ? `، من شعرائه ${poets.join(' و')}` : '';
      return `تصفح ${poemsLabel} على بحر ${name} على ${SITE_NAME_AR}${poetsPart}، ${ARCHIVE_TAIL}`;
    },
    jsonLdName: (name) => `قصائد بحر ${name}`,
    jsonLdDescLead: (name) => `مجموعة قصائد على بحر ${name}`,
    heading: (name, poemsLabel) => `قصائد بحر ${name} (${poemsLabel})`,
    emptyText: 'لا توجد قصائد لهذا البحر.',
    secondary: (poem) => poem.poet.name,
  },
  rhymes: {
    get: (slug) => getRhyme(slug as RhymeSlug),
    makeFilters: (slug) => ({ rhymeSlugs: [slug as RhymeSlug] }),
    crumbLabel: 'القوافي',
    titleLead: (name) => `قصائد على روي ${name}`,
    description: (name, poemsLabel, poems) => {
      const sample = firstAttributedPoem(poems);
      const samplePart = sample ? `، منها «${sample.title}» نظمها ${sample.poet.name}` : '';
      return `تصفح ${poemsLabel} قافيتها ${name} على ${SITE_NAME_AR}${samplePart}، ${ARCHIVE_TAIL}`;
    },
    jsonLdName: (name) => `قصائد قافية ${name}`,
    jsonLdDescLead: (name) => `مجموعة قصائد على قافية ${name}`,
    heading: (name, poemsLabel) => `${name} (${poemsLabel})`,
    emptyText: 'لا توجد قصائد لهذه القافية.',
    secondary: (poem) => poem.meter.name,
  },
  themes: {
    get: (slug) => getTheme(slug as ThemeSlug),
    makeFilters: (slug) => ({ themeSlugs: [slug as ThemeSlug] }),
    crumbLabel: 'الأغراض',
    titleLead: (name) => `شعر ${name}`,
    description: (name, poemsLabel, poems) => {
      const poets = distinctPoetNames(poems, 2);
      const poetsPart = poets.length > 0 ? `، من شعرائه ${poets.join(' و')}` : '';
      return `تصفح ${poemsLabel} في غرض ${name} على ${SITE_NAME_AR}${poetsPart}، ${ARCHIVE_TAIL}`;
    },
    jsonLdName: (name) => `قصائد غرض ${name}`,
    jsonLdDescLead: (name) => `مجموعة قصائد من غرض ${name}`,
    heading: (name, poemsLabel) => `قصائد ${name} (${poemsLabel})`,
    emptyText: 'لا توجد قصائد لهذا الغرض.',
    secondary: (poem) => poem.poet.name,
  },
  collections: {
    get: (slug) => getCollection(slug as CollectionSlug),
    makeFilters: (slug) => ({ collectionSlugs: [slug as CollectionSlug] }),
    crumbLabel: 'الدواوين',
    titleLead: (name) => `ديوان ${name}`,
    description: (name, poemsLabel, poems) => {
      const sample = firstAttributedPoem(poems);
      const samplePart = sample ? `، منها «${sample.title}» نظمها ${sample.poet.name}` : '';
      return `تصفح ${poemsLabel} في ديوان ${name} على ${SITE_NAME_AR}${samplePart}، ${ARCHIVE_TAIL}`;
    },
    jsonLdName: (name) => `قصائد ديوان ${name}`,
    jsonLdDescLead: (name) => `مجموعة قصائد من ديوان ${name}`,
    heading: (name, poemsLabel) => `قصائد ${name} (${poemsLabel})`,
    emptyText: 'لا توجد قصائد في هذا الديوان.',
    secondary: (poem) => poem.poet.name,
  },
};

export type IndexPageConfig = {
  readonly all: () => Promise<readonly IndexTermData[]>;
  readonly filter?: (term: IndexTermData) => boolean;
  readonly metaTitle: string;
  readonly metaDescription: (terms: readonly IndexTermData[], termsLabel: string) => string;
  readonly jsonLdName: string;
  readonly jsonLdListDescription: (termsLabel: string) => string;
  readonly jsonLdItemDescription: (name: string, poemsLabel: string) => string;
  readonly crumbLabel: string;
  readonly nounForms: ArabicNounForms;
  readonly heading: (termsLabel: string) => string;
  readonly subtitle: (term: IndexTermData) => string;
};

export const INDEX_PAGE_CONFIG: Record<TaxonomySection, IndexPageConfig> = {
  meters: {
    all: allMeters,
    metaTitle: withBrand('بحور الشعر العربي'),
    metaDescription: (terms, termsLabel) => {
      const top = terms
        .filter((term) => term.name !== UNKNOWN_ENTITY_NAME)
        .sort((a, b) => b.poemsCount - a.poemsCount)
        .slice(0, 6)
        .map((term) => term.name)
        .join('، ');
      return `${termsLabel} في الشعر العربي، أبرزها ${top}.`;
    },
    jsonLdName: 'البحور الشعرية',
    jsonLdListDescription: (termsLabel) =>
      `قائمة بجميع البحور الشعرية في موقع ${SITE_NAME_AR} - ${termsLabel}`,
    jsonLdItemDescription: (name, poemsLabel) => `قصائد من بحر ${name} - ${poemsLabel}`,
    crumbLabel: 'البحور',
    nounForms: METERS_NOUN_FORMS,
    heading: (termsLabel) => `جميع البحور (${termsLabel})`,
    subtitle: (term) => formatArabicCount({ count: term.poemsCount, nounForms: POEMS_NOUN_FORMS }),
  },
  rhymes: {
    all: allRhymes,
    filter: (term) => term.poemsCount > 0,
    metaTitle: withBrand('قوافي الشعر العربي وحروف الروي'),
    metaDescription: (_terms, termsLabel) =>
      `قوافي الشعر العربي من الهمزة إلى الياء، ${termsLabel} موثق بعدد القصائد لكل حرف روي.`,
    jsonLdName: 'القوافي الشعرية',
    jsonLdListDescription: (termsLabel) =>
      `قائمة بجميع القوافي الشعرية في موقع ${SITE_NAME_AR} - ${termsLabel}`,
    jsonLdItemDescription: (name, poemsLabel) => `قصائد على قافية ${name} - ${poemsLabel}`,
    crumbLabel: 'القوافي',
    nounForms: LETTERS_NOUN_FORMS,
    heading: (termsLabel) => `جميع القوافي (${termsLabel})`,
    subtitle: (term) => formatArabicCount({ count: term.poemsCount, nounForms: POEMS_NOUN_FORMS }),
  },
  themes: {
    all: allThemes,
    metaTitle: withBrand('أغراض الشعر العربي'),
    metaDescription: (terms) =>
      `أغراض الشعر العربي: ${terms
        .filter((term) => term.name !== UNKNOWN_ENTITY_NAME)
        .map((term) => term.name)
        .join('، ')}.`,
    jsonLdName: 'أغراض الشعر',
    jsonLdListDescription: (termsLabel) =>
      `قائمة بجميع أغراض الشعر في موقع ${SITE_NAME_AR} - ${termsLabel}`,
    jsonLdItemDescription: (name, poemsLabel) => `قصائد من غرض ${name} - ${poemsLabel}`,
    crumbLabel: 'الأغراض',
    nounForms: THEMES_NOUN_FORMS,
    heading: (termsLabel) => `جميع الأغراض (${termsLabel})`,
    subtitle: (term) => formatArabicCount({ count: term.poemsCount, nounForms: POEMS_NOUN_FORMS }),
  },
  collections: {
    all: allCollections,
    metaTitle: withBrand('دواوين الشعر العربي وأمهات الكتب'),
    metaDescription: (terms) =>
      `دواوين ومختارات الشعر العربي على ${SITE_NAME_AR}: ${terms.map((term) => term.name).join('، ')}، موثقة بعدد القصائد لكل ديوان.`,
    jsonLdName: 'دواوين الشعر',
    jsonLdListDescription: (termsLabel) =>
      `قائمة بجميع دواوين الشعر في موقع ${SITE_NAME_AR} - ${termsLabel}`,
    jsonLdItemDescription: (name, poemsLabel) => `قصائد من ديوان ${name} - ${poemsLabel}`,
    crumbLabel: 'الدواوين',
    nounForms: COLLECTIONS_NOUN_FORMS,
    heading: (termsLabel) => `جميع الدواوين (${termsLabel})`,
    subtitle: (term) => formatArabicCount({ count: term.poemsCount, nounForms: POEMS_NOUN_FORMS }),
  },
};
