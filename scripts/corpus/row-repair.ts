import { DEFAULT_SOUND_CLASSES, lettersOnly } from './arabic-text';
import { rhymeScore } from './qafiya';

const UNSPLIT_RATIO = 1.6;
const HALF_LINE_RATIO = 1.3;
const SPLIT_LOW = 0.7;
const SPLIT_HIGH = 1.3;
const RHYMES = 0.8;
const DOES_NOT_RHYME = 0.6;
const MIN_VERSES = 4;

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

export function isFullVerse(row: string): boolean {
  const parts = row.split('*');
  return parts.length === 2 && parts.every((part) => letters(part) > 0);
}

export function typicalHalf(rows: readonly string[], meterBaseline?: number): number | undefined {
  const verses = rows.filter((row) => isFullVerse(row));
  if (verses.length < MIN_VERSES) return meterBaseline;
  return median(verses.flatMap((row) => row.split('*').map((part) => letters(part))));
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

const MARKS = /[ً-ٰٟۖ-ۭـ]/;
const ANSWER = /^w(\d+)(?:\+(\d+))?$/;

const CLOSING = /^[^\p{L}\p{N}([{«“]+$/u;
const NEXT_TOKEN = /^\s+(\S+)/;

const isLetter = (character: string): boolean => lettersOnly(character).length > 0;

function closingEnd(row: string, end: number): number {
  const glued = /^\S*/.exec(row.slice(end))?.[0] ?? '';
  if (glued.length > 0 && !CLOSING.test(glued)) return end;
  let next = end + glued.length;
  let token = NEXT_TOKEN.exec(row.slice(next));
  while (token !== null && CLOSING.test(token[1] ?? '')) {
    next += token[0].length;
    token = NEXT_TOKEN.exec(row.slice(next));
  }
  return next;
}

export function numberedWords(row: string): string {
  return row
    .trim()
    .split(/\s+/)
    .map((word, index) => `${index + 1}:${word}`)
    .join(' ');
}

export function answerToLetters(row: string, answer: string): number | undefined {
  const match = ANSWER.exec(answer.trim());
  if (match === null) return undefined;
  const words = row.trim().split(/\s+/);
  const word = Number(match[1]);
  const offset = match[2] === undefined ? 0 : Number(match[2]);
  if (word < 1 || word > words.length) return undefined;
  if (offset > 0 && offset >= letters(words[word - 1] ?? '')) return undefined;
  const before =
    words.slice(0, word - 1).reduce((sum, earlier) => sum + letters(earlier), 0) + offset;
  return before > 0 ? before : undefined;
}

export function cutAt(row: string, lettersBefore: number): [string, string] | undefined {
  let seen = 0;
  for (let index = 0; index < row.length; index += 1) {
    if (isLetter(row[index] ?? '')) seen += 1;
    if (seen === lettersBefore) {
      let end = index + 1;
      while (end < row.length && MARKS.test(row[end] ?? '')) end += 1;
      end = closingEnd(row, end);
      const first = row.slice(0, end).trim();
      const second = row.slice(end).trim();
      return first.length > 0 && letters(second) > 0 ? [first, second] : undefined;
    }
  }
  return undefined;
}

export function acceptSplit(row: string, lettersBefore: number, half: number): string | undefined {
  const halves = cutAt(row, lettersBefore);
  if (halves === undefined) return undefined;
  const [first, second] = halves;
  const balanced = [first, second].every((part) => {
    const ratio = letters(part) / half;
    return ratio >= SPLIT_LOW && ratio <= SPLIT_HIGH;
  });
  const repaired = `${first}*${second}`;
  return balanced && lettersOnly(repaired) === lettersOnly(row) ? repaired : undefined;
}

export function copyCuts(rows: Iterable<string>): Map<string, number> {
  const cuts = new Map<string, number>();
  for (const row of rows) {
    if (!isFullVerse(row)) continue;
    cuts.set(foldKey(row), letters(row.split('*')[0] ?? ''));
  }
  return cuts;
}

export type Change = {
  readonly poemId: string;
  readonly slug: string;
  readonly oldRows: readonly string[];
  readonly newRows: readonly string[];
  readonly reason: string;
};

const TAG = '$qafiyah$';
const NUMERIC_ID = /^\d+$/;
const REFRESHES = [
  'SELECT refresh_poem_relations();',
  'SELECT refresh_random_poem_pool();',
  'SELECT refresh_taxonomy_stats();',
];

function quote(text: string): string {
  if (text.includes(TAG)) throw new Error(`refusing a row containing the quote tag ${TAG}`);
  return `${TAG}${text}${TAG}`;
}

export function buildApplySql(changes: readonly Change[]): string {
  const lines = ['BEGIN;'];
  for (const change of changes) {
    if (!NUMERIC_ID.test(change.poemId)) {
      throw new Error(`refusing a non-numeric poem id: ${change.poemId}`);
    }
    if (lettersOnly(change.newRows.join('')) !== lettersOnly(change.oldRows.join(''))) {
      throw new Error(`refusing ${change.slug}: its letters changed`);
    }
    lines.push(`DELETE FROM poem_verses WHERE poem_id = ${change.poemId};`);
    for (const [index, row] of change.newRows.entries()) {
      const text = quote(row);
      lines.push(
        `INSERT INTO verses (content, content_hash) VALUES (${text}, md5(${text})) ON CONFLICT (content_hash) DO NOTHING;`,
        `INSERT INTO poem_verses (poem_id, verse_id, position) SELECT ${change.poemId}, id, ${index + 1} FROM verses WHERE content_hash = md5(${text});`
      );
    }
    lines.push(
      `UPDATE poems SET verse_count = ${change.newRows.length} WHERE id = ${change.poemId};`
    );
  }
  lines.push('COMMIT;', ...REFRESHES);
  return `${lines.join('\n')}\n`;
}

export type PoemPlan =
  | { readonly kind: 'none' }
  | { readonly kind: 'merge'; readonly newRows: readonly string[] }
  | { readonly kind: 'unclear' }
  | { readonly kind: 'splits'; readonly indexes: readonly number[] };

function unsplitIndexes(rows: readonly string[], half: number): number[] {
  return rows.flatMap((row, index) => (isUnsplitVerse(row, half) ? [index] : []));
}

export function planPoem(rows: readonly string[], half: number): PoemPlan {
  const kind = classifyFlat(rows, half);
  if (kind === 'half-line-pairs') return { kind: 'merge', newRows: mergePairs(rows) };
  if (kind === 'unclear') return { kind: 'unclear' };
  if (kind === 'rhyming-half-lines') return { kind: 'none' };
  const indexes = unsplitIndexes(rows, half);
  return indexes.length > 0 ? { kind: 'splits', indexes } : { kind: 'none' };
}

export function applySplits(
  rows: readonly string[],
  splits: ReadonlyMap<number, string>
): string[] {
  return rows.map((row, index) => splits.get(index) ?? row);
}

export function acceptCouplets(rows: readonly string[]): string[] | undefined {
  const secondHalves = rows.filter((_, index) => index % 2 === 1);
  if (secondHalves.length < 2) return undefined;
  return rhymeScore(secondHalves, DEFAULT_SOUND_CLASSES) >= RHYMES ? mergePairs(rows) : undefined;
}

export function parseAnswers(csv: string): Map<string, string> {
  const answers = new Map<string, string>();
  for (const line of csv.split('\n').slice(1)) {
    const trimmed = line.trim();
    if (trimmed.length === 0) continue;
    const comma = trimmed.indexOf(',');
    if (comma < 0) continue;
    answers.set(trimmed.slice(0, comma).trim(), trimmed.slice(comma + 1).trim());
  }
  return answers;
}

export type MeasureScore = {
  readonly total: number;
  readonly answered: number;
  readonly exact: number;
  readonly unsure: number;
};

export function scoreMeasure(
  rows: ReadonlyMap<string, string>,
  truth: ReadonlyMap<string, number>,
  answers: ReadonlyMap<string, string>
): MeasureScore {
  let answered = 0;
  let exact = 0;
  let unsure = 0;
  for (const [key, cut] of truth) {
    const answer = answers.get(key);
    if (answer === undefined || answer === '?') {
      unsure += 1;
      continue;
    }
    answered += 1;
    if (answerToLetters(rows.get(key) ?? '', answer) === cut) exact += 1;
  }
  return { total: truth.size, answered, exact, unsure };
}

export function csvLine(fields: readonly string[]): string {
  return fields
    .map((field) => (/[",\n]/.test(field) ? `"${field.replaceAll('"', '""')}"` : field))
    .join(',');
}
