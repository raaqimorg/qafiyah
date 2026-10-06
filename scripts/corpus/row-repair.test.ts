import { describe, expect, it } from 'bun:test';

import { lettersOnly } from './arabic-text';
import {
  classifyFlat,
  foldKey,
  isUnsplitVerse,
  mergePairs,
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
