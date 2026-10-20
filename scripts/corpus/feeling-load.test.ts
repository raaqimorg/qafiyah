import { describe, expect, it } from 'bun:test';

import { cleanVerse, feelingRows, loadSql } from './feeling-load';

describe('cleanVerse', () => {
  it('drops the vowel marks and joins the two halves with a space', () => {
    expect(cleanVerse('أَلا فَاعْجِلْ لِبُرْجَةَ بِالصَّبُوحِ*صَرِيحاً إِنَّها بِنْتُ الصَّرِيحِ')).toBe(
      'ألا فاعجل لبرجة بالصبوح صريحا إنها بنت الصريح'
    );
  });

  it('drops the tatweel and collapses the spaces', () => {
    expect(cleanVerse(' قـفا  نبك *  من ذكرى ')).toBe('قفا نبك من ذكرى');
  });
});

describe('feelingRows', () => {
  const feelings = new Set(['alhuzn', 'alshawq']);
  const results = new Map([
    [
      'قفا نبك من ذكرى',
      [
        ['alhuzn', 3],
        ['alshawq', 2],
        ['alhuzn', 2],
      ] as const,
    ],
    ['ألا هبي بصحنك', []],
  ]);

  it('gives each verse row the feelings of its text, each feeling once', () => {
    const { rows } = feelingRows(
      [
        { poemId: 17, position: 1, content: 'قِفا نَبكِ*مِن ذِكرى' },
        { poemId: 40, position: 9, content: 'ألا هبي*بصحنك' },
      ],
      results,
      feelings
    );
    expect(rows).toEqual([
      { poemId: 17, position: 1, slug: 'alhuzn', strength: 3 },
      { poemId: 17, position: 1, slug: 'alshawq', strength: 2 },
    ]);
  });

  it('counts the verse rows whose text has no result', () => {
    const { unclassified } = feelingRows(
      [{ poemId: 5, position: 2, content: 'لخولة أطلال*ببرقة ثهمد' }],
      results,
      feelings
    );
    expect(unclassified).toBe(1);
  });

  it('refuses a feeling that the tree does not have', () => {
    expect(() =>
      feelingRows(
        [{ poemId: 17, position: 1, content: 'قفا نبك*من ذكرى' }],
        results,
        new Set(['alhuzn'])
      )
    ).toThrow('alshawq');
  });
});

describe('loadSql', () => {
  const sql = loadSql([{ poemId: 17, position: 1, slug: 'alhuzn', strength: 3 }]);

  it('runs in one transaction', () => {
    expect(sql.startsWith('BEGIN;')).toBe(true);
    expect(sql.trimEnd().endsWith('COMMIT;')).toBe(true);
  });

  it('replaces every verse feeling with the given rows', () => {
    expect(sql).toContain('17,1,alhuzn,3\n\\.\n');
    expect(sql).toContain('DELETE FROM public.feeling_verses;');
  });

  it('finds each feeling by its slug', () => {
    expect(sql).toContain('JOIN public.feelings f ON f.slug = r.slug');
  });

  it('refreshes the counts and the ranked verses', () => {
    expect(sql).toContain('SELECT public.refresh_taxonomy_stats();');
  });
});
