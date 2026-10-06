import { describe, expect, it } from 'bun:test';

import { lettersOnly } from './arabic-text';
import {
  acceptCouplets,
  acceptSplit,
  answerToLetters,
  applySplits,
  buildApplySql,
  csvLine,
  parseAnswers,
  planPoem,
  scoreMeasure,
  classifyFlat,
  copyCuts,
  cutAt,
  foldKey,
  isUnsplitVerse,
  mergePairs,
  numberedWords,
  rhymePattern,
  typicalHalf,
} from './row-repair';

const RHYMING = 'ده';
const OTHERS = ['قلم', 'سفر', 'حجب', 'ملك', 'جبل', 'وطن'];

function line(letters: number, ending: string): string {
  const padding = letters - lettersOnly(ending).length;
  return `${'ب'.repeat(padding)} ${ending}`;
}

function half(letters: number): string {
  return 'س'.repeat(letters);
}

describe('typicalHalf', () => {
  it('is the median letter count of the halves of rows with one separator', () => {
    const rows = [
      `${half(9)}*${half(10)}`,
      `${half(10)}*${half(11)}`,
      `${half(10)}*${half(10)}`,
      `${half(9)}*${half(11)}`,
      half(12),
    ];
    expect(typicalHalf(rows)).toBe(10);
  });

  it('falls back to the meter baseline when fewer than four rows are full verses', () => {
    const rows = [`${half(9)}*${half(9)}`, `${half(9)}*${half(9)}`, `${half(9)}*${half(9)}`];
    expect(typicalHalf([...rows, half(19), half(20)], 19)).toBe(19);
  });

  it('does not count a row whose separator has no letters on one side', () => {
    const notes = Array.from({ length: 4 }, () => `*${half(10)}`);
    expect(typicalHalf([...notes, half(19), half(20)], 19)).toBe(19);
  });

  it('is undefined with neither enough halves nor a baseline', () => {
    expect(typicalHalf([half(20)])).toBeUndefined();
  });
});

describe('isUnsplitVerse', () => {
  it('flags a one-part row about twice a half-line long', () => {
    expect(isUnsplitVerse(half(20), 10)).toBe(true);
  });

  it('leaves a row one and a half half-lines long', () => {
    expect(isUnsplitVerse(half(15), 10)).toBe(false);
  });

  it('leaves any row that already has a separator', () => {
    expect(isUnsplitVerse(`${half(20)}*${half(20)}`, 10)).toBe(false);
  });
});

describe('rhymePattern', () => {
  it('is every when every line ends on the same rhyme', () => {
    expect(rhymePattern(Array.from({ length: 6 }, () => line(10, RHYMING)))).toBe('every');
  });

  it('is alternating when only every second line rhymes', () => {
    const lines = OTHERS.slice(0, 3).flatMap((other) => [line(10, other), line(10, RHYMING)]);
    expect(rhymePattern(lines)).toBe('alternating');
  });

  it('is unclear when the line ends share nothing', () => {
    expect(rhymePattern(OTHERS.map((other) => line(10, other)))).toBe('unclear');
  });
});

describe('classifyFlat', () => {
  it('finds half-line pairs when half-line rows rhyme on every second row', () => {
    const rows = OTHERS.slice(0, 3).flatMap((other) => [line(10, other), line(10, RHYMING)]);
    expect(classifyFlat(rows, 10)).toBe('half-line-pairs');
  });

  it('finds rhyming half-lines when every half-line row rhymes', () => {
    expect(
      classifyFlat(
        Array.from({ length: 6 }, () => line(10, RHYMING)),
        10
      )
    ).toBe('rhyming-half-lines');
  });

  it('finds verse-long rows about twice a half-line', () => {
    expect(
      classifyFlat(
        Array.from({ length: 4 }, () => line(20, RHYMING)),
        10
      )
    ).toBe('verse-long');
  });

  it('leaves half-line rows whose rhyme says nothing as unclear', () => {
    expect(
      classifyFlat(
        OTHERS.map((other) => line(10, other)),
        10
      )
    ).toBe('unclear');
  });

  it('is not flat when any row has a separator', () => {
    expect(classifyFlat([`${half(10)}*${half(10)}`, half(10)], 10)).toBe('not-flat');
  });
});

