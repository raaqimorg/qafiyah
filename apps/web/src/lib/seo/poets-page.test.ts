import { describe, expect, it, vi } from 'vitest';

vi.mock('@/lib/server/client', () => ({ apiServer: {} }));

import { buildPoetLayout, buildPoetsIndexView } from './poets-page';

const basePoet = {
  name: 'إباء إسماعيل',
  slug: 'CCMr',
  poemsCount: 65,
  era: { name: 'معاصر', slug: 'muasir' },
} as unknown as Parameters<typeof buildPoetLayout>[0]['poet'];

const words = (count: number): string => Array.from({ length: count }, () => 'كلمة').join(' ');

describe('buildPoetLayout', () => {
  it('falls back to era and poem count when the poet has no bio', () => {
    const layout = buildPoetLayout({
      poet: basePoet,
      poems: [],
      pagination: { page: 1, totalPages: 1 },
    });
    expect(layout.description).toBe(
      'ديوان إباء إسماعيل على قافية. شاعر من العصر المعاصر، له ٦٥ قصيدة.'
    );
  });

  it('uses the bio when present', () => {
    const poet = { ...basePoet, bio: 'شاعرة سورية معاصرة.' };
    const layout = buildPoetLayout({ poet, poems: [], pagination: { page: 1, totalPages: 1 } });
    expect(layout.description).toBe('ديوان إباء إسماعيل على قافية. شاعرة سورية معاصرة.');
  });

  it('returns the bio unsanitized for display while the meta description stays sanitized', () => {
    const poet = { ...basePoet, bio: 'شاعرة "سورية"  معاصرة.' };
    const layout = buildPoetLayout({ poet, poems: [], pagination: { page: 1, totalPages: 1 } });
    expect(layout.bio).toEqual({ kind: 'full', text: 'شاعرة "سورية"  معاصرة.' });
    expect(layout.description).toBe('ديوان إباء إسماعيل على قافية. شاعرة سورية معاصرة.');
  });

  it('splits a long bio into a preview and the rest, cutting on a word boundary', () => {
    const text = words(200);
    const layout = buildPoetLayout({
      poet: { ...basePoet, bio: text },
      poems: [],
      pagination: { page: 1, totalPages: 1 },
    });
    const bio = layout.bio;
    if (bio?.kind !== 'truncated') throw new Error(`expected a truncated bio, got ${bio?.kind}`);
    expect(bio.head.length).toBeLessThanOrEqual(300);
    expect(bio.head + bio.rest).toBe(text);
    expect(bio.head.endsWith(' ')).toBe(false);
    expect(bio.rest.startsWith(' ')).toBe(true);
  });

  it('keeps a bio just past the preview length whole rather than hiding a few words', () => {
    const text = words(70);
    expect(text.length).toBeGreaterThan(300);
    const layout = buildPoetLayout({
      poet: { ...basePoet, bio: text },
      poems: [],
      pagination: { page: 1, totalPages: 1 },
    });
    expect(layout.bio).toEqual({ kind: 'full', text });
  });

  it('has no bio when the poet has none', () => {
    const layout = buildPoetLayout({
      poet: basePoet,
      poems: [],
      pagination: { page: 1, totalPages: 1 },
    });
    expect(layout.bio).toBeUndefined();
  });

  it('keeps the poem filters in the pagination links but not in the canonical crumb', () => {
    const layout = buildPoetLayout({
      poet: basePoet,
      poems: [],
      pagination: { page: 2, totalPages: 3 },
      filters: { meter: ['altawil'], rhyme: [], theme: ['alnasib'] },
    });
    expect(layout.pag.nextPageUrl).toBe('/poets/CCMr?meter=altawil&theme=alnasib&page=3');
    expect(layout.pag.prevPageUrl).toBe('/poets/CCMr?meter=altawil&theme=alnasib');
    expect(layout.crumbItems.at(-1)?.path).toBe('/poets/CCMr');
  });

  it('puts a nickname that adds something at the end of the subtitle', () => {
    const poet = { ...basePoet, name: 'المتنبي', nickname: 'أبو الطيب' };
    const layout = buildPoetLayout({ poet, poems: [], pagination: { page: 1, totalPages: 1 } });
    expect(layout.subtitle).toBe('٦٥ قصيدة · معاصر · أبو الطيب');
  });

  it('carries the poem count in the subtitle, not the heading', () => {
    const layout = buildPoetLayout({
      poet: basePoet,
      poems: [],
      pagination: { page: 1, totalPages: 1 },
    });
    expect(layout.heading).toBe('إباء إسماعيل');
    expect(layout.subtitle).toBe('٦٥ قصيدة · معاصر');
  });

  it('drops a nickname identical to the name from the subtitle', () => {
    const poet = { ...basePoet, name: 'أم النحيف', nickname: 'أم النحيف' };
    const layout = buildPoetLayout({ poet, poems: [], pagination: { page: 1, totalPages: 1 } });
    expect(layout.subtitle).toBe('٦٥ قصيدة · معاصر');
  });

  it('drops a nickname contained in the name from the subtitle', () => {
    const poet = { ...basePoet, name: 'عبد الحسين الحياوي', nickname: 'الحياوي' };
    const layout = buildPoetLayout({ poet, poems: [], pagination: { page: 1, totalPages: 1 } });
    expect(layout.subtitle).toBe('٦٥ قصيدة · معاصر');
  });

  it('builds an R2 avatar url from the slug when the poet has one', () => {
    const layout = buildPoetLayout({
      poet: { ...basePoet, hasAvatar: true },
      poems: [],
      pagination: { page: 1, totalPages: 1 },
    });
    expect(layout.avatarUrl).toBe('https://cdn.qafiyah.com/poets/CCMr/avatar.webp');
  });

  it('has no avatar url when the poet has no avatar', () => {
    const layout = buildPoetLayout({
      poet: { ...basePoet, hasAvatar: false },
      poems: [],
      pagination: { page: 1, totalPages: 1 },
    });
    expect(layout.avatarUrl).toBeUndefined();
  });

  it('puts the avatar on the Person node so the portrait binds to the entity', () => {
    const layout = buildPoetLayout({
      poet: { ...basePoet, hasAvatar: true },
      poems: [],
      pagination: { page: 1, totalPages: 1 },
    });
    expect(layout.personJsonLd.image).toBe('https://cdn.qafiyah.com/poets/CCMr/avatar.webp');
  });

  it('omits Person.image entirely when the poet has no avatar', () => {
    const layout = buildPoetLayout({
      poet: { ...basePoet, hasAvatar: false },
      poems: [],
      pagination: { page: 1, totalPages: 1 },
    });
    expect(layout.personJsonLd.image).toBeUndefined();
    expect(JSON.stringify(layout.personJsonLd)).not.toContain('image');
  });

  it('shares the avatar on both cards when the poet has one', () => {
    const layout = buildPoetLayout({
      poet: { ...basePoet, hasAvatar: true },
      poems: [],
      pagination: { page: 1, totalPages: 1 },
    });
    expect(layout.socialImage).toEqual({
      url: 'https://cdn.qafiyah.com/poets/CCMr/avatar.webp',
      alt: basePoet.name,
      mimeType: 'image/webp',
    });
  });

  it('leaves both cards at the site defaults when the poet has no avatar', () => {
    const layout = buildPoetLayout({
      poet: { ...basePoet, hasAvatar: false },
      poems: [],
      pagination: { page: 1, totalPages: 1 },
    });
    expect(layout.socialImage).toBeUndefined();
  });

  it('leaves the era out of the subtitle when it is unknown', () => {
    const poet = {
      ...basePoet,
      era: { name: 'غير معروف', slug: 'ghayrmaruf' },
      nickname: 'أبو الطيب',
    };
    const layout = buildPoetLayout({ poet, poems: [], pagination: { page: 1, totalPages: 1 } });
    expect(layout.subtitle).toBe('٦٥ قصيدة · أبو الطيب');
  });

  it('drops the era clause instead of saying "unknown" era (regression: "العصر الغير معروف" is broken Arabic)', () => {
    const poet = { ...basePoet, era: { name: 'غير معروف', slug: 'ghayrmaruf' } };
    const layout = buildPoetLayout({ poet, poems: [], pagination: { page: 1, totalPages: 1 } });
    expect(layout.description).toBe('ديوان إباء إسماعيل على قافية. شاعر له ٦٥ قصيدة.');
  });
});

