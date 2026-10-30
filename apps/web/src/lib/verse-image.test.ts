import { describe, expect, it } from 'vitest';

import { layoutVerseCard } from './verse-image';

const measure = (line: string, fontSize: number) => line.length * fontSize * 0.5;

function layout(text: string, devicePixelRatio = 1) {
  const result = layoutVerseCard(text, measure, devicePixelRatio);
  if (result === null) throw new Error('the text does not fit on a card');
  return result;
}

describe('layoutVerseCard', () => {
  it('uses the largest font size when every line fits', () => {
    const card = layout('قصير\nقصير');
    expect(card.fontSize).toBe(54);
    expect(card.lines).toEqual(['قصير', 'قصير']);
  });

  it('shrinks the font until the longest line fits', () => {
    const line = 'ب'.repeat(40);
    const card = layout(line);
    expect(card.fontSize).toBeLessThan(54);
    expect(measure(line, card.fontSize)).toBeLessThanOrEqual(904);
  });

  it('wraps a line at word breaks when it is too long at the smallest font', () => {
    const word = 'كلمة';
    const line = Array.from({ length: 30 }, () => word).join(' ');
    const card = layout(line);
    expect(card.fontSize).toBe(28);
    expect(card.lines.length).toBeGreaterThan(1);
    expect(card.lines.join(' ')).toBe(line);
    for (const wrapped of card.lines) expect(measure(wrapped, 28)).toBeLessThanOrEqual(904);
  });

  it('keeps the blank line between verses', () => {
    const card = layout('أ\nب\n\nج\nد');
    expect(card.lines).toEqual(['أ', 'ب', '', 'ج', 'د']);
  });

  it('grows the card with the number of lines', () => {
    const short = layout('أ');
    const long = layout('أ\nب\nج\nد');
    expect(long.height).toBeGreaterThan(short.height);
  });

  it('draws at the device pixel ratio up to 2', () => {
    expect(layout('أ').pixelRatio).toBe(1);
    expect(layout('أ', 3).pixelRatio).toBe(2);
  });

  it('shrinks the font until the card fits the story height', () => {
    const text = Array.from({ length: 24 }, () => 'أ').join('\n');
    const card = layout(text);
    expect(card.fontSize).toBeLessThan(54);
    expect(card.height).toBeLessThanOrEqual(1920);
  });

  it('gives no card when the text is too tall even at the smallest font', () => {
    const text = Array.from({ length: 60 }, () => 'أ').join('\n');
    expect(layoutVerseCard(text, measure, 1)).toBeNull();
  });
});
