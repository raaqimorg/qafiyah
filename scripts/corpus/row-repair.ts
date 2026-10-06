import { DEFAULT_SOUND_CLASSES, lettersOnly } from './arabic-text';
import { rhymeScore } from './qafiya';

export const UNSPLIT_RATIO = 1.6;
export const HALF_LINE_RATIO = 1.3;
export const SPLIT_LOW = 0.7;
export const SPLIT_HIGH = 1.3;
const RHYMES = 0.8;
const DOES_NOT_RHYME = 0.6;
const MIN_HALVES = 4;

export type RhymePattern = 'every' | 'alternating' | 'unclear';
export type FlatKind =
  | 'half-line-pairs'
  | 'rhyming-half-lines'
  | 'unclear'
  | 'verse-long'
  | 'not-flat';

export const letters = (text: string): number => lettersOnly(text).length;

function median(values: readonly number[]): number | undefined {
  if (values.length === 0) return undefined;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}

export function foldKey(text: string): string {
  return lettersOnly(
    text
      .replaceAll(/[أإآٱ]/g, 'ا')
      .replaceAll('ى', 'ي')
      .replaceAll('ة', 'ه')
  );
}

export function typicalHalf(rows: readonly string[], meterBaseline?: number): number | undefined {
  const halves = rows
    .filter((row) => row.split('*').length === 2)
    .flatMap((row) => row.split('*').map((part) => letters(part)));
  return halves.length >= MIN_HALVES ? median(halves) : meterBaseline;
}

export function isUnsplitVerse(row: string, half: number): boolean {
  return !row.includes('*') && letters(row) >= UNSPLIT_RATIO * half;
}

export function rhymePattern(lines: readonly string[]): RhymePattern {
  if (rhymeScore(lines, DEFAULT_SOUND_CLASSES) >= RHYMES) return 'every';
  const odd = rhymeScore(
    lines.filter((_, index) => index % 2 === 1),
    DEFAULT_SOUND_CLASSES
  );
  const even = rhymeScore(
    lines.filter((_, index) => index % 2 === 0),
    DEFAULT_SOUND_CLASSES
  );
  return odd >= RHYMES && even < DOES_NOT_RHYME ? 'alternating' : 'unclear';
}

export function classifyFlat(rows: readonly string[], half: number): FlatKind {
  if (rows.some((row) => row.includes('*'))) return 'not-flat';
  const length = median(rows.map((row) => letters(row))) ?? 0;
  if (length >= UNSPLIT_RATIO * half) return 'verse-long';
  if (length > HALF_LINE_RATIO * half) return 'unclear';
  const pattern = rhymePattern(rows);
  if (pattern === 'alternating') return 'half-line-pairs';
  if (pattern === 'every') return 'rhyming-half-lines';
  return 'unclear';
}

export function mergePairs(rows: readonly string[]): string[] {
  const merged: string[] = [];
  for (let index = 0; index < rows.length; index += 2) {
    const first = rows[index] ?? '';
    const second = rows[index + 1];
    merged.push(second === undefined ? first : `${first}*${second}`);
  }
  return merged;
}
