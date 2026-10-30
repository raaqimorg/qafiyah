import { describe, expect, it } from 'vitest';

import { layoutVerseCard } from './verse-image';

const measure = (line: string, fontSize: number) => line.length * fontSize * 0.5;

describe('layoutVerseCard', () => {
  it('uses the largest font size when every line fits', () => {
    const layout = layoutVerseCard('قصير\nقصير', measure, 1);
    expect(layout.fontSize).toBe(54);
    expect(layout.lines).toEqual(['قصير', 'قصير']);
  });

  it('shrinks the font until the longest line fits', () => {
    const line = 'ب'.repeat(40);
    const layout = layoutVerseCard(line, measure, 1);
    expect(layout.fontSize).toBeLessThan(54);
    expect(measure(line, layout.fontSize)).toBeLessThanOrEqual(904);
  });

  it('wraps a line at word breaks when it is too long at the smallest font', () => {
    const word = 'كلمة';
    const line = Array.from({ length: 30 }, () => word).join(' ');
    const layout = layoutVerseCard(line, measure, 1);
    expect(layout.fontSize).toBe(28);
    expect(layout.lines.length).toBeGreaterThan(1);
    expect(layout.lines.join(' ')).toBe(line);
    for (const wrapped of layout.lines) expect(measure(wrapped, 28)).toBeLessThanOrEqual(904);
  });

  it('keeps the blank line between verses', () => {
    const layout = layoutVerseCard('أ\nب\n\nج\nد', measure, 1);
    expect(layout.lines).toEqual(['أ', 'ب', '', 'ج', 'د']);
  });

  it('grows the card with the number of lines', () => {
    const short = layoutVerseCard('أ', measure, 1);
    const long = layoutVerseCard('أ\nب\nج\nد', measure, 1);
    expect(long.height).toBeGreaterThan(short.height);
  });

  it('draws at the device pixel ratio up to 2', () => {
    expect(layoutVerseCard('أ', measure, 1).pixelRatio).toBe(1);
    expect(layoutVerseCard('أ', measure, 3).pixelRatio).toBe(2);
  });
});
