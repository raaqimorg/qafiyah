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
  isFullVerse,
  letters,
  numberedWords,
  parseAnswers,
  planPoem,
  scoreMeasure,
  typicalHalf,
  type Change,
} from './row-repair';

const DEFAULT_DIR = 'reports/corpus/row-repair';
const CLASSICAL = 'amudi';
const BATCH = 100;
const MEASURE_SIZE = 300;
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
  readonly index: number;
  readonly half: number;
};

type Auto = {
  readonly merges: readonly { readonly slug: string; readonly newRows: readonly string[] }[];
  readonly copySplits: readonly { readonly key: string; readonly row: string }[];
  readonly splitQueue: readonly string[];
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

function classical(poems: readonly Poem[]): { poems: Poem[]; baselines: Map<string, number> } {
  const kept = poems.filter((poem) => poem.type === CLASSICAL);
  return { poems: kept, baselines: meterBaselines(kept) };
}

async function writeMeasureSample(dir: string, poems: readonly Poem[]): Promise<number> {
  const byMeter = new Map<string, { poem: Poem; index: number; half: number }[]>();
  const ordered = poems
    .filter((each) => allClean(each))
    .sort((a, b) => stableOrder(a.slug) - stableOrder(b.slug));
  for (const poem of ordered) {
    const half = typicalHalf(poem.rows);
    if (half === undefined) continue;
    const list = byMeter.get(poem.meter) ?? [];
    if (list.length < MEASURE_SIZE) {
      list.push({ poem, index: stableOrder(`${poem.slug}:row`) % poem.rows.length, half });
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

async function plan(corpusPath: string, dir: string): Promise<void> {
  const everything = readCorpus(await Bun.file(corpusPath).text());
  const { poems, baselines } = classical(everything);
  await mkdir(dir, { recursive: true });

  const merges: { slug: string; newRows: readonly string[] }[] = [];
  const unclear: { poem: Poem; half: number }[] = [];
  const candidates: Candidate[] = [];
  const counts = new Map<string, number>();
  const bump = (name: string): void => {
    counts.set(name, (counts.get(name) ?? 0) + 1);
  };

  for (const poem of poems) {
    const half = typicalHalf(poem.rows, baselines.get(poem.meter));
    if (half === undefined) {
      bump('no typical half');
      continue;
    }
    const decision = planPoem(poem.rows, half);
    bump(`plan: ${decision.kind}`);
    if (decision.kind === 'merge') merges.push({ slug: poem.slug, newRows: decision.newRows });
    if (decision.kind === 'unclear') unclear.push({ poem, half });
    if (decision.kind === 'splits') {
      for (const index of decision.indexes)
        candidates.push({ key: rowKey(poem, index), poem, index, half });
    }
  }

  const wanted = new Set(
    candidates.map((candidate) => foldKey(candidate.poem.rows[candidate.index] ?? ''))
  );
  const copies = copyCuts(
    everything.flatMap((poem) =>
      poem.rows.filter((row) => isFullVerse(row) && wanted.has(foldKey(row)))
    )
  );

  const copySplits: { key: string; row: string }[] = [];
  const queue: Candidate[] = [];
  for (const candidate of candidates) {
    const row = candidate.poem.rows[candidate.index] ?? '';
    const cut = copies.get(foldKey(row));
    const repaired = cut === undefined ? undefined : acceptSplit(row, cut, candidate.half);
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
      numberedWords(candidate.poem.rows[candidate.index] ?? ''),
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

  const splitBatches = await writeBatches(dir, 'splits', 'key,meter,half,words', splitLines);
  const unclearBatches = await writeBatches(dir, 'unclear', 'slug,meter,rows,lines', unclearLines);
  const keyFile = Bun.file(join(dir, 'measure-key.csv'));
  const measureNote = (await keyFile.exists())
    ? 'kept the existing measurement sample, which its answers refer to'
    : `${await writeMeasureSample(dir, poems)} verses`;

  const auto: Auto = {
    merges,
    copySplits,
    splitQueue: queue.map((candidate) => candidate.key),
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
    '',
    `Review batches: measure (${measureNote}), ${splitBatches} splits (${queue.length} rows), ${unclearBatches} unclear (${unclear.length} poems)`,
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

async function sql(corpusPath: string, dir: string): Promise<void> {
  const { poems, baselines } = classical(readCorpus(await Bun.file(corpusPath).text()));
  const bySlug = new Map(poems.map((poem) => [poem.slug, poem]));
  const auto = (await Bun.file(join(dir, 'auto.json')).json()) as Auto;
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

  const splits = new Map<string, Map<number, string>>();
  const addSplit = (key: string, row: string): void => {
    const [slug = '', position = '0'] = key.split(':');
    const forPoem = splits.get(slug) ?? new Map<number, string>();
    forPoem.set(Number(position) - 1, row);
    splits.set(slug, forPoem);
  };
  for (const { key, row } of auto.copySplits) addSplit(key, row);
  for (const key of auto.splitQueue) {
    const answer = splitAnswers.get(key);
    const [slug = '', position = '0'] = key.split(':');
    const poem = bySlug.get(slug);
    const row = poem?.rows[Number(position) - 1];
    if (poem === undefined || row === undefined) {
      review.push(`- ${key}: not in the corpus export`);
      continue;
    }
    if (answer === 'x') {
      review.push(`- ${key}: not a verse (a note or heading stored as a row)`);
      continue;
    }
    if (answer === undefined || answer === '?') {
      review.push(`- ${key}: ${answer === undefined ? 'unanswered' : 'unsure'}`);
      continue;
    }
    const half = typicalHalf(poem.rows, baselines.get(poem.meter)) ?? 0;
    const cut = answerToLetters(row, answer);
    const repaired = cut === undefined ? undefined : acceptSplit(row, cut, half);
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
      reason: 'split',
    });
  }
  for (const [slug, newRows] of merges) {
    const poem = bySlug.get(slug);
    if (poem !== undefined)
      changes.push({ poemId: poem.id, slug, oldRows: poem.rows, newRows, reason: 'merge' });
  }

  await Bun.write(join(dir, 'apply.sql'), buildApplySql(changes));
  await Bun.write(join(dir, 'plan.json'), `${JSON.stringify(changes)}\n`);
  await Bun.write(join(dir, 'review.md'), `# Not applied\n\n${review.join('\n')}\n`);
  console.log(
    `${changes.length} poems changed (${changes.filter((change) => change.reason === 'split').length} split, ${changes.filter((change) => change.reason === 'merge').length} merged); ${review.length} listed for review`
  );
}

const [mode, first, second] = Bun.argv.slice(2);
if (mode === 'plan' && first !== undefined) await plan(first, second ?? DEFAULT_DIR);
else if (mode === 'score') await score(first ?? DEFAULT_DIR);
else if (mode === 'sql' && first !== undefined) await sql(first, second ?? DEFAULT_DIR);
else {
  console.error(
    'usage: repair-rows.ts plan <corpus.tsv> [dir] | score [dir] | sql <corpus.tsv> [dir]'
  );
  process.exit(1);
}
