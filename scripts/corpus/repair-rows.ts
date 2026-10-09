#!/usr/bin/env bun

import { mkdir, readdir } from 'node:fs/promises';
import { join } from 'node:path';

import {
  acceptCouplets,
  acceptSplit,
  answerToLetters,
  applySplits,
  buildApplySql,
  copyCuts,
  csvLine,
  foldKey,
  glueSplit,
  indexesInLongRuns,
  isConfidentStray,
  isFullVerse,
  joinSplit,
  letters,
  numberedWords,
  parseAnswers,
  planPoem,
  referenceHalf,
  scoreMeasure,
  typicalHalf,
  type Change,
  type TitleChange,
} from './row-repair';

const DEFAULT_DIR = 'reports/corpus/row-repair';
const CLASSICAL = 'amudi';
const UNKNOWN_TYPE = 'majhul';
const CLASSICAL_METERS = new Set([
  'altawil',
  'alkamil',
  'albasit',
  'alwafir',
  'alkhafif',
  'alsarie',
  'alrajz',
  'almutakarib',
  'alramal',
  'almunsarih',
  'almujtath',
  'alhazaj',
  'almadid',
  'almutadarak',
  'alkhabab',
  'almuqtadab',
  'almudare',
  'majzualkamil',
  'majzualramal',
  'majzualrajaz',
  'majzualwafir',
  'majzualkhafif',
  'majzualbasit',
  'majzualmutaqarib',
  'majzualmadid',
  'mashturalrajaz',
  'mashturalsari',
  'ahdhalkamil',
  'ahdhalwafir',
  'maqtualkamil',
  'mukhallaalbasit',
  'manhukalmutadarik',
]);
const BATCH = 100;
const MEASURE_SIZE = 300;
const STRAY_SAMPLE_SIZE = 150;
const STRAY_SAMPLE_ROUND = 4;
const MEASURE_ROUND = 2;
const MIN_EXACT_RATE = 0.98;
const SEED = 186;

type Poem = {
  readonly id: string;
  readonly slug: string;
  readonly type: string;
  readonly meter: string;
  readonly rows: readonly string[];
};

type Candidate = {
  readonly key: string;
  readonly poem: Poem;
  readonly row: string;
  readonly half: number;
};

type RowFix = { readonly key: string; readonly row: string };

type Auto = {
  readonly merges: readonly { readonly slug: string; readonly newRows: readonly string[] }[];
  readonly copySplits: readonly RowFix[];
  readonly strays: readonly RowFix[];
  readonly strayQueue: readonly string[];
  readonly splitQueue: readonly RowFix[];
  readonly unclearQueue: readonly string[];
};

type Score = {
  readonly total: number;
  readonly answered: number;
  readonly exact: number;
  readonly unsure: number;
};

function readCorpus(text: string): Poem[] {
  const poems: Poem[] = [];
  for (const line of text.split('\n')) {
    const fields = line.split('\t');
    if (fields.length < 6) continue;
    const [id = '', slug = '', type = '', meter = ''] = fields;
    poems.push({ id, slug, type, meter, rows: fields.slice(5).join('\t').split('|') });
  }
  return poems;
}

function median(values: readonly number[]): number | undefined {
  if (values.length === 0) return undefined;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}

const allClean = (poem: Poem): boolean => poem.rows.every((row) => isFullVerse(row));
const rowKey = (poem: Poem, index: number): string => `${poem.slug}:${index + 1}`;

function meterBaselines(poems: readonly Poem[]): Map<string, number> {
  const halves = new Map<string, number[]>();
  for (const poem of poems) {
    if (!allClean(poem)) continue;
    const list = halves.get(poem.meter) ?? [];
    for (const row of poem.rows) for (const part of row.split('*')) list.push(letters(part));
    halves.set(poem.meter, list);
  }
  const baselines = new Map<string, number>();
  for (const [meter, list] of halves) {
    const value = median(list);
    if (value !== undefined) baselines.set(meter, value);
  }
  return baselines;
}

const stableOrder = (text: string): number => Bun.hash.crc32(`${SEED}:${text}`);

