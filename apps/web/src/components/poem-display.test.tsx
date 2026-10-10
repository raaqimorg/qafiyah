import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import { PoemDisplay } from './poem-display';

type Props = Parameters<typeof PoemDisplay>[0];

const facets = {
  title: 'عنوان',
  poet: { name: 'شاعر', slug: 'Abcd', hasAvatar: false },
  era: { name: 'العباسي', slug: 'abbasi' },
  meter: { name: 'الطويل', slug: 'altawil' },
  theme: { name: 'المديح', slug: 'almadih' },
  verseCount: 1,
} as unknown as Omit<Props, 'verses' | 'poemType'>;

type Line = { readonly attributes: string; readonly text: string };

function render(verses: Props['verses'], poemTypeSlug = 'amudi') {
  const poemType = { name: 'نوع', slug: poemTypeSlug };
  const html = renderToStaticMarkup(
    <PoemDisplay {...facets} poemType={poemType} verses={verses} />
  );
  const article = html.slice(html.indexOf('<article'));
  const column = /<div class="([^"]*text-verse[^"]*)"/.exec(article)?.[1] ?? '';
  const entries: Line[][] = [
    ...article.matchAll(/<div class="[^"]*text-center[^"]*">(.*?)<\/div>/g),
  ].map((entry) =>
    [...(entry[1] ?? '').matchAll(/<p([^>]*)>(.*?)<\/p>/g)].map((line) => ({
      attributes: line[1] ?? '',
      text: line[2] ?? '',
    }))
  );
  return { article, column, entries };
}

describe('PoemDisplay', () => {
  it('renders a two-part entry as a classical verse, first half right and second half left', () => {
    const { entries } = render([['صدر', 'عجز']]);
    expect(entries).toHaveLength(1);
    expect(entries[0]?.map((line) => line.text)).toEqual(['صدر', 'عجز']);
    expect(entries[0]?.[0]?.attributes).toContain('text-right');
    expect(entries[0]?.[1]?.attributes).toContain('text-left');
  });

  it('renders a two-part entry of a nabati poem as a classical verse', () => {
    const { entries } = render([['صدر', 'عجز']], 'nabati');
    expect(entries[0]?.[0]?.attributes).toContain('text-right');
    expect(entries[0]?.[1]?.attributes).toContain('text-left');
  });

  it('centers a two-part entry of a poem type with no classical layout', () => {
    const { entries } = render([['صدر', 'عجز']], 'muwashshah');
    for (const line of entries[0] ?? []) {
      expect(line.attributes).not.toMatch(/text-right|text-left/);
    }
  });

  it('renders a one-part entry as one centered line with no empty second line', () => {
    const { article, entries } = render([['صدر', 'عجز'], ['سطر وحيد'], ['صدر ثان', 'عجز ثان']]);
    expect(entries.map((entry) => entry.length)).toEqual([2, 1, 2]);
    expect(entries[1]?.[0]?.text).toBe('سطر وحيد');
    expect(entries[1]?.[0]?.attributes).not.toMatch(/text-right|text-left/);
    expect(article).not.toMatch(/<p[^>]*><\/p>/);
  });

  it('never shifts the verses after a lone line', () => {
    const { entries } = render([['أ', 'ب'], ['ج'], ['د', 'ه']]);
    expect(entries.map((entry) => entry.map((line) => line.text))).toEqual([
      ['أ', 'ب'],
      ['ج'],
      ['د', 'ه'],
    ]);
  });

  it('renders a three-part entry as three stacked centered lines', () => {
    const { entries } = render([['أول', 'ثان', 'ثالث']]);
    expect(entries[0]?.map((line) => line.text)).toEqual(['أول', 'ثان', 'ثالث']);
    for (const line of entries[0] ?? []) {
      expect(line.attributes).not.toMatch(/text-right|text-left/);
    }
  });

  it('spaces a free-verse poem line by line rather than verse by verse', () => {
    const { column } = render([['سطر أول'], ['سطر ثان', 'سطر ثالث'], ['سطر رابع']], 'hurr');
    expect(column).toContain('gap-4 sm:gap-5');
    expect(column).not.toContain('gap-10');
  });

  it('spaces a poem made only of single lines line by line, whatever its type', () => {
    const { column } = render([['شطر أول'], ['شطر ثان'], ['شطر ثالث']], 'amudi');
    expect(column).toContain('gap-4 sm:gap-5');
    expect(column).not.toContain('gap-10');
  });

  it('keeps verse spacing for a poem that holds verses and is not free verse', () => {
    const { column } = render([['صدر', 'عجز'], ['قفل']], 'muwashshah');
    expect(column).toContain('gap-10 sm:gap-12');
  });
});
