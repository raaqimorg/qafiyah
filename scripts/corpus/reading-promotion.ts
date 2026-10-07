export type Verse = { readonly n: number; readonly sadr: string; readonly ajuz: string };
export type Reading = { readonly primarySlug: string; readonly verses: readonly Verse[] };

export const MUALLAQAT_PRIMARIES: Readonly<Record<string, string>> = {
  'imru-al-qais': 'rHUD',
  tarafa: 'xjIC',
  zuhayr: 'gnNg',
  antara: 'iaqM',
  'amr-ibn-kulthum': 'YksA',
  'al-harith': 'xxWN',
  labid: 'DTyF',
};

const TAG = '$qafiyah$';
const SLUG = /^[A-Za-z]{4}$/;
const LETTERS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz';
const COPIED = [
  'poet_id',
  'meter_id',
  'theme_id',
  'rhyme_id',
  'era_id',
  'collection_id',
  'form_id',
  'poem_type_id',
  'register_id',
  'genre_id',
  'rhyme_majra_id',
  'title',
  'verse_count',
  'flags',
  'is_hidden',
  'has_tashkeel',
].join(', ');

function quote(text: string): string {
  if (text.includes(TAG)) throw new Error(`refusing a row containing the quote tag ${TAG}`);
  return `${TAG}${text}${TAG}`;
}

function half(text: string, n: number): string {
  const trimmed = text.trim();
  if (trimmed === '') throw new Error(`verse ${n} has an empty half-line`);
  if (trimmed.includes('*')) throw new Error(`verse ${n} has a star inside a half-line`);
  return trimmed;
}

export function verseRows(verses: readonly Verse[]): string[] {
  return verses.map((verse, index) => {
    if (verse.n !== index + 1) throw new Error(`expected verse ${index + 1}, found ${verse.n}`);
    return `${half(verse.sadr, verse.n)}*${half(verse.ajuz, verse.n)}`;
  });
}

function promoteOne(reading: Reading, source: string): string {
  if (!SLUG.test(reading.primarySlug)) throw new Error(`refusing the slug ${reading.primarySlug}`);
  const rows = verseRows(reading.verses);
  const inserts = rows.flatMap((row, index) => {
    const text = quote(row);
    return [
      `  INSERT INTO public.verses (content, content_hash) VALUES (${text}, md5(${text})) ON CONFLICT (content_hash) DO NOTHING;`,
      `  INSERT INTO public.poem_verses (poem_id, verse_id, position) SELECT primary_id, id, ${index + 1} FROM public.verses WHERE content_hash = md5(${text});`,
    ];
  });
  return [
    'DO $promote$',
    'DECLARE',
    '  primary_id integer;',
    '  new_id integer;',
    '  new_slug text;',
    '  old_count integer;',
    'BEGIN',
    `  SELECT id, verse_count INTO primary_id, old_count FROM public.poems WHERE slug = '${reading.primarySlug}' AND recension_of_id IS NULL;`,
    `  IF primary_id IS NULL THEN RAISE EXCEPTION 'no primary with the slug ${reading.primarySlug}'; END IF;`,
    `  IF EXISTS (SELECT 1 FROM public.poems WHERE id = primary_id AND source IS NOT NULL) THEN RAISE EXCEPTION '${reading.primarySlug} has a source already'; END IF;`,
    '  LOOP',
    `    new_slug := (SELECT string_agg(substr('${LETTERS}', 1 + floor(random() * 52)::integer, 1), '') FROM generate_series(1, 4));`,
    '    EXIT WHEN NOT EXISTS (SELECT 1 FROM public.poems WHERE slug = new_slug)',
    '      AND NOT EXISTS (SELECT 1 FROM public.poem_aliases WHERE slug = new_slug);',
    '  END LOOP;',
    `  INSERT INTO public.poems (${COPIED}, slug, recension_of_id)`,
    `  SELECT ${COPIED}, new_slug, primary_id FROM public.poems WHERE id = primary_id`,
    '  RETURNING id INTO new_id;',
    '  UPDATE public.poem_verses SET poem_id = new_id WHERE poem_id = primary_id;',
    ...inserts,
    `  UPDATE public.poems SET verse_count = ${rows.length}, source = ${quote(source)} WHERE id = primary_id;`,
    `  IF (SELECT count(*) FROM public.poem_verses WHERE poem_id = new_id) <> old_count THEN RAISE EXCEPTION '${reading.primarySlug}: old_count <> moved rows'; END IF;`,
    `  IF (SELECT count(*) FROM public.poem_verses WHERE poem_id = primary_id) <> ${rows.length} THEN RAISE EXCEPTION '${reading.primarySlug}: new rows missing'; END IF;`,
    `  RAISE NOTICE '${reading.primarySlug}: % old rows moved to %, ${rows.length} new rows', old_count, new_slug;`,
    'END $promote$;',
  ].join('\n');
}

export function promotionSql(readings: readonly Reading[], source: string): string {
  return `${['BEGIN;', ...readings.map((reading) => promoteOne(reading, source)), 'COMMIT;'].join('\n')}\n`;
}