function lastWord(row: string): string {
  const words = row
    .trim()
    .split(/\s+/)
    .filter((token) => letters(token) > 0);
  return words.at(-1) ?? '';
}

async function writeBatches(
  dir: string,
  prefix: string,
  header: string,
  lines: readonly string[]
): Promise<number> {
  let count = 0;
  for (let start = 0; start < lines.length; start += BATCH) {
    count += 1;
    const name = `${prefix}-${String(count).padStart(3, '0')}.csv`;
    await Bun.write(
      join(dir, name),
      `${header}\n${lines.slice(start, start + BATCH).join('\n')}\n`
    );
  }
  return count;
}

async function readAnswers(dir: string, prefix: string): Promise<Map<string, string>> {
  const pattern = new RegExp(`^${prefix}-\\d+-answers\\.csv$`);
  const answers = new Map<string, string>();
  for (const name of (await readdir(dir))
    .filter((file) => pattern.test(file))
    .sort((a, b) => a.localeCompare(b))) {
    for (const [key, answer] of parseAnswers(await Bun.file(join(dir, name)).text()))
      answers.set(key, answer);
  }
  return answers;
}

const hasClassicalMeter = (poem: Poem): boolean => CLASSICAL_METERS.has(poem.meter);

function classical(poems: readonly Poem[]): { poems: Poem[]; baselines: Map<string, number> } {
  const kept = poems.filter(
    (poem) => poem.type === CLASSICAL || (poem.type === UNKNOWN_TYPE && hasClassicalMeter(poem))
  );
  return { poems: kept, baselines: meterBaselines(kept.filter((poem) => hasClassicalMeter(poem))) };
}

async function writeMeasureSample(dir: string, poems: readonly Poem[]): Promise<number> {
  const byMeter = new Map<string, { poem: Poem; index: number; half: number }[]>();
  const ordered = poems
    .filter((each) => allClean(each))
    .sort(
      (a, b) =>
        stableOrder(`measure-${MEASURE_ROUND}:${a.slug}`) -
        stableOrder(`measure-${MEASURE_ROUND}:${b.slug}`)
    );
  for (const poem of ordered) {
    const half = typicalHalf(poem.rows);
    if (half === undefined) continue;
    const list = byMeter.get(poem.meter) ?? [];
    if (list.length < MEASURE_SIZE) {
      const index = stableOrder(`measure-${MEASURE_ROUND}:${poem.slug}:row`) % poem.rows.length;
      list.push({ poem, index, half });
    }
    byMeter.set(poem.meter, list);
  }
  const measured: { poem: Poem; index: number; half: number }[] = [];
  for (let round = 0; measured.length < MEASURE_SIZE && round < MEASURE_SIZE; round += 1) {
    for (const list of byMeter.values()) {
      const pick = list[round];
      if (pick !== undefined && measured.length < MEASURE_SIZE) measured.push(pick);
    }
  }

  const measureLines: string[] = [];
  const keyLines: string[] = [];
  for (const { poem, index, half } of measured) {
    const [first = '', second = ''] = (poem.rows[index] ?? '').split('*');
    const text = `${first.trim()} ${second.trim()}`;
    const key = rowKey(poem, index);
    measureLines.push(csvLine([key, poem.meter, String(half), numberedWords(text)]));
    keyLines.push(csvLine([key, String(letters(first)), text]));
  }

  await writeBatches(dir, 'measure', 'key,meter,half,words', measureLines);
  await Bun.write(join(dir, 'measure-key.csv'), `key,cut,text\n${keyLines.join('\n')}\n`);
  return measured.length;
}

async function previousAnswers(
  previousDir: string | undefined
): Promise<{ splits: Map<string, string>; unclear: Map<string, string> }> {
  if (previousDir === undefined) return { splits: new Map(), unclear: new Map() };
  return {
    splits: await readAnswers(previousDir, 'splits'),
    unclear: await readAnswers(previousDir, 'unclear'),
  };
}

