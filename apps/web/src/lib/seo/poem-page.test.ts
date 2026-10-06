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
  poet: { name: 'المتنبي', slug: 'mtnb', hasAvatar: false },
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

  it('puts each entry on its own line, joining the parts of an entry of one or three parts without empty separators', () => {
    const poem = {
      ...basePoem,
      verses: [['أ', 'ب'], ['ج'], ['د', 'ه', 'و']],
      verseCount: 3,
    };
    const [article] = buildPoemLayout(poem, 'brda' as Parameters<typeof buildPoemLayout>[1]).jsonLd;
    expect(article.text).toBe('أ - ب\nج\nد - ه - و');
  });

  it('takes the opening from the first part of the first entry when that entry is a single line', () => {
    const poem = {
      ...basePoem,
      verses: [['سَطْرٌ'], ['صدر', 'عجز']],
      verseCount: 2,
    };
    const layout = buildPoemLayout(poem, 'brda' as Parameters<typeof buildPoemLayout>[1]);
    expect(layout.description).toContain('مطلعها: سطر.');
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

  it("shares the poet's avatar on both cards when the poet has one", () => {
    const poem = {
      ...basePoem,
      poet: { name: 'المتنبي', slug: 'mtnb', hasAvatar: true },
    } as Parameters<typeof buildPoemLayout>[0];
    const layout = buildPoemLayout(poem, 'brda' as Parameters<typeof buildPoemLayout>[1]);
    expect(layout.socialImage).toEqual({
      url: 'https://cdn.qafiyah.com/poets/mtnb/avatar.webp',
      alt: 'المتنبي',
      mimeType: 'image/webp',
    });
  });

  it('leaves both cards at the site defaults when the poet has no avatar', () => {
    const layout = buildPoemLayout(basePoem, 'brda' as Parameters<typeof buildPoemLayout>[1]);
    expect(layout.socialImage).toBeUndefined();
  });

  it('hides adjacent-poems entirely when the poet has only one poem', () => {
    const layout = buildPoemLayout(basePoem, 'brda' as Parameters<typeof buildPoemLayout>[1]);
    expect(layout.adjacentPoems.hidden).toBe(true);
  });
});
