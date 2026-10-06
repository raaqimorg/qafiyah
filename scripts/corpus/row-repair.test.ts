import { describe, expect, it } from 'bun:test';

import { lettersOnly } from './arabic-text';
import {
  acceptSplit,
  answerToLetters,
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
    const rows = [`${half(9)}*${half(10)}`, `${half(10)}*${half(11)}`, half(12)];
    expect(typicalHalf(rows)).toBe(10);
  });

  it('falls back to the meter baseline when the poem has fewer than four halves', () => {
    expect(typicalHalf([`${half(9)}*${half(10)}`, half(20)], 12)).toBe(12);
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

  it('accepts halves within the expected share of a half-line and keeps every letter', () => {
    const repaired = acceptSplit(row, 11, 10);
    expect(repaired).toBe(`${'ب'.repeat(11)}*${'س'.repeat(9)}`);
    expect(lettersOnly(repaired ?? '')).toBe(lettersOnly(row));
  });

  it('refuses halves far from a half-line', () => {
    expect(acceptSplit(row, 5, 10)).toBeUndefined();
  });
});

describe('copyCuts', () => {
  it('keeps the first half length of every row with exactly one separator, by its folded letters', () => {
    const cuts = copyCuts([`${half(3)}*بببب`, half(5), 'ا*ب*ج']);
    expect([...cuts]).toEqual([[foldKey(`${half(3)}بببب`), 3]]);
  });
});