async function plan(corpusPath: string, dir: string, previousDir?: string): Promise<void> {
  const everything = readCorpus(await Bun.file(corpusPath).text());
  const { poems, baselines } = classical(everything);
  const previous = await previousAnswers(previousDir);
  await mkdir(dir, { recursive: true });

  const merges: { slug: string; newRows: readonly string[] }[] = [];
  const unclear: { poem: Poem; half: number }[] = [];
  const candidates: Candidate[] = [];
  const strays: RowFix[] = [];
  const strayReview: string[] = [];
  const strayQueue: string[] = [];
  const longLines: string[] = [];
  const counts = new Map<string, number>();
  const bump = (name: string, by = 1): void => {
    counts.set(name, (counts.get(name) ?? 0) + by);
  };

  for (const poem of poems) {
    const reference = referenceHalf(poem.rows, baselines.get(poem.meter));
    if (reference === undefined) {
      bump('no typical half');
      continue;
    }
    const { half } = reference;
    const decision = planPoem(poem.rows, reference);
    bump(`plan: ${decision.kind}`);
    if (decision.kind === 'merge') merges.push({ slug: poem.slug, newRows: decision.newRows });
    if (decision.kind === 'unclear') {
      if (previous.unclear.has(poem.slug)) bump('unclear: reviewed in #186, skipped');
      else unclear.push({ poem, half });
    }
    if (decision.kind !== 'rows') continue;

    const inLongRuns = indexesInLongRuns(decision.stray);
    for (const index of decision.stray) {
      const row = poem.rows[index] ?? '';
      const joined = joinSplit(row);
      if (joined !== undefined && !inLongRuns.has(index) && isConfidentStray(row, poem.meter)) {
        strays.push({ key: rowKey(poem, index), row: joined });
      } else {
        strayQueue.push(rowKey(poem, index));
        strayReview.push(csvLine([rowKey(poem, index), poem.meter, String(half), row]));
      }
    }
    for (const index of decision.long) {
      const row = poem.rows[index] ?? '';
      longLines.push(
        csvLine([rowKey(poem, index), poem.meter, (letters(row) / half).toFixed(2), row])
      );
    }
    const toSplit = [
      ...decision.unsplit.map((index) => ({ index, row: poem.rows[index] })),
      ...decision.misplaced.map((index) => ({ index, row: joinSplit(poem.rows[index] ?? '') })),
    ];
    for (const { index, row } of toSplit) {
      const key = rowKey(poem, index);
      if (previous.splits.has(key)) bump('split: reviewed in #186, skipped');
      else if (row === undefined) bump('split: misplaced next to a one-letter piece, skipped');
      else candidates.push({ key, poem, row, half });
    }
    bump('stray rows', decision.stray.length);
    bump('misplaced rows', decision.misplaced.length);
    bump('unsplit rows', decision.unsplit.length);
    bump('long rows, listed only', decision.long.length);
  }

  const wanted = new Set(candidates.map((candidate) => foldKey(candidate.row)));
  const copies = copyCuts(
    everything.flatMap((poem) =>
      poem.rows.filter((row) => isFullVerse(row) && wanted.has(foldKey(row)))
    )
  );

  const copySplits: RowFix[] = [];
  const queue: Candidate[] = [];
  for (const candidate of candidates) {
    const cut = copies.get(foldKey(candidate.row));
    const repaired = cut === undefined ? undefined : acceptSplit(candidate.row, cut);
    if (repaired === undefined) {
      queue.push(candidate);
      bump(cut === undefined ? 'split: no copy, queued' : 'split: copy failed the checks, queued');
    } else {
      copySplits.push({ key: candidate.key, row: repaired });
      bump('split: taken from a copy');
    }
  }

  const splitLines = queue.map((candidate) =>
    csvLine([
      candidate.key,
      candidate.poem.meter,
      String(candidate.half),
      numberedWords(candidate.row),
    ])
  );
  const unclearLines = unclear.map(({ poem }) =>
    csvLine([
      poem.slug,
      poem.meter,
      String(poem.rows.length),
      poem.rows.map((row) => `${letters(row)}:${lastWord(row)}`).join(' | '),
    ])
  );
  const bySlug = new Map(poems.map((poem) => [poem.slug, poem]));
  const strayLine = ({ key, row }: RowFix): string => {
    const [slug = '', position = '0'] = key.split(':');
    return csvLine([key, bySlug.get(slug)?.rows[Number(position) - 1] ?? '', row]);
  };
  const sample = [...strays]
    .sort(
      (a, b) =>
        stableOrder(`stray-${STRAY_SAMPLE_ROUND}:${a.key}`) -
        stableOrder(`stray-${STRAY_SAMPLE_ROUND}:${b.key}`)
    )
    .slice(0, STRAY_SAMPLE_SIZE);

  const splitBatches = await writeBatches(dir, 'splits', 'key,meter,half,words', splitLines);
  const unclearBatches = await writeBatches(dir, 'unclear', 'slug,meter,rows,lines', unclearLines);
  await writeBatches(
    dir,
    'strays-sample',
    'key,stored,joined',
    sample.map((fix) => strayLine(fix))
  );
  await writeBatches(dir, 'strays-review', 'key,meter,half,row', strayReview);
  await Bun.write(join(dir, 'long.csv'), `key,meter,size,row\n${longLines.join('\n')}\n`);
  const keyFile = Bun.file(join(dir, 'measure-key.csv'));
  const measureNote = (await keyFile.exists())
    ? 'kept the existing measurement sample, which its answers refer to'
    : `${await writeMeasureSample(dir, poems)} verses`;

  const auto: Auto = {
    merges,
    copySplits,
    strays,
    strayQueue,
    splitQueue: queue.map(({ key, row }) => ({ key, row })),
    unclearQueue: unclear.map(({ poem }) => poem.slug),
  };
  await Bun.write(join(dir, 'auto.json'), `${JSON.stringify(auto)}\n`);

  const summary = [
    '# Verse row repair: plan',
    '',
    `Classical poems: ${poems.length}`,
    ...[...counts]
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([name, value]) => `- ${name}: ${value}`),
    `- strays: ${strays.length} joined automatically, ${strayReview.length} left for review`,
    '',
    `Review batches: measure (${measureNote}), ${splitBatches} splits (${queue.length} rows), ${unclearBatches} unclear (${unclear.length} poems), stray sample (${sample.length} rows)`,
  ];
  await Bun.write(join(dir, 'summary.md'), `${summary.join('\n')}\n`);
  console.log(summary.join('\n'));
}

