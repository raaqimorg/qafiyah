import { formatArabicCount, toArabicDigits } from '@/lib/arabic';
import { SITE_URL } from '@/lib/constants/config';
import { SITE_NAME_AR, type SocialImage } from '@/lib/constants/site-meta';
import { POEMS_NOUN_FORMS, POETS_NOUN_FORMS } from '@/lib/constants/taxonomy-data';
import { derivePagination, type PaginationView } from '@/lib/pagination';
import {
  type BreadcrumbItem,
  buildBreadcrumbList,
  type BreadcrumbListDoc,
} from '@/lib/seo/json-ld/breadcrumb-list';
import { collectionPageNode, type CollectionPageDoc } from '@/lib/seo/json-ld/collection-page';
import { withContext } from '@/lib/seo/json-ld/document';
import { buildItemList } from '@/lib/seo/json-ld/item-list';
import { personNode, type PersonDoc } from '@/lib/seo/json-ld/person';
import { buildThingRef, type CreativeWorkRef, type PersonRef } from '@/lib/seo/json-ld/thing-ref';
import { websiteRef } from '@/lib/seo/json-ld/website';
import {
  excerptAtWordBoundary,
  sanitizeMetaText,
  UNKNOWN_ENTITY_NAME,
  withBrand,
} from '@/lib/seo/meta-text';
import { poetAvatarImage } from '@/lib/seo/social-images';
import { poemUrl, poetsUrl, poetUrl } from '@/lib/urls';

import type { Poet } from '@/lib/server/poets';
import type { Ok } from '@/lib/server/types';

type EraOption = Ok<'/eras'>['data'][number];
type PoetsListItem = Ok<'/poets'>['data'][number];
type PoetsPagination = Ok<'/poets'>['pagination'];
type PoemRow = Ok<'/poems'>['data'][number];

const POET_DESCRIPTION_TARGET_LENGTH = 160;
const BIO_PREVIEW_LENGTH = 300;
const BIO_TRUNCATION_FLOOR = 380;
const SUBTITLE_SEPARATOR = '·';

type ListCardItem = { readonly title: string; readonly subtitle: string; readonly href: string };

export type PoetsIndexView = {
  readonly title: string;
  readonly description: string;
  readonly isFiltered: boolean;
  readonly pag: PaginationView;
  readonly collectionJsonLd: CollectionPageDoc<PersonRef>;
  readonly crumbItems: readonly BreadcrumbItem[];
  readonly crumbsJsonLd: BreadcrumbListDoc;
  readonly heading: string;
  readonly emptyText: string;
  readonly items: readonly ListCardItem[];
};

export function buildPoetsIndexView(input: {
  readonly poets: readonly PoetsListItem[];
  readonly pagination: PoetsPagination;
  readonly activeEra: EraOption | undefined;
  readonly queryFilter: string | undefined;
  readonly eraParam: string | undefined;
}): PoetsIndexView {
  const { poets, pagination, activeEra, queryFilter, eraParam } = input;
  const pageNumber = pagination.page;
  const totalPoets = pagination.totalItems;
  const isFiltered = Boolean(activeEra ?? queryFilter);

  const pag = derivePagination(pagination, (p) =>
    poetsUrl({ page: p, era: eraParam, q: queryFilter })
  );

  const poetsLabel = formatArabicCount({ count: totalPoets, nounForms: POETS_NOUN_FORMS });

  let heading: string;
  if (activeEra) {
    heading = `شعراء العصر ال${activeEra.name} (${poetsLabel})`;
  } else if (queryFilter !== undefined && queryFilter !== '') {
    heading = `نتائج البحث عن ${queryFilter} (${poetsLabel})`;
  } else {
    heading = `جميع الشعراء (${poetsLabel})`;
  }

  const emptyText = isFiltered ? 'لا يوجد شعراء مطابقون' : 'لا يوجد المزيد من الشعراء';

  const title = withBrand('شعراء العرب ودواوينهم');
  const description = `صفحة الشعراء على ${SITE_NAME_AR}. تصفح دواوين الشعراء من العصر الجاهلي إلى المعاصر.`;

  const collectionJsonLd = collectionPageNode({
    name: 'قائمة الشعراء',
    url: `${SITE_URL}${poetsUrl({ page: pageNumber })}`,
    description: `قائمة بجميع الشعراء في موقع ${SITE_NAME_AR} - الصفحة ${toArabicDigits(pageNumber)} من ${toArabicDigits(pag.totalPages)}`,
    isPartOf: websiteRef(),
    mainEntity: buildItemList(
      poets.map((poet) =>
        buildThingRef({
          '@type': 'Person',
          name: poet.name,
          url: `${SITE_URL}${poetUrl(poet.slug)}`,
        })
      )
    ),
  });

  const crumbItems: readonly BreadcrumbItem[] = [
    { name: SITE_NAME_AR, path: '/' },
    { name: 'الشعراء', path: poetsUrl() },
  ];
  const crumbsJsonLd = buildBreadcrumbList(crumbItems);

  const items: readonly ListCardItem[] = poets.map((poet) => ({
    title: poet.name,
    subtitle: formatArabicCount({ count: poet.poemsCount, nounForms: POEMS_NOUN_FORMS }),
    href: poetUrl(poet.slug),
  }));

  return {
    title,
    description,
    isFiltered,
    pag,
    collectionJsonLd,
    crumbItems,
    crumbsJsonLd,
    heading,
    emptyText,
    items,
  };
}

