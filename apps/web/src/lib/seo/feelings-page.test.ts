import { describe, expect, it } from 'vitest';

import { SITE_URL } from '@/lib/constants/config';

import { buildFeelingSeo, buildFeelingsIndexSeo } from './feelings-page';

const sadness = { slug: 'alhuzn', name: 'الحزن', verseCount: 48857, children: [] };
const grief = { slug: 'alasa', name: 'الأسى', verseCount: 120, children: [] };

type VerseRow = Parameters<typeof buildFeelingSeo>[0]['verses'][number];

const verse = (poem: string, poet: string, isAnonymous = false): VerseRow =>
  ({
    poem: { title: `قصيدة ${poem}`, slug: poem },
    poet: { name: poet, slug: `${poet}-slug`, isAnonymous },
  }) as unknown as VerseRow;

describe('buildFeelingsIndexSeo', () => {
  it('lists every feeling in the collection page, with a two-step breadcrumb', () => {
    const seo = buildFeelingsIndexSeo([sadness, grief]);
    const [page, crumbs] = seo.jsonLd;
    expect(seo.title).toBe('شجرة المشاعر في الشعر العربي | قافية');
    expect(page.mainEntity.itemListElement.map((entry) => entry.item.url)).toEqual([
      `${SITE_URL}/feelings/alhuzn`,
      `${SITE_URL}/feelings/alasa`,
    ]);
    expect(page.mainEntity.itemListElement[0]?.item.description).toBe(
      'أبيات في الحزن (٤٨٬٨٥٧ بيتا)'
    );
    expect(crumbs.itemListElement.map((entry) => entry.item)).toEqual([
      `${SITE_URL}/`,
      `${SITE_URL}/feelings`,
    ]);
  });
});

describe('buildFeelingSeo', () => {
  it('names two poets from the page in the description', () => {
    const seo = buildFeelingSeo({
      feeling: sadness,
      parents: [],
      verses: [verse('aaaa', 'ابن الرومي'), verse('bbbb', 'المتنبي'), verse('cccc', 'البحتري')],
      page: 1,
      totalPages: 2,
    });
    expect(seo.description).toBe(
      'تصفح ٤٨٬٨٥٧ بيتا في الحزن من الشعر الجاهلي إلى الأندلسي، من شعرائها ابن الرومي والمتنبي، ضمن أرشيف قافية الشامل للشعر العربي.'
    );
  });

  it('skips anonymous poets, and drops the poets clause when none are known', () => {
    const seo = buildFeelingSeo({
      feeling: grief,
      parents: [],
      verses: [verse('aaaa', 'غير معروف', true)],
      page: 1,
      totalPages: 1,
    });
    expect(seo.description).toBe(
      'تصفح ١٢٠ بيتا في الأسى من الشعر الجاهلي إلى الأندلسي، ضمن أرشيف قافية الشامل للشعر العربي.'
    );
  });

  it('lists each poem of the page once, and walks the tree in the breadcrumb', () => {
    const seo = buildFeelingSeo({
      feeling: grief,
      parents: [sadness],
      verses: [verse('aaaa', 'ابن الرومي'), verse('aaaa', 'ابن الرومي'), verse('bbbb', 'المتنبي')],
      page: 2,
      totalPages: 3,
    });
    const [page, crumbs] = seo.jsonLd;
    expect(seo.title).toBe('أبيات في الأسى، الصفحة ٢ | قافية');
    expect(page.url).toBe(`${SITE_URL}/feelings/alasa?page=2`);
    expect(page.mainEntity.itemListElement.map((entry) => entry.item.url)).toEqual([
      `${SITE_URL}/poems/aaaa`,
      `${SITE_URL}/poems/bbbb`,
    ]);
    expect(crumbs.itemListElement.map((entry) => entry.name)).toEqual([
      'قافية',
      'المشاعر',
      'الحزن',
      'الأسى',
    ]);
  });
});