function parseKeyFile(text: string): { rows: Map<string, string>; truth: Map<string, number> } {
  const rows = new Map<string, string>();
  const truth = new Map<string, number>();
  for (const line of text.split('\n').slice(1)) {
    const match = /^([^,]+),(\d+),"?(.*?)"?$/.exec(line.trim());
    if (match === null) continue;
    const [, key = '', cut = '0', rowText = ''] = match;
    rows.set(key, rowText.replaceAll('""', '"'));
    truth.set(key, Number(cut));
  }
  return { rows, truth };
}

async function score(dir: string): Promise<void> {
  const { rows, truth } = parseKeyFile(await Bun.file(join(dir, 'measure-key.csv')).text());
  const result = scoreMeasure(rows, truth, await readAnswers(dir, 'measure'));
  const rate = result.answered === 0 ? 0 : result.exact / result.answered;
  await Bun.write(join(dir, 'score.json'), `${JSON.stringify({ ...result, rate })}\n`);
  console.log(
    `measured ${result.total}: answered ${result.answered}, exact ${result.exact} (${(rate * 100).toFixed(1)}%), unsure ${result.unsure}`
  );
}

type HandEdit = { readonly rows: readonly string[]; readonly dropped?: readonly string[] };

async function readHandEdits(dir: string): Promise<Map<string, HandEdit>> {
  const file = Bun.file(join(dir, 'hand-edits.json'));
  if (!(await file.exists())) return new Map();
  return new Map(Object.entries((await file.json()) as Record<string, HandEdit>));
}

type TitleFix = { readonly from: string; readonly to: string };

async function readTitles(dir: string): Promise<Map<string, TitleFix>> {
  const file = Bun.file(join(dir, 'titles.json'));
  if (!(await file.exists())) return new Map();
  return new Map(Object.entries((await file.json()) as Record<string, TitleFix>));
}

