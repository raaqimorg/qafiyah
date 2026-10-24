import { describe, expect, it, vi } from 'vitest';

vi.mock('@/lib/server/client', () => ({ apiServer: {} }));

import { buildPoetLayout, buildPoetsIndexView } from './poets-page';

import type { PoemsList } from '@/lib/server/poems';

const basePoet = {
  name: 'إباء إسماعيل',
  slug: 'CCMr',
  poemsCount: 65,
  era: { name: 'معاصر', slug: 'muasir' },
} as unknown as Parameters<typeof buildPoetLayout>[0]['poet'];

function pageOf<T>(data: T[], cursors: { next?: string; prev?: string } = {}) {
  return { data, total: data.length, next: cursors.next ?? null, prev: cursors.prev ?? null };
}

const words = (count: number): string => Array.from({ length: count }, () => 'كلمة').join(' ');

describe('buildPoetLayout', () => {
  it('falls back to era and poem count when the poet has no bio', () => {
    const layout = buildPoetLayout({
      poet: basePoet,
      page: pageOf([]),
    });
    expect(layout.description).toBe(
      'ديوان إباء إسماعيل على قافية. شاعر من العصر المعاصر، له ٦٥ قصيدة.'
    );
  });

  it('names two of the poems when the poet has no bio', () => {
    const poems = [
      { title: 'أمل', slug: 'p1', meter: { name: 'الكامل' } },
      { title: 'غربة', slug: 'p2', meter: { name: 'الكامل' } },
      { title: 'وطن', slug: 'p3', meter: { name: 'الكامل' } },
    ] as unknown as PoemsList['data'];
    const layout = buildPoetLayout({
      poet: basePoet,
      page: pageOf(poems),
    });
    expect(layout.description).toBe(
      'ديوان إباء إسماعيل على قافية. شاعر من العصر المعاصر، له ٦٥ قصيدة، منها «أمل» و«غربة».'
    );
  });

  it('lists the poems after a colon when they are all the poet has', () => {
    const poet = { ...basePoet, poemsCount: 1 };
    const poems = [
      { title: 'أمل', slug: 'p1', meter: { name: 'الكامل' } },
    ] as unknown as PoemsList['data'];
    const layout = buildPoetLayout({ poet, page: pageOf(poems) });
    expect(layout.description).toBe(
      'ديوان إباء إسماعيل على قافية. شاعر من العصر المعاصر، له قصيدة: «أمل».'
    );
  });

  it('names fewer poems rather than cut a title when the description would run long', () => {
    const long = 'كلمة '.repeat(14).trim();
    const poems = [
      { title: long, slug: 'p1', meter: { name: 'الكامل' } },
      { title: long, slug: 'p2', meter: { name: 'الكامل' } },
      { title: long, slug: 'p3', meter: { name: 'الكامل' } },
    ] as unknown as PoemsList['data'];
    const layout = buildPoetLayout({
      poet: basePoet,
      page: pageOf(poems),
    });
    expect(layout.description).toBe(
      `ديوان إباء إسماعيل على قافية. شاعر من العصر المعاصر، له ٦٥ قصيدة، منها «${long}».`
    );
  });

  it('uses the bio when present', () => {
    const poet = { ...basePoet, bio: 'شاعرة سورية معاصرة.' };
    const layout = buildPoetLayout({ poet, page: pageOf([]) });
    expect(layout.description).toBe('ديوان إباء إسماعيل على قافية. شاعرة سورية معاصرة.');
  });

  it('returns the bio unsanitized for display while the meta description stays sanitized', () => {
    const poet = { ...basePoet, bio: 'شاعرة "سورية"  معاصرة.' };
    const layout = buildPoetLayout({ poet, page: pageOf([]) });
    expect(layout.bio).toEqual({ kind: 'full', text: 'شاعرة "سورية"  معاصرة.' });
    expect(layout.description).toBe('ديوان إباء إسماعيل على قافية. شاعرة سورية معاصرة.');
  });

  it('collapses a long bio and keeps its whole text', () => {
    const text = words(200);
    const layout = buildPoetLayout({
      poet: { ...basePoet, bio: text },
      page: pageOf([]),
    });
    expect(layout.bio).toEqual({ kind: 'collapsed', text });
  });

  it('shows a bio of up to 380 characters in full', () => {
    const text = words(70);
    expect(text.length).toBeGreaterThan(300);
    expect(text.length).toBeLessThanOrEqual(380);
    const layout = buildPoetLayout({
      poet: { ...basePoet, bio: text },
      page: pageOf([]),
    });
    expect(layout.bio).toEqual({ kind: 'full', text });
  });

  it('has no bio when the poet has none', () => {
    const layout = buildPoetLayout({
      poet: basePoet,
      page: pageOf([]),
    });
    expect(layout.bio).toBeUndefined();
  });

  it('keeps the poem filters in the pagination links but not in the canonical crumb', () => {
    const layout = buildPoetLayout({
      poet: basePoet,
      page: pageOf([], { next: 'n', prev: 'p' }),
      filters: { meter: ['altawil'], rhyme: [], theme: ['alnasib'] },
    });
    expect(layout.pag.nextUrl).toBe('/poets/CCMr?meter=altawil&theme=alnasib&cursor=n');
    expect(layout.pag.prevUrl).toBe('/poets/CCMr?meter=altawil&theme=alnasib&cursor=p');
    expect(layout.crumbItems.at(-1)?.path).toBe('/poets/CCMr');
  });

  it('gives every page of the poet the same title', () => {
    const layout = buildPoetLayout({ poet: basePoet, page: pageOf([], { prev: 'p' }) });
    expect(layout.title).toBe('ديوان إباء إسماعيل | قافية');
  });

  it('puts a nickname that adds something at the end of the subtitle', () => {
    const poet = { ...basePoet, name: 'المتنبي', nickname: 'أبو الطيب' };
    const layout = buildPoetLayout({ poet, page: pageOf([]) });
    expect(layout.subtitle).toBe('٦٥ قصيدة · معاصر · أبو الطيب');
  });

  it('carries the poem count in the subtitle, not the heading', () => {
    const layout = buildPoetLayout({
      poet: basePoet,
      page: pageOf([]),
    });
    expect(layout.heading).toBe('إباء إسماعيل');
    expect(layout.subtitle).toBe('٦٥ قصيدة · معاصر');
  });

  it('drops a nickname identical to the name from the subtitle', () => {
    const poet = { ...basePoet, name: 'أم النحيف', nickname: 'أم النحيف' };
    const layout = buildPoetLayout({ poet, page: pageOf([]) });
    expect(layout.subtitle).toBe('٦٥ قصيدة · معاصر');
  });

  it('drops a nickname contained in the name from the subtitle', () => {
    const poet = { ...basePoet, name: 'عبد الحسين الحياوي', nickname: 'الحياوي' };
    const layout = buildPoetLayout({ poet, page: pageOf([]) });
    expect(layout.subtitle).toBe('٦٥ قصيدة · معاصر');
  });

  it('builds an R2 avatar url from the slug when the poet has one', () => {
    const layout = buildPoetLayout({
      poet: { ...basePoet, hasAvatar: true },
      page: pageOf([]),
    });
    expect(layout.avatarUrl).toBe('https://cdn.qafiyah.com/poets/CCMr/avatar.webp');
  });

  it('has no avatar url when the poet has no avatar', () => {
    const layout = buildPoetLayout({
      poet: { ...basePoet, hasAvatar: false },
      page: pageOf([]),
    });
    expect(layout.avatarUrl).toBeUndefined();
  });

  it('puts the avatar on the Person node so the portrait binds to the entity', () => {
    const layout = buildPoetLayout({
      poet: { ...basePoet, hasAvatar: true },
      page: pageOf([]),
    });
    expect(layout.personJsonLd.image).toBe('https://cdn.qafiyah.com/poets/CCMr/avatar.webp');
  });

  it('omits Person.image entirely when the poet has no avatar', () => {
    const layout = buildPoetLayout({
      poet: { ...basePoet, hasAvatar: false },
      page: pageOf([]),
    });
    expect(layout.personJsonLd.image).toBeUndefined();
    expect(JSON.stringify(layout.personJsonLd)).not.toContain('image');
  });

  it('shares the avatar on both cards when the poet has one', () => {
    const layout = buildPoetLayout({
      poet: { ...basePoet, hasAvatar: true },
      page: pageOf([]),
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
      page: pageOf([]),
    });
    expect(layout.socialImage).toBeUndefined();
  });

  it('leaves the era out of the subtitle when it is unknown', () => {
    const poet = {
      ...basePoet,
      era: { name: 'غير معروف', slug: 'ghayrmaruf' },
      nickname: 'أبو الطيب',
    };
    const layout = buildPoetLayout({ poet, page: pageOf([]) });
    expect(layout.subtitle).toBe('٦٥ قصيدة · أبو الطيب');
  });

  it('drops the era clause instead of saying "unknown" era (regression: "العصر الغير معروف" is broken Arabic)', () => {
    const poet = { ...basePoet, era: { name: 'غير معروف', slug: 'ghayrmaruf' } };
    const layout = buildPoetLayout({ poet, page: pageOf([]) });
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
      page: pageOf([INDEX_POET]),
      cursor: undefined,
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
      page: pageOf([INDEX_POET]),
      cursor: undefined,
      activeEra: { name: 'العباسي', slug: 'abbasi', poemsCount: 10, poetsCount: 1 },
      queryFilter: undefined,
      eraParam: 'abbasi',
    });
    expect(view.heading).toContain('العباسي');
    expect(view.isFiltered).toBe(true);
  });

  it('switches the empty text between filtered and unfiltered', () => {
    const unfiltered = buildPoetsIndexView({
      page: pageOf([]),
      cursor: undefined,
      activeEra: undefined,
      queryFilter: undefined,
      eraParam: undefined,
    });
    expect(unfiltered.emptyText).toBe('لا يوجد المزيد من الشعراء');
  });
});
