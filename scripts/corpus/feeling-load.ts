import type { Tag } from './feeling-replies';

const MARKS = /[ؐ-ًؚ-ٰٟـ]/g;

export type VerseRow = {
  readonly poemId: number;
  readonly position: number;
  readonly content: string;
};

export type FeelingRow = {
  readonly poemId: number;
  readonly position: number;
  readonly slug: string;
  readonly strength: number;
};

export function cleanVerse(text: string): string {
  return text
    .replaceAll(MARKS, '')
    .replaceAll('*', ' ')
    .split(/\s+/)
    .filter((word) => word !== '')
    .join(' ');
}

export function feelingRows(
  verses: readonly VerseRow[],
  results: ReadonlyMap<string, readonly Tag[]>,
  feelings: ReadonlySet<string>
): { rows: FeelingRow[]; unclassified: number } {
  const rows: FeelingRow[] = [];
  let unclassified = 0;
  for (const verse of verses) {
    const tags = results.get(cleanVerse(verse.content));
    if (tags === undefined) {
      unclassified += 1;
      continue;
    }
    const seen = new Set<string>();
    for (const [slug, strength] of tags) {
      if (!feelings.has(slug)) throw new Error(`the tree has no feeling ${slug}`);
      if (seen.has(slug)) continue;
      seen.add(slug);
      rows.push({ poemId: verse.poemId, position: verse.position, slug, strength });
    }
  }
  return { rows, unclassified };
}

export function loadSql(rows: readonly FeelingRow[]): string {
  const data = rows
    .map((row) => `${row.poemId},${row.position},${row.slug},${row.strength}\n`)
    .join('');
  return [
    'BEGIN;',
    'CREATE TEMP TABLE feeling_rows (poem_id integer, position integer, slug text, strength smallint) ON COMMIT DROP;',
    'COPY feeling_rows FROM STDIN WITH (FORMAT csv);',
    `${data}\\.`,
    'DELETE FROM public.feeling_verses;',
    'INSERT INTO public.feeling_verses (poem_id, position, feeling_id, strength)',
    'SELECT r.poem_id, r.position, f.id, r.strength FROM feeling_rows r JOIN public.feelings f ON f.slug = r.slug;',
    'SELECT public.refresh_taxonomy_stats();',
    'COMMIT;',
    '',
  ].join('\n');
}
