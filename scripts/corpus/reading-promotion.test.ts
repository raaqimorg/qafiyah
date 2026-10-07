import { describe, expect, it } from 'bun:test';

import { MUALLAQAT_PRIMARIES, promotionSql, verseRows } from './reading-promotion';

const SOURCE = 'رواية الأنباري، بضبط فيصل المنصور';
const verses = [
  { n: 1, sadr: 'قِفا نَبكِ', ajuz: 'بِسَقطِ اللِّوى' },
  { n: 2, sadr: 'فتُوضِحَ', ajuz: 'لما نسجَتْها' },
];

describe('verseRows', () => {
  it('joins the two halves of each verse with one star', () => {
    expect(verseRows(verses)).toEqual(['قِفا نَبكِ*بِسَقطِ اللِّوى', 'فتُوضِحَ*لما نسجَتْها']);
  });

  it('refuses verses that are not numbered 1, 2, 3 in order', () => {
    expect(() => verseRows([verses[1]!, verses[0]!])).toThrow('verse 1');
  });

  it('refuses a half-line that holds a star', () => {
    expect(() => verseRows([{ n: 1, sadr: 'a*b', ajuz: 'c' }])).toThrow('star');
  });

  it('refuses an empty half-line', () => {
    expect(() => verseRows([{ n: 1, sadr: ' ', ajuz: 'c' }])).toThrow('empty');
  });
});

describe('promotionSql', () => {
  const sql = promotionSql([{ primarySlug: 'rHUD', verses }], SOURCE);

  it('runs in one transaction', () => {
    expect(sql.startsWith('BEGIN;')).toBe(true);
    expect(sql.trimEnd().endsWith('COMMIT;')).toBe(true);
  });

  it('finds the primary by its slug and refuses a primary that has a source already', () => {
    expect(sql).toContain("slug = 'rHUD'");
    expect(sql).toContain('source IS NOT NULL');
    expect(sql).toContain('RAISE EXCEPTION');
  });

  it('picks a new slug that no poem and no alias holds', () => {
    expect(sql).toContain('FROM public.poems WHERE slug = new_slug');
    expect(sql).toContain('FROM public.poem_aliases WHERE slug = new_slug');
  });

  it('moves the old rows to the new recension and writes the new rows into the primary', () => {
    expect(sql).toContain(
      'UPDATE public.poem_verses SET poem_id = new_id WHERE poem_id = primary_id;'
    );
    expect(sql).toContain('$qafiyah$قِفا نَبكِ*بِسَقطِ اللِّوى$qafiyah$');
    expect(sql).toContain('verse_count = 2');
    expect(sql).toContain(`source = $qafiyah$${SOURCE}$qafiyah$`);
  });

  it('checks the counts before it commits', () => {
    expect(sql).toContain('<> old_count');
    expect(sql.indexOf('<> old_count')).toBeLessThan(sql.indexOf('COMMIT;'));
  });

  it('refuses a row that holds the quote tag', () => {
    expect(() =>
      promotionSql(
        [{ primarySlug: 'rHUD', verses: [{ n: 1, sadr: '$qafiyah$', ajuz: 'b' }] }],
        SOURCE
      )
    ).toThrow('quote tag');
  });

  it('refuses a slug that is not four letters', () => {
    expect(() => promotionSql([{ primarySlug: "x'; DROP", verses }], SOURCE)).toThrow('slug');
  });

  it('knows the seven primaries', () => {
    expect(Object.values(MUALLAQAT_PRIMARIES)).toEqual([
      'rHUD',
      'xjIC',
      'gnNg',
      'iaqM',
      'YksA',
      'xxWN',
      'DTyF',
    ]);
  });
});