async function sql(corpusPath: string, dir: string): Promise<void> {
  const everyPoem = readCorpus(await Bun.file(corpusPath).text());
  const { poems } = classical(everyPoem);
  const bySlug = new Map(poems.map((poem) => [poem.slug, poem]));
  const anyBySlug = new Map(everyPoem.map((poem) => [poem.slug, poem]));
  const handEdits = await readHandEdits(dir);
  const handEdited = (key: string): boolean => handEdits.has(key.split(':')[0] ?? '');
  const raw = (await Bun.file(join(dir, 'auto.json')).json()) as Auto;
  const auto: Auto = {
    ...raw,
    merges: raw.merges.filter((merge) => !handEdits.has(merge.slug)),
    copySplits: raw.copySplits.filter((fix) => !handEdited(fix.key)),
    strays: raw.strays.filter((fix) => !handEdited(fix.key)),
    strayQueue: raw.strayQueue.filter((key) => !handEdited(key)),
    splitQueue: raw.splitQueue.filter((fix) => !handEdited(fix.key)),
    unclearQueue: raw.unclearQueue.filter((slug) => !handEdits.has(slug)),
  };
  const splitAnswers = await readAnswers(dir, 'splits');
  const unclearAnswers = await readAnswers(dir, 'unclear');
  const review: string[] = [];

  if (splitAnswers.size > 0) {
    const measured = (await Bun.file(join(dir, 'score.json'))
      .json()
      .catch(() => undefined)) as (Score & { rate: number }) | undefined;
    if (measured === undefined || measured.rate < MIN_EXACT_RATE) {
      console.error(
        `refusing review splits: the measured exact-cut rate is ${measured?.rate ?? 'missing'}, below ${MIN_EXACT_RATE}`
      );
      process.exit(1);
    }
  }

  const strayVerdicts = await readAnswers(dir, 'strays-sample');
  if (auto.strays.length > 0) {
    const judged = [...strayVerdicts.values()].filter(
      (verdict) => verdict === 'ok' || verdict === 'no'
    );
    const rate =
      judged.length === 0 ? 0 : judged.filter((verdict) => verdict === 'ok').length / judged.length;
    if (judged.length < Math.min(STRAY_SAMPLE_SIZE, auto.strays.length) || rate < MIN_EXACT_RATE) {
      console.error(
        `refusing stray joins: ${judged.length} sample rows judged, ${(rate * 100).toFixed(1)}% ok, below ${MIN_EXACT_RATE * 100}%`
      );
      process.exit(1);
    }
  }

  const splits = new Map<string, Map<number, string>>();
  const addSplit = (key: string, row: string): void => {
    const [slug = '', position = '0'] = key.split(':');
    const forPoem = splits.get(slug) ?? new Map<number, string>();
    forPoem.set(Number(position) - 1, row);
    splits.set(slug, forPoem);
  };
  for (const { key, row } of auto.copySplits) addSplit(key, row);
  for (const { key, row } of auto.strays) {
    if (strayVerdicts.get(key) === 'no') review.push(`- ${key}: stray join refused in the sample`);
    else addSplit(key, row);
  }
  const strayAnswers = await readAnswers(dir, 'strays-review');
  for (const key of auto.strayQueue) {
    const [slug = '', position = '0'] = key.split(':');
    const row = bySlug.get(slug)?.rows[Number(position) - 1];
    const answer = strayAnswers.get(key) ?? strayAnswers.get(slug);
    if (row === undefined) {
      review.push(`- ${key}: not in the corpus export`);
      continue;
    }
    if (answer === 'keep') continue;
    if (answer !== 'join' && answer !== 'glue') {
      review.push(`- ${key}: stray ${answer === undefined ? 'unanswered' : 'unsure'}`);
      continue;
    }
    const repaired = answer === 'join' ? joinSplit(row) : glueSplit(row);
    if (repaired === undefined) review.push(`- ${key}: stray ${answer} failed the checks`);
    else addSplit(key, repaired);
  }
  for (const { key, row } of auto.splitQueue) {
    const answer = splitAnswers.get(key);
    const [slug = ''] = key.split(':');
    const poem = bySlug.get(slug);
    if (poem === undefined) {
      review.push(`- ${key}: not in the corpus export`);
      continue;
    }
    if (answer === '-') continue;
    if (answer === 'x') {
      review.push(`- ${key}: not a verse (a note or heading stored as a row)`);
      continue;
    }
    if (answer === undefined || answer === '?') {
      review.push(`- ${key}: ${answer === undefined ? 'unanswered' : 'unsure'}`);
      continue;
    }
    const cut = answerToLetters(row, answer);
    const repaired = cut === undefined ? undefined : acceptSplit(row, cut);
    if (repaired === undefined) review.push(`- ${key}: answer ${answer} failed the checks`);
    else addSplit(key, repaired);
  }

  const merges = new Map(auto.merges.map((merge) => [merge.slug, merge.newRows]));
  for (const slug of auto.unclearQueue) {
    const answer = unclearAnswers.get(slug);
    if (answer === 'single-lines') continue;
    if (answer !== 'couplets') {
      review.push(`- ${slug}: ${answer === undefined ? 'unanswered' : `answer ${answer}`}`);
      continue;
    }
    const rows = bySlug.get(slug)?.rows ?? [];
    const merged = acceptCouplets(rows);
    if (merged === undefined)
      review.push(`- ${slug}: couplets refused, the merged second halves do not rhyme`);
    else merges.set(slug, merged);
  }

  const changes: Change[] = [];
  for (const [slug, forPoem] of splits) {
    const poem = bySlug.get(slug);
    if (poem === undefined) continue;
    if (merges.has(slug)) throw new Error(`refusing ${slug}: it is both split and merged`);
    changes.push({
      poemId: poem.id,
      slug,
      oldRows: poem.rows,
      newRows: applySplits(poem.rows, forPoem),
      reason: 'rows',
    });
  }
  for (const [slug, newRows] of merges) {
    const poem = bySlug.get(slug);
    if (poem !== undefined)
      changes.push({ poemId: poem.id, slug, oldRows: poem.rows, newRows, reason: 'merge' });
  }
  for (const [slug, edit] of handEdits) {
    const poem = anyBySlug.get(slug);
    if (poem === undefined) review.push(`- ${slug}: hand edit not in the corpus export`);
    else
      changes.push({
        poemId: poem.id,
        slug,
        oldRows: poem.rows,
        newRows: edit.rows,
        dropped: edit.dropped ?? [],
        reason: 'hand',
      });
  }

  const titles: TitleChange[] = [];
  for (const [slug, fix] of await readTitles(dir)) {
    const poem = anyBySlug.get(slug);
    if (poem === undefined) review.push(`- ${slug}: title fix not in the corpus export`);
    else titles.push({ poemId: poem.id, slug, from: fix.from, to: fix.to });
  }

  await Bun.write(join(dir, 'apply.sql'), buildApplySql(changes, titles));
  await Bun.write(join(dir, 'plan.json'), `${JSON.stringify(changes)}\n`);
  await Bun.write(join(dir, 'review.md'), `# Not applied\n\n${review.join('\n')}\n`);
  console.log(
    `${changes.length} poems changed (${changes.filter((change) => change.reason === 'rows').length} with row fixes, ${changes.filter((change) => change.reason === 'merge').length} merged, ${changes.filter((change) => change.reason === 'hand').length} edited by hand), ${titles.length} titles; ${review.length} listed for review`
  );
}

const [mode, first, second, third] = Bun.argv.slice(2);
if (mode === 'plan' && first !== undefined) await plan(first, second ?? DEFAULT_DIR, third);
else if (mode === 'score') await score(first ?? DEFAULT_DIR);
else if (mode === 'sql' && first !== undefined) await sql(first, second ?? DEFAULT_DIR);
else {
  console.error(
    'usage: repair-rows.ts plan <corpus.tsv> [dir] [previous-dir] | score [dir] | sql <corpus.tsv> [dir]'
  );
  process.exit(1);
}
