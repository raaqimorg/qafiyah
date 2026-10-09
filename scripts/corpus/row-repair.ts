import { DEFAULT_SOUND_CLASSES, lettersOnly } from './arabic-text';
import { rhymeScore, sharedLiteralSuffixLength } from './qafiya';

const UNSPLIT_RATIO = 1.6;
const LONG_RATIO = 2.6;
const HALF_LINE_RATIO = 1.3;
const STRAY_MIN = 0.8;
const STRAY_MAX = 1.15;
const LONGEST_STRAY_RUN = 3;
const SHORT_HALF = 0.55;
const LONG_HALF = 1.45;
const FULL_VERSE_RATIO = 1.7;
const MIN_FULL_VERSE_SHARE = 0.15;
const OWN_MIN = 0.8;
const OWN_MAX = 1.25;
const MIN_REGULAR_SHARE = 0.7;
const RHYMING_SUFFIX = 2;
const MIN_SHARE = 0.35;
const MAX_SHARE = 0.65;
const RHYMES = 0.8;
const DOES_NOT_RHYME = 0.6;
const MIN_VERSES = 4;
const FOUR_FOOT_METERS = new Set(['altawil', 'albasit', 'almutakarib', 'almutadarak']);
const ELLIPSIS = /\.\.|…/;
const DIGIT = /\p{Nd}/u;

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

export type Reference =
  | { readonly kind: 'checked'; readonly half: number }
  | { readonly kind: 'unchecked'; readonly half: number };

function fullVerseShare(rows: readonly string[], half: number): number {
  return rows.filter((row) => letters(row) >= FULL_VERSE_RATIO * half).length / rows.length;
}

export function isRegular(rows: readonly string[], half: number): boolean {
  const regular = rows.filter((row) => {
    const size = letters(row) / half;
    return (size >= 0.75 && size <= 1.25) || (size >= UNSPLIT_RATIO && size <= 2.4);
  });
  return regular.length / rows.length >= MIN_REGULAR_SHARE;
}

export function referenceHalf(
  rows: readonly string[],
  meterBaseline: number | undefined
): Reference | undefined {
  const own = typicalHalf(rows);
  if (meterBaseline === undefined) {
    return own === undefined ? undefined : { kind: 'unchecked', half: own };
  }
  const hasFullVerses = fullVerseShare(rows, meterBaseline) >= MIN_FULL_VERSE_SHARE;
  const checkedIfRegular = (half: number): Reference => ({
    kind: isRegular(rows, half) ? 'checked' : 'unchecked',
    half,
  });
  if (own === undefined) {
    return hasFullVerses
      ? checkedIfRegular(meterBaseline)
      : { kind: 'unchecked', half: meterBaseline };
  }
  const ratio = own / meterBaseline;
  if (ratio >= OWN_MIN && ratio <= OWN_MAX) return checkedIfRegular(own);
  if (ratio < OWN_MIN && hasFullVerses) return checkedIfRegular(meterBaseline);
  return { kind: 'unchecked', half: own };
}

export function isUnsplitVerse(row: string, half: number): boolean {
  const size = letters(row) / half;
  return !row.includes('*') && size >= UNSPLIT_RATIO && size < LONG_RATIO;
}

export function isStraySplit(row: string, half: number): boolean {
  const size = letters(row) / half;
  return isFullVerse(row) && size >= STRAY_MIN && size <= STRAY_MAX;
}

export function isMisplacedSplit(row: string, half: number): boolean {
  if (!isFullVerse(row)) return false;
  const sizes = row.split('*').map((part) => letters(part) / half);
  const size = letters(row) / half;
  return (
    size >= UNSPLIT_RATIO &&
    size < LONG_RATIO &&
    Math.min(...sizes) < SHORT_HALF &&
    Math.max(...sizes) > LONG_HALF
  );
}

export function isLongRow(row: string, half: number): boolean {
  return letters(row) >= LONG_RATIO * half;
}

function wordsWithLetters(text: string): string[] {
  return text
    .trim()
    .split(/\s+/)
    .filter((word) => letters(word) > 0);
}

export function isConfidentStray(row: string, meter: string): boolean {
  const parts = row.split('*');
  return (
    FOUR_FOOT_METERS.has(meter) &&
    parts.length === 2 &&
    parts.every((part) => wordsWithLetters(part).length >= 2) &&
    sharedLiteralSuffixLength(parts) < RHYMING_SUFFIX &&
    !ELLIPSIS.test(row) &&
    !DIGIT.test(row)
  );
}

export function indexesInLongRuns(indexes: readonly number[]): Set<number> {
  const inLongRuns = new Set<number>();
  let run: number[] = [];
  const close = (): void => {
    if (run.length > LONGEST_STRAY_RUN) for (const index of run) inLongRuns.add(index);
    run = [];
  };
  for (const index of [...indexes].sort((a, b) => a - b)) {
    if (run.length > 0 && index !== (run.at(-1) ?? 0) + 1) close();
    run.push(index);
  }
  close();
  return inLongRuns;
}