describe('mergePairs', () => {
  it('joins rows two by two and keeps an odd last row single', () => {
    expect(mergePairs(['a', 'b', 'c', 'd', 'e'])).toEqual(['a*b', 'c*d', 'e']);
  });
});

describe('foldKey', () => {
  it('folds alif forms, alif maqsura, and ta marbuta and drops marks, spaces, and separators', () => {
    expect(foldKey('أَإِآ ى ة*')).toBe(foldKey('ااا ي ه'));
  });
});

describe('numberedWords', () => {
  it('numbers each word from one so a review answer can name it', () => {
    expect(numberedWords(' قِفا  نَبكِ مِن ')).toBe('1:قِفا 2:نَبكِ 3:مِن');
  });
});

describe('answerToLetters', () => {
  const row = `${half(3)} ${half(4)} ${half(5)} ${half(6)}`;

  it('counts the letters before the word that opens the second half', () => {
    expect(answerToLetters(row, 'w3')).toBe(7);
  });

  it('adds the letters a straddling word keeps in the first half', () => {
    expect(answerToLetters(row, 'w2+2')).toBe(5);
  });

  it('refuses a cut before the first letter, a word past the end, an offset past the word, and an unsure answer', () => {
    expect(answerToLetters(row, 'w1')).toBeUndefined();
    expect(answerToLetters(row, 'w5')).toBeUndefined();
    expect(answerToLetters(row, 'w2+9')).toBeUndefined();
    expect(answerToLetters(row, '?')).toBeUndefined();
  });
});

describe('cutAt', () => {
  it('cuts at a word boundary and drops the space between the halves', () => {
    expect(cutAt(`${half(3)} ${half(4)}`, 3)).toEqual([half(3), half(4)]);
  });

  it('carries the diacritics of the last letter into the first half', () => {
    expect(cutAt('بَكَى سَمِعَ', 2)).toEqual(['بَكَ', 'ى سَمِعَ']);
  });

  it('keeps punctuation glued to the last word in the first half', () => {
    expect(cutAt('سسس، صصص', 3)).toEqual(['سسس،', 'صصص']);
  });

  it('keeps standalone closing punctuation in the first half', () => {
    expect(cutAt('سسس ... صصص', 3)).toEqual(['سسس ...', 'صصص']);
    expect(cutAt('سسس . . صصص', 3)).toEqual(['سسس . .', 'صصص']);
  });

  it('leaves an opening quote with the second half', () => {
    expect(cutAt('سسس « صصص »', 3)).toEqual(['سسس', '« صصص »']);
  });

  it('cuts inside a word when the word straddles the halves', () => {
    expect(cutAt(`${half(4)} ${half(4)}`, 2)).toEqual(['سس', `سس ${half(4)}`]);
  });

  it('refuses a cut that leaves either half empty', () => {
    expect(cutAt(half(4), 4)).toBeUndefined();
    expect(cutAt(half(4), 9)).toBeUndefined();
  });
});

describe('a real verse whose word straddles the halves', () => {
  const row = 'لَنَا وَلِهِنْدٍ بِبَطْنِ العَقِيقِ مُبْدىً وَمَنْزِلُهُ مُونِقُ';
  const copyFirst = 'لَنا وَلهندٍ بِبَطنِ العَقي';
  const copySecond = 'قِ مُبدىً وَمَنزِلُهُ مُونِقُ';

  it('splits where its correctly stored copy splits, inside a word, keeping every letter', () => {
    const [first, second] = cutAt(row, lettersOnly(copyFirst).length) ?? ['', ''];
    expect([lettersOnly(first).length, lettersOnly(second).length]).toEqual([17, 15]);
    expect(foldKey(first)).toBe(foldKey(copyFirst));
    expect(foldKey(second)).toBe(foldKey(copySecond));
    expect(lettersOnly(first + second)).toBe(lettersOnly(row));
  });
});

