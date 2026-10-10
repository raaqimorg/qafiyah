import { describe, expect, it } from 'vitest';

import { stripTashkeel } from './arabic';
import {
  buildHighlightRegex,
  escapeRegExp,
  highlightSegments,
  MAX_HIGHLIGHT_TERM_LENGTH,
  MAX_HIGHLIGHT_TERMS,
  parseHighlightTerms,
  safeDecode,
} from './highlight';

describe('safeDecode', () => {
  it('decodes a percent-encoded term', () => {
    expect(safeDecode('%D9%8A%D8%A7')).toBe('يا');
  });

  it('returns the raw value when the encoding is malformed', () => {
    expect(safeDecode('%zz')).toBe('%zz');
  });
});

describe('escapeRegExp', () => {
  it('escapes every regex metacharacter', () => {
    expect(escapeRegExp('a.b*c?d^e$f(g)h[i]j{k}l|m\\n')).toBe(
      'a\\.b\\*c\\?d\\^e\\$f\\(g\\)h\\[i\\]j\\{k\\}l\\|m\\\\n'
    );
  });
});

describe('buildHighlightRegex', () => {
  it('returns null for no terms', () => {
    expect(buildHighlightRegex([])).toBeNull();
  });

  it('matches whole words only, not substrings of longer words', () => {
    const regex = buildHighlightRegex(['حب']);
    expect(regex?.test('قلب حب')).toBe(true);
    expect(regex?.test('حبيب')).toBe(false);
  });

  it('orders longer alternatives first so the longest wins', () => {
    const regex = buildHighlightRegex(['يا', 'يا ليت']);
    const match = regex?.exec('يا ليت')?.[0];
    expect(match).toBe('يا ليت');
  });

  it('treats regex metacharacters literally', () => {
    const regex = buildHighlightRegex(['.']) as RegExp;
    expect(regex.test('قف . نت')).toBe(true);
    expect(regex.test('قف د نت')).toBe(false);
  });
});

describe('parseHighlightTerms', () => {
  it('reads terms from a #h= fragment', () => {
    expect(parseHighlightTerms('#h=%D9%8A%D8%A7,هذا')).toEqual(['يا', 'هذا']);
  });

  it('returns nothing for any other fragment', () => {
    expect(parseHighlightTerms('')).toEqual([]);
    expect(parseHighlightTerms('#other')).toEqual([]);
  });

  it('drops empty terms and caps the count at the real constant', () => {
    const many = Array.from({ length: 40 }, (_, i) => `t${i}`).join(',');
    expect(parseHighlightTerms(`#h=${many}`)).toHaveLength(MAX_HIGHLIGHT_TERMS);
  });

  it('drops terms longer than the real constant', () => {
    expect(parseHighlightTerms(`#h=${'x'.repeat(MAX_HIGHLIGHT_TERM_LENGTH + 1)}`)).toEqual([]);
  });
});

describe('highlightSegments', () => {
  it('marks only the matched spans', () => {
    const regex = buildHighlightRegex(['حب']) as RegExp;
    expect(highlightSegments('قلب حب القلب', regex)).toEqual([
      { text: 'قلب ', highlighted: false },
      { text: 'حب', highlighted: true },
      { text: ' القلب', highlighted: false },
    ]);
  });

  it('returns a single unhighlighted segment when nothing matches', () => {
    const regex = buildHighlightRegex(['غائب']) as RegExp;
    expect(highlightSegments('نص', regex)).toEqual([{ text: 'نص', highlighted: false }]);
  });

  it('still finds a vocalized search term once the verse and the term both lose their tashkeel', () => {
    const verse = 'وَإِذا الشَّدائِدُ مَرَّةً أَشْجَتْكُمُ';
    const regex = buildHighlightRegex([stripTashkeel('الشَّدائِدُ مَرَّةً')]) as RegExp;
    expect(highlightSegments(stripTashkeel(verse), regex)).toContainEqual({
      text: 'الشدائد مرة',
      highlighted: true,
    });
  });
});
