import { describe, expect, it, vi } from 'vitest';

vi.mock('@/lib/server/client', () => ({ apiServer: {} }));

import {
  buildTaxonomyIndexView,
  buildTaxonomyTermView,
  type TaxonomyTermLoad,
} from './taxonomy-pages';

const poems = [
  {
    title: 'البردة',
    slug: 'p1',
    poet: { name: 'المتنبي', slug: 'mutanabbi' },
    meter: { name: 'الكامل', slug: 'alkamil' },
  },
] as unknown as TaxonomyTermLoad['poems'];

describe('buildTaxonomyTermView', () => {
  it('renders the meters term page strings, with the poet as the card subtitle', () => {
    const { layout, body } = buildTaxonomyTermView('meters', {
      term: { name: 'الطويل', slug: 'altaweel', poemsCount: 12 },
      poems,
      pagination: { page: 1, totalPages: 3 },
    });

    expect(layout.title).toBe('بحر الطويل: قصائده وشعراؤه | قافية');
    expect(layout.description).toBe(
      'تصفح ١٢ قصيدة على بحر الطويل على قافية، لـالمتنبي، ضمن أرشيف قافية الشامل للشعر العربي.'
    );
    expect(layout.canonical).toBe('/meters/altaweel');
    expect((layout.jsonLd[0] as { name: string }).name).toBe('قصائد بحر الطويل');
    expect(body.heading).toBe('قصائد بحر الطويل (١٢ قصيدة)');
    expect(body.items[0]?.subtitle).toBe('المتنبي');
    expect(body.pagination.hasPrevPage).toBe(false);
    expect(body.pagination.hasNextPage).toBe(true);
  });

  it('renders the rhymes heading without a prefix and uses the meter as the card subtitle', () => {
    const { body, layout } = buildTaxonomyTermView('rhymes', {
      term: { name: 'الراء', slug: 'r', poemsCount: 5 },
      poems,
      pagination: { page: 2, totalPages: 4 },
    });

    expect(body.heading).toBe('الراء (٥ قصائد)');
    expect(body.items[0]?.subtitle).toBe('الكامل');
    expect(layout.canonical).toBe('/rhymes/r?page=2');
  });

  it('never surfaces an anonymous poet as a sample poet (regression: the unknown poet has the most poems of any poet)', () => {
    const poemsWithUnknownPoet = [
      {
        title: 'قصيدة أولى',
        slug: 'p1',
        poet: { name: 'غير معروف', slug: 'JJHE', isAnonymous: true },
        meter: {},
      },
      {
        title: 'قصيدة ثانية',
        slug: 'p2',
        poet: { name: 'مجهول (عباسي)', slug: 'EaOH', isAnonymous: true },
        meter: {},
      },
      {
        title: 'قصيدة ثالثة',
        slug: 'p3',
        poet: { name: 'المتنبي', slug: 'mutanabbi', isAnonymous: false },
        meter: {},
      },
    ] as unknown as TaxonomyTermLoad['poems'];

    const { layout } = buildTaxonomyTermView('meters', {
      term: { name: 'الطويل', slug: 'altaweel', poemsCount: 2 },
      poems: poemsWithUnknownPoet,
      pagination: { page: 1, totalPages: 1 },
    });

    expect(layout.description).not.toContain('غير معروف');
    expect(layout.description).not.toContain('مجهول');
    expect(layout.description).toContain('المتنبي');
  });

  it.each(['meters', 'rhymes', 'themes', 'collections'] as const)(
    '%s term cards open the poem in that listing while the structured data keeps the bare URL',
    (section) => {
      const { layout, body } = buildTaxonomyTermView(section, {
        term: { name: 'ب', slug: 'x', poemsCount: 1 },
        poems,
        pagination: { page: 1, totalPages: 1 },
      });
      expect(body.items[0]?.href).toBe(`/poems/p1?from=${section}`);
      expect(JSON.stringify(layout.jsonLd[0])).toContain('/poems/p1"');
    }
  );

  it.each(['meters', 'rhymes', 'themes', 'collections'] as const)(
    '%s term description clears the 60-char SEO minimum even with the shortest name, a single poem, and no sample poet',
    (section) => {
      const { layout } = buildTaxonomyTermView(section, {
        term: { name: 'ب', slug: 'x', poemsCount: 1 },
        poems: [],
        pagination: { page: 1, totalPages: 1 },
      });
      expect(layout.description.length).toBeGreaterThanOrEqual(60);
    }
  );
});

describe('buildTaxonomyIndexView', () => {
  it('shows only the poems count in the meters subtitle', () => {
    const { body } = buildTaxonomyIndexView('meters', [
      { name: 'الطويل', slug: 'altaweel', poemsCount: 12, poetsCount: 3 },
    ]);
    expect(body.heading).toBe('جميع البحور (بحر)');
    expect(body.items[0]?.subtitle).toBe('١٢ قصيدة');
    expect(body.emptyVariant).toBe('error');
  });

  it('shows only the poems count in the rhymes subtitle', () => {
    const { body } = buildTaxonomyIndexView('rhymes', [
      { name: 'الراء', slug: 'r', poemsCount: 5, poetsCount: 2 },
    ]);
    expect(body.items[0]?.subtitle).toBe('٥ قصائد');
  });

  it('never lists the unknown-meter sentinel among the top meters (regression: it has the most poems of any meter)', () => {
    const { layout } = buildTaxonomyIndexView('meters', [
      { name: 'غير معروف', slug: 'ghayrmaruf', poemsCount: 199829 },
      { name: 'الطويل', slug: 'altaweel', poemsCount: 44303 },
    ]);
    expect(layout.description).not.toContain('غير معروف');
    expect(layout.description).toContain('الطويل');
  });

  it('collections index description clears the 60-char SEO minimum even with a single, short collection', () => {
    const { layout } = buildTaxonomyIndexView('collections', [
      { name: 'ب', slug: 'x', poemsCount: 1 },
    ]);
    expect(layout.description.length).toBeGreaterThanOrEqual(60);
  });
});