describe('acceptSplit', () => {
  const row = `${'ب'.repeat(11)} ${'س'.repeat(9)}`;

  it('accepts a cut that leaves each half a fair share of the row and keeps every letter', () => {
    const repaired = acceptSplit(row, 11);
    expect(repaired).toBe(`${'ب'.repeat(11)}*${'س'.repeat(9)}`);
    expect(lettersOnly(repaired ?? '')).toBe(lettersOnly(row));
  });

  it('refuses a cut that leaves one half under 35 percent of the row', () => {
    expect(acceptSplit(row, 5)).toBeUndefined();
  });

  it('accepts a balanced cut of a row longer than the poem usual verse', () => {
    const long = `${'ب'.repeat(25)} ${'س'.repeat(27)}`;
    expect(acceptSplit(long, 25)).toBe(`${'ب'.repeat(25)}*${'س'.repeat(27)}`);
  });
});

describe('copyCuts', () => {
  it('keeps the first half length of every row with exactly one separator, by its folded letters', () => {
    const cuts = copyCuts([`${half(3)}*بببب`, half(5), 'ا*ب*ج']);
    expect([...cuts]).toEqual([[foldKey(`${half(3)}بببب`), 3]]);
  });
});

describe('buildApplySql', () => {
  const split = {
    poemId: '42',
    slug: 'abcd',
    oldRows: [`${half(3)} ${half(4)}`, half(5)],
    newRows: [`${half(3)}*${half(4)}`, half(5)],
    reason: 'split',
  };

  it('replaces the rows of this poem only, inserting each text once by its hash, and sets its verse count', () => {
    const lines = buildApplySql([split]).trim().split('\n');
    expect(lines[0]).toBe('BEGIN;');
    expect(lines[1]).toBe('DELETE FROM poem_verses WHERE poem_id = 42;');
    expect(lines[2]).toBe(
      `INSERT INTO verses (content, content_hash) VALUES ($qafiyah$${half(3)}*${half(4)}$qafiyah$, md5($qafiyah$${half(3)}*${half(4)}$qafiyah$)) ON CONFLICT (content_hash) DO NOTHING;`
    );
    expect(lines[3]).toBe(
      `INSERT INTO poem_verses (poem_id, verse_id, position) SELECT 42, id, 1 FROM verses WHERE content_hash = md5($qafiyah$${half(3)}*${half(4)}$qafiyah$);`
    );
    expect(lines[5]).toContain('SELECT 42, id, 2 FROM verses');
    expect(lines[6]).toBe('UPDATE poems SET verse_count = 2 WHERE id = 42;');
  });

  it('never edits or deletes a verse, which another poem may share', () => {
    const sql = buildApplySql([split]);
    expect(sql).not.toMatch(/UPDATE\s+verses|DELETE\s+FROM\s+verses/i);
    expect(sql.match(/DELETE FROM poem_verses WHERE poem_id = (\d+);/g)).toEqual([
      'DELETE FROM poem_verses WHERE poem_id = 42;',
    ]);
  });

  it('runs as one transaction and refreshes the derived tables after it', () => {
    const lines = buildApplySql([split]).trim().split('\n');
    expect(lines.slice(-4)).toEqual([
      'COMMIT;',
      'SELECT refresh_poem_relations();',
      'SELECT refresh_random_poem_pool();',
      'SELECT refresh_taxonomy_stats();',
    ]);
  });

  it('refuses a non-numeric poem id, text containing the quote tag, and any change to the letters', () => {
    expect(() => buildApplySql([{ ...split, poemId: '42; DROP' }])).toThrow('non-numeric');
    expect(() =>
      buildApplySql([{ ...split, newRows: ['$qafiyah$'], oldRows: ['$qafiyah$'] }])
    ).toThrow('quote tag');
    expect(() => buildApplySql([{ ...split, newRows: [half(3)] }])).toThrow('letters changed');
  });
});

