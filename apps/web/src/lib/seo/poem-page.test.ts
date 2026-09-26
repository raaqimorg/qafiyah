import { describe, expect, it } from 'vitest';

import { SITE_URL } from '@/lib/constants/config';

import { buildPoemLayout } from './poem-page';

const basePoem = {
  title: 'البردة',
  keywords: 'مدح',
  verses: [
    ['صَدْرٌ', 'عَجُزٌ'],
    ['صدر ثان', 'عجز ثان'],
  ],
  verseCount: 2,
  poet: { name: 'المتنبي', slug: 'mtnb' },
  era: { name: 'العباسي', slug: 'abbasi' },
  meter: { name: 'الطويل', slug: 'altawil' },
  rhyme: { name: 'الراء', slug: 'r' },
  theme: { name: 'المديح', slug: 'almadih' },
  recensions: [],
} as unknown as Parameters<typeof buildPoemLayout>[0];

describe('buildPoemLayout', () => {
  it('gives the poem article a publisher with both a url and a logo (regression: url used to be missing)', () => {
    const layout = buildPoemLayout(basePoem, 'brda' as Parameters<typeof buildPoemLayout>[1]);
    const [article] = layout.jsonLd;
    expect(article.publisher.url).toBeTruthy();
    expect(article.publisher.logo['@type']).toBe('ImageObject');
  });

  it('keywords are the poem facets, not every word of the poem', () => {
    const layout = buildPoemLayout(basePoem, 'brda' as Parameters<typeof buildPoemLayout>[1]);
    const [article] = layout.jsonLd;
    expect(article.keywords).toBe('المديح, الطويل, الراء, العباسي, المتنبي');
    expect(article.keywords).not.toContain('صدر');
  });

  it('puts the poem body in text, one verse per line', () => {
    const layout = buildPoemLayout(basePoem, 'brda' as Parameters<typeof buildPoemLayout>[1]);
    const [article] = layout.jsonLd;
    expect(article.text).toBe('صَدْرٌ - عَجُزٌ\nصدر ثان - عجز ثان');
  });

  it('gives the article the same human description as the meta tag, not the poem body', () => {
    const layout = buildPoemLayout(basePoem, 'brda' as Parameters<typeof buildPoemLayout>[1]);
    const [article] = layout.jsonLd;
    expect(article.description).toBe(layout.description);
    expect(article.description).not.toContain('عَجُزٌ');
  });

  it('drops unknown facets from keywords rather than listing them', () => {
    const poem = {
      ...basePoem,
      theme: { name: 'غير معروف', slug: 'ghayrmaruf' },
    };
    const layout = buildPoemLayout(poem, 'brda' as Parameters<typeof buildPoemLayout>[1]);
    const [article] = layout.jsonLd;
    expect(article.keywords).toBe('الطويل, الراء, العباسي, المتنبي');
  });

  it('points a recension canonical URL at its primary poem', () => {
    const poem = { ...basePoem, recensionOf: { slug: 'PRIM', title: 'الأصل' } };
    const layout = buildPoemLayout(poem, 'brda' as Parameters<typeof buildPoemLayout>[1]);
    expect(layout.canonical).toBe('/poems/PRIM');
  });

  it('gives a recension JSON-LD the same URL as its canonical', () => {
    const poem = { ...basePoem, recensionOf: { slug: 'PRIM', title: 'الأصل' } };
    const layout = buildPoemLayout(poem, 'brda' as Parameters<typeof buildPoemLayout>[1]);
    const [article, breadcrumbs] = layout.jsonLd;
    expect(article.url).toBe(`${SITE_URL}/poems/PRIM`);
    expect(breadcrumbs.itemListElement.at(-1)?.item).toBe(`${SITE_URL}/poems/PRIM`);
  });

  it('keeps a primary poem canonical to itself', () => {
    const layout = buildPoemLayout(basePoem, 'brda' as Parameters<typeof buildPoemLayout>[1]);
    expect(layout.canonical).toBe('/poems/brda');
  });

  it('drops an anonymous poet from keywords', () => {
    const poem = {
      ...basePoem,
      poet: { name: 'مجهول (عباسي)', slug: 'EaOH', hasAvatar: false, isAnonymous: true },
    };
    const layout = buildPoemLayout(poem, 'brda' as Parameters<typeof buildPoemLayout>[1]);
    const [article] = layout.jsonLd;
    expect(article.keywords).toBe('المديح, الطويل, الراء, العباسي');
  });

  it('titles a muallaqa by its traditional name in search while share cards keep the poem title', () => {
    const poem = {
      ...basePoem,
      title: 'قفا نبك من ذكرى حبيب ومنزل',
      poet: { name: 'امرؤ القيس', slug: 'imru' },
    } as Parameters<typeof buildPoemLayout>[0];
    const layout = buildPoemLayout(poem, 'rHUD' as Parameters<typeof buildPoemLayout>[1]);
    expect(layout.title).toBe('معلقة امرئ القيس | قافية');
    expect(layout.ogTitle).toBe('قفا نبك من ذكرى حبيب ومنزل - امرؤ القيس | قافية');
    expect(layout.twitterTitle).toBe(layout.ogTitle);
  });

  it('ends the title with the brand suffix', () => {
    const layout = buildPoemLayout(basePoem, 'brda' as Parameters<typeof buildPoemLayout>[1]);
    expect(layout.title).toBe('البردة - المتنبي | قافية');
  });

  it('describes the poem with meter, rhyme, verse count, and a tashkeel-free opening', () => {
    const layout = buildPoemLayout(basePoem, 'brda' as Parameters<typeof buildPoemLayout>[1]);
    expect(layout.description).toContain('الطويل');
    expect(layout.description).toContain('الراء');
    expect(layout.description).toContain('مطلعها: صدر.');
  });

  it('keeps og and twitter description in sync (regression: they used to drift)', () => {
    const layout = buildPoemLayout(basePoem, 'brda' as Parameters<typeof buildPoemLayout>[1]);
    expect(layout.twitterDescription).toBe(layout.ogDescription);
    expect(layout.twitterTitle).toBe(layout.ogTitle);
  });

  it('trails the poet by default and keeps the canonical URL bare', () => {
    const layout = buildPoemLayout(basePoem, 'brda' as Parameters<typeof buildPoemLayout>[1]);
    expect(layout.crumbItems.map((item) => item.name)).toEqual([
      'قافية',
      'الشعراء',
      'المتنبي',
      'البردة',
    ]);
    expect(layout.canonical).toBe('/poems/brda');
  });

  it('trails the theme when opened from a theme listing, with next and previous kept in it', () => {
    const poem = {
      ...basePoem,
      prev: { title: 'قصيدة سابقة', slug: 'UmlG' },
      next: { title: 'قصيدة تالية', slug: 'SOeo' },
    };
    const layout = buildPoemLayout(poem, 'brda' as Parameters<typeof buildPoemLayout>[1], 'themes');
    expect(layout.crumbItems).toEqual([
      { name: 'قافية', path: '/' },
      { name: 'الأغراض', path: '/themes' },
      { name: 'المديح', path: '/themes/almadih' },
      { name: 'البردة', path: '/poems/brda' },
    ]);
    expect(layout.adjacentPoems.nextHref).toBe('/poems/SOeo?from=themes');
    expect(layout.adjacentPoems.prevHref).toBe('/poems/UmlG?from=themes');
    expect(layout.canonical).toBe('/poems/brda');
  });

  it('falls back to the poet when opened from a collection the poem is not in', () => {
    const layout = buildPoemLayout(
      basePoem,
      'brda' as Parameters<typeof buildPoemLayout>[1],
      'collections'
    );
    expect(layout.crumbItems[1]).toEqual({ name: 'الشعراء', path: '/poets' });
    expect(layout.adjacentPoems.label).toBe('تصفح قصائد الشاعر');
  });

  it('trails the collection when the poem is in one', () => {
    const poem = { ...basePoem, collection: { name: 'المعلقات', slug: 'almuallaqat' } };
    const layout = buildPoemLayout(
      poem,
      'brda' as Parameters<typeof buildPoemLayout>[1],
      'collections'
    );
    expect(layout.crumbItems[1]).toEqual({ name: 'الدواوين', path: '/collections' });
    expect(layout.crumbItems[2]).toEqual({ name: 'المعلقات', path: '/collections/almuallaqat' });
  });

  it('hides adjacent-poems entirely when the poet has only one poem', () => {
    const layout = buildPoemLayout(basePoem, 'brda' as Parameters<typeof buildPoemLayout>[1]);
    expect(layout.adjacentPoems.hidden).toBe(true);
  });
});