type BioView =
  | { readonly kind: 'full'; readonly text: string }
  | { readonly kind: 'truncated'; readonly head: string; readonly rest: string };

export type PoetLayoutView = {
  readonly title: string;
  readonly description: string;
  readonly heading: string;
  readonly subtitle: string;
  readonly avatarUrl: string | undefined;
  readonly socialImage: SocialImage | undefined;
  readonly bio: BioView | undefined;
  readonly pag: PaginationView;
  readonly personJsonLd: PersonDoc;
  readonly worksJsonLd: ReturnType<typeof buildWorksJsonLd>;
  readonly crumbItems: readonly BreadcrumbItem[];
  readonly crumbsJsonLd: BreadcrumbListDoc;
  readonly items: readonly ListCardItem[];
};

function buildBioView(text: string): BioView {
  if (text.length <= BIO_TRUNCATION_FLOOR) return { kind: 'full', text };
  const head = excerptAtWordBoundary(text, BIO_PREVIEW_LENGTH);
  const rest = text.slice(head.length);
  return rest.trim() ? { kind: 'truncated', head, rest } : { kind: 'full', text };
}

function pickAdditiveNickname(poet: Poet): string | undefined {
  const nickname = poet.nickname?.trim();
  if (nickname === undefined || nickname === '') return undefined;
  return poet.name.includes(nickname) ? undefined : nickname;
}

function buildWorksJsonLd(poet: Poet, listedPoems: readonly PoemRow[]) {
  return withContext(
    buildItemList<CreativeWorkRef>(
      listedPoems.map((poem) =>
        buildThingRef({
          '@type': 'CreativeWork',
          name: poem.title,
          description: `قصيدة (${poem.title}) على ${poem.meter.name}`,
          url: `${SITE_URL}${poemUrl(poem.slug)}`,
        })
      ),
      { name: `قصائد ${poet.name}` }
    )
  );
}

export function buildPoetLayout(input: {
  readonly poet: Poet;
  readonly poems: readonly PoemRow[];
  readonly pagination: { readonly page: number; readonly totalPages: number };
}): PoetLayoutView {
  const { poet, poems, pagination } = input;
  const slug = poet.slug;
  const pag = derivePagination(pagination, (p) => poetUrl(slug, p));

  const poemsLabel = formatArabicCount({ count: poet.poemsCount, nounForms: POEMS_NOUN_FORMS });

  const title = withBrand(`ديوان ${poet.name}`);
  const knownEra = poet.era.name === UNKNOWN_ENTITY_NAME ? undefined : poet.era.name;
  const subtitle = [poemsLabel, knownEra, pickAdditiveNickname(poet)]
    .filter((part): part is string => part !== undefined)
    .join(` ${SUBTITLE_SEPARATOR} `);
  const socialImage = poetAvatarImage(poet);
  const avatarUrl = socialImage?.url;
  const rawBio = poet.bio?.trim();
  const bioText = rawBio === undefined || rawBio === '' ? undefined : rawBio;
  const bio = bioText === undefined ? undefined : buildBioView(bioText);
  const metaBio = bioText === undefined ? undefined : sanitizeMetaText(bioText);
  const eraClause = knownEra === undefined ? '' : `من العصر ال${knownEra}، `;
  const fallback = `شاعر ${eraClause}له ${poemsLabel}.`;
  const description = excerptAtWordBoundary(
    metaBio === undefined
      ? `ديوان ${poet.name} على ${SITE_NAME_AR}. ${fallback}`
      : `ديوان ${poet.name} على ${SITE_NAME_AR}. ${metaBio}`,
    POET_DESCRIPTION_TARGET_LENGTH
  );

  const personJsonLd = personNode({
    name: poet.name,
    url: `${SITE_URL}${poetUrl(slug)}`,
    description: `ديوان ${poet.name} (${poemsLabel})`,
    image: avatarUrl,
    mainEntityOfPage: buildThingRef({
      '@type': 'CollectionPage',
      name: `ديوان ${poet.name}`,
      url: `${SITE_URL}${poetUrl(slug)}`,
    }),
  });

  const listedPoems = poems.slice(0, 10);
  const worksJsonLd = buildWorksJsonLd(poet, listedPoems);

  const crumbItems: readonly BreadcrumbItem[] = [
    { name: SITE_NAME_AR, path: '/' },
    { name: 'الشعراء', path: poetsUrl() },
    { name: poet.name, path: poetUrl(slug) },
  ];
  const crumbsJsonLd = buildBreadcrumbList(crumbItems);

  const items: readonly ListCardItem[] = poems.map((poem) => ({
    title: poem.title,
    subtitle: poem.meter.name,
    href: poemUrl(poem.slug),
  }));

  return {
    title,
    description,
    heading: poet.name,
    subtitle,
    avatarUrl,
    socialImage,
    bio,
    pag,
    personJsonLd,
    worksJsonLd,
    crumbItems,
    crumbsJsonLd,
    items,
  };
}