describe('planPoem', () => {
  const pairs = OTHERS.slice(0, 3).flatMap((other) => [line(10, other), line(10, RHYMING)]);

  it('merges a poem of half-line pairs into couplets', () => {
    expect(planPoem(pairs, 10)).toEqual({ kind: 'merge', newRows: mergePairs(pairs) });
  });

  it('queues a poem of unclear half-lines for review', () => {
    expect(
      planPoem(
        OTHERS.map((other) => line(10, other)),
        10
      )
    ).toEqual({ kind: 'unclear' });
  });

  it('leaves a poem of rhyming half-lines alone', () => {
    expect(
      planPoem(
        Array.from({ length: 6 }, () => line(10, RHYMING)),
        10
      )
    ).toEqual({ kind: 'none' });
  });

  it('lists the unsplit verse rows of a mixed poem by index', () => {
    const rows = [`${half(10)}*${half(10)}`, half(20), `${half(10)}*${half(10)}`, half(11)];
    expect(planPoem(rows, 10)).toEqual({ kind: 'splits', indexes: [1] });
  });

  it('lists every verse-long row of a flat poem as an unsplit verse', () => {
    expect(planPoem([half(20), half(21), half(19)], 10)).toEqual({
      kind: 'splits',
      indexes: [0, 1, 2],
    });
  });

  it('leaves a poem of full verses alone', () => {
    expect(planPoem([`${half(10)}*${half(10)}`], 10)).toEqual({ kind: 'none' });
  });
});

describe('applySplits', () => {
  it('replaces only the split rows and keeps the rest in order', () => {
    const repaired = applySplits(['a', 'bc', 'd'], new Map([[1, 'b*c']]));
    expect(repaired).toEqual(['a', 'b*c', 'd']);
  });
});

describe('acceptCouplets', () => {
  it('merges when the merged second halves rhyme', () => {
    const rows = OTHERS.slice(0, 3).flatMap((other) => [line(10, other), line(10, RHYMING)]);
    expect(acceptCouplets(rows)).toEqual(mergePairs(rows));
  });

  it('refuses when the merged second halves do not rhyme', () => {
    expect(acceptCouplets(OTHERS.map((other) => line(10, other)))).toBeUndefined();
  });
});

describe('parseAnswers', () => {
  it('reads key,answer lines after the header and skips blank lines', () => {
    expect([...parseAnswers('key,answer\nabcd:3,w4\n\nabcd:5,?\n')]).toEqual([
      ['abcd:3', 'w4'],
      ['abcd:5', '?'],
    ]);
  });
});

describe('scoreMeasure', () => {
  it('counts exact cuts among answered verses and unsure answers apart', () => {
    const rows = new Map([
      ['a:1', `${half(3)} ${half(4)}`],
      ['b:1', `${half(3)} ${half(4)}`],
      ['c:1', `${half(3)} ${half(4)}`],
    ]);
    const truth = new Map([
      ['a:1', 3],
      ['b:1', 3],
      ['c:1', 3],
    ]);
    const answers = new Map([
      ['a:1', 'w2'],
      ['b:1', 'w2+1'],
      ['c:1', '?'],
    ]);
    expect(scoreMeasure(rows, truth, answers)).toEqual({
      total: 3,
      answered: 2,
      exact: 1,
      unsure: 1,
    });
  });
});

describe('csvLine', () => {
  it('quotes a field that holds a comma, a quote, or a newline', () => {
    expect(csvLine(['a', 'b,c', 'say "hi"'])).toBe('a,"b,c","say ""hi"""');
  });
});
