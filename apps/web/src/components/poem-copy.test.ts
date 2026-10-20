import { describe, expect, it } from 'vitest';

import {
  buildHighlightRegex,
  MAX_HIGHLIGHT_TERM_LENGTH,
  MAX_HIGHLIGHT_TERMS,
  parseHighlightTerms,
} from '@/lib/highlight';

import { poemSelectionUrl } from './poem-copy';

const canonicalUrl = 'https://qafiyah.com/poems/kdmy';

function termsFrom(lines: readonly string[]): readonly string[] {
  return parseHighlightTerms(new URL(poemSelectionUrl(canonicalUrl, lines)).hash);
}

describe('poemSelectionUrl', () => {
  it('uses the existing search fragment for each copied line', () => {
    const lines = ['لَمْ أَرَ خَيْلاً مِثْلَها', 'بَنِي شَمَجَى'];
    const url = new URL(poemSelectionUrl(canonicalUrl, lines));
    expect(`${url.origin}${url.pathname}`).toBe(canonicalUrl);
    expect(termsFrom(lines)).toEqual(lines);
  });

  it('replaces old search highlights with the copied lines', () => {
    const url = poemSelectionUrl(`${canonicalUrl}#h=old`, ['النص الجديد']);
    expect(parseHighlightTerms(new URL(url).hash)).toEqual(['النص الجديد']);
  });

  it('preserves commas and URL punctuation inside a copied line', () => {
    expect(termsFrom(['قال، وقال, ثم & سأل؟'])).toEqual(['قال، وقال, ثم & سأل؟']);
  });

  it('splits long lines at word boundaries within the existing highlight limit', () => {
    const line = 'لَمْ أَرَ خَيْلاً مِثْلَها يَوْمَ أَدْرَكَتْ بَنِي شَمَجَى خَلْفَ اللُّهَيْمِ عَلى ظَهْرِ';
    const terms = termsFrom([line]);
    expect(terms.length).toBeGreaterThan(1);
    expect(terms.every((term) => term.length <= MAX_HIGHLIGHT_TERM_LENGTH)).toBe(true);
    expect(terms.join(' ')).toBe(line);
    const regex = buildHighlightRegex(terms)!;
    expect([...line.matchAll(regex)].map((match) => match[0])).toEqual(terms);
  });

  it('keeps source whitespace so phrases still match the poem', () => {
    const line = `${'أول  ثانٍ   '.repeat(12)}آخر`;
    const terms = termsFrom([line]);
    expect(terms.length).toBeGreaterThan(1);
    expect(terms.every((term) => line.includes(term))).toBe(true);
  });

  it('does not merge phrases across an unrepresentable word', () => {
    expect(termsFrom([`أول ${'ع'.repeat(MAX_HIGHLIGHT_TERM_LENGTH + 1)} آخر`])).toEqual([
      'أول',
      'آخر',
    ]);
  });

  it('deduplicates repeated lines and respects the existing term count limit', () => {
    expect(termsFrom(['بيت مكرر', 'بيت مكرر'])).toEqual(['بيت مكرر']);
    const lines = Array.from({ length: MAX_HIGHLIGHT_TERMS + 2 }, (_, i) => `البيت ${i}`);
    expect(termsFrom(lines)).toEqual(lines.slice(0, MAX_HIGHLIGHT_TERMS));
  });
});