const INDEX_POET = {
  name: 'المتنبي',
  slug: 'mtnb',
  poemsCount: 12,
};

describe('buildPoetsIndexView', () => {
  it('titles the unfiltered list and counts all poets', () => {
    const view = buildPoetsIndexView({
      poets: [INDEX_POET],
      pagination: { page: 1, pageSize: 30, totalPages: 1, totalItems: 1 },
      activeEra: undefined,
      queryFilter: undefined,
      eraParam: undefined,
    });
    expect(view.heading).toContain('جميع الشعراء');
    expect(view.isFiltered).toBe(false);
    expect(view.items[0]?.title).toBe('المتنبي');
    expect(view.items[0]?.href).toBe('/poets/mtnb');
  });

  it('titles an era filter by that era and marks it filtered', () => {
    const view = buildPoetsIndexView({
      poets: [INDEX_POET],
      pagination: { page: 1, pageSize: 30, totalPages: 1, totalItems: 1 },
      activeEra: { name: 'العباسي', slug: 'abbasi', poemsCount: 10, poetsCount: 1 },
      queryFilter: undefined,
      eraParam: 'abbasi',
    });
    expect(view.heading).toContain('العباسي');
    expect(view.isFiltered).toBe(true);
  });

  it('switches the empty text between filtered and unfiltered', () => {
    const unfiltered = buildPoetsIndexView({
      poets: [],
      pagination: { page: 1, pageSize: 30, totalPages: 1, totalItems: 0 },
      activeEra: undefined,
      queryFilter: undefined,
      eraParam: undefined,
    });
    expect(unfiltered.emptyText).toBe('لا يوجد المزيد من الشعراء');
  });
});