export function glueSplit(row: string): string | undefined {
  const parts = row.split('*');
  if (parts.length !== 2) return undefined;
  return `${(parts[0] ?? '').trimEnd()}${(parts[1] ?? '').trimStart()}`;
}

export function joinSplit(row: string): string | undefined {
  const parts = row.split('*');
  if (parts.length !== 2) return undefined;
  const first = (parts[0] ?? '').trimEnd();
  const second = (parts[1] ?? '').trimStart();
  const before = wordsWithLetters(first).at(-1) ?? '';
  const after = wordsWithLetters(second)[0] ?? '';
  if (letters(before) <= 1 || letters(after) <= 1) return undefined;
  return `${first} ${second}`;
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
const GLUED_CLOSING = /^[^\p{L}\p{N}\s"([{«“]*/u;
const NEXT_TOKEN = /^\s+(\S+)/;

const isLetter = (character: string): boolean => lettersOnly(character).length > 0;

function closingEnd(row: string, end: number): number {
  const glued = /^\S*/.exec(row.slice(end))?.[0] ?? '';
  if (glued.length > 0 && !CLOSING.test(glued)) {
    return end + (GLUED_CLOSING.exec(glued)?.[0].length ?? 0);
  }
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

export function acceptSplit(row: string, lettersBefore: number): string | undefined {
  const halves = cutAt(row, lettersBefore);
  if (halves === undefined) return undefined;
  const [first, second] = halves;
  const share = letters(first) / letters(row);
  const balanced = share >= MIN_SHARE && share <= MAX_SHARE;
  const repaired = `${first}*${second}`;
  return balanced && lettersOnly(repaired) === lettersOnly(row) ? repaired : undefined;
}

export function copyCuts(rows: Iterable<string>): Map<string, number> {
  const cuts = new Map<string, number>();
  for (const row of rows) {
    if (!isFullVerse(row)) continue;
    const first = letters(row.split('*')[0] ?? '');
    const share = first / letters(row);
    if (share >= MIN_SHARE && share <= MAX_SHARE) cuts.set(foldKey(row), first);
  }
  return cuts;
}

export type Change = {
  readonly poemId: string;
  readonly slug: string;
  readonly oldRows: readonly string[];
  readonly newRows: readonly string[];
  readonly dropped?: readonly string[];
  readonly reason: string;
};

function lettersWithout(rows: readonly string[], dropped: readonly string[]): string | undefined {
  let remaining = lettersOnly(rows.join(''));
  for (const text of dropped) {
    const removed = lettersOnly(text);
    const at = removed.length === 0 ? -1 : remaining.indexOf(removed);
    if (at < 0) return undefined;
    remaining = remaining.slice(0, at) + remaining.slice(at + removed.length);
  }
  return remaining;
}

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
    const kept = lettersWithout(change.oldRows, change.dropped ?? []);
    if (kept === undefined) {
      throw new Error(`refusing ${change.slug}: a dropped text is not in its rows`);
    }
    if (lettersOnly(change.newRows.join('')) !== kept) {
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

export type RowRepairs = {
  readonly unsplit: readonly number[];
  readonly misplaced: readonly number[];
  readonly stray: readonly number[];
  readonly long: readonly number[];
};

export type PoemPlan =
  | { readonly kind: 'none' }
  | { readonly kind: 'merge'; readonly newRows: readonly string[] }
  | { readonly kind: 'unclear' }
  | ({ readonly kind: 'rows' } & RowRepairs);

function indexesWhere(rows: readonly string[], test: (row: string) => boolean): number[] {
  return rows.flatMap((row, index) => (test(row) ? [index] : []));
}

export function planPoem(rows: readonly string[], reference: Reference): PoemPlan {
  const { half } = reference;
  const kind = classifyFlat(rows, half);
  if (kind === 'half-line-pairs') return { kind: 'merge', newRows: mergePairs(rows) };
  if (kind === 'unclear') return { kind: 'unclear' };
  if (kind === 'rhyming-half-lines') return { kind: 'none' };
  const repairs: RowRepairs = {
    unsplit: indexesWhere(rows, (row) => isUnsplitVerse(row, half)),
    misplaced: indexesWhere(rows, (row) => isMisplacedSplit(row, half)),
    stray: reference.kind === 'checked' ? indexesWhere(rows, (row) => isStraySplit(row, half)) : [],
    long: indexesWhere(rows, (row) => isLongRow(row, half)),
  };
  const isEmpty = Object.values(repairs).every((indexes) => indexes.length === 0);
  return isEmpty ? { kind: 'none' } : { kind: 'rows', ...repairs };
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
