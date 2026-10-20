#!/usr/bin/env bun

import { appendFileSync, existsSync, rmSync } from 'node:fs';
import { mkdir } from 'node:fs/promises';
import { join } from 'node:path';

import { ROOT } from '../lib/root';

import { chat, isPeak, offPeakDollars, type Usage } from './deepseek';
import DESCRIPTIONS from './feeling-descriptions.json';
import { cleanVerse, feelingRows, loadSql, type VerseRow } from './feeling-load';
import { classifyMessages, verifyMessages, type Feeling } from './feeling-prompts';
import { keptTags, parseClassifyReply, parseVerifyReply, type Tag } from './feeling-replies';

const DEFAULT_DIR = 'reports/corpus/feelings';
const BATCH = 20;
const WORKERS = 16;
const ATTEMPTS = 4;
const BUDGET_DOLLARS = 15;
const PROGRESS_EVERY = 500;
const CLASSIFY_MAX_TOKENS = 8000;
const VERIFY_MAX_TOKENS = 2000;
const NO_USAGE: Usage = {
  prompt_cache_hit_tokens: 0,
  prompt_cache_miss_tokens: 0,
  completion_tokens: 0,
};

const VERSES_SQL = `
SELECT json_build_object('poemId', poem_id, 'position', position, 'content', content)
FROM (
  SELECT DISTINCT ON (pv.verse_id) pv.verse_id, pv.poem_id, pv.position, v.content
  FROM poem_verses pv
  JOIN poems p ON p.id = pv.poem_id
  JOIN poets pt ON pt.id = p.poet_id
  JOIN eras e ON e.id = p.era_id
  JOIN verses v ON v.id = pv.verse_id
  WHERE e.slug IN ('jahili', 'islami', 'umawi', 'abbasi', 'andalusi')
    AND NOT p.is_hidden AND NOT pt.is_hidden AND p.recension_of_id IS NULL
    AND v.content ~ '^[^*]*[^*[:space:]][^*]*\\*[^*]*[^*[:space:]][^*]*$'
  ORDER BY pv.verse_id, pv.poem_id, pv.position
) verse
ORDER BY verse_id`;

const FEELINGS_SQL = `
SELECT json_build_object('slug', slug, 'name', name, 'wheelTerm', coalesce(wheel_term, ''))
FROM feelings
ORDER BY id`;

type Pair = readonly [verse: string, slug: string];
type Entry<Item, Answer> = {
  readonly items: readonly Item[];
  readonly answers: readonly Answer[];
  readonly usage: Usage;
};
type Pass<Item, Answer> = {
  readonly name: string;
  readonly path: string;
  readonly ask: (items: readonly Item[]) => Promise<{ answers: Answer[]; usage: Usage }>;
};

const fail = (message: string): never => {
  console.error(message);
  process.exit(1);
};

const pairKey = ([verse, slug]: Pair): string => `${verse}\t${slug}`;

const addUsage = (a: Usage, b: Usage): Usage => ({
  prompt_cache_hit_tokens: a.prompt_cache_hit_tokens + b.prompt_cache_hit_tokens,
  prompt_cache_miss_tokens: a.prompt_cache_miss_tokens + b.prompt_cache_miss_tokens,
  completion_tokens: a.completion_tokens + b.completion_tokens,
});

function chunks<Item>(items: readonly Item[]): (readonly Item[])[] {
  return Array.from({ length: Math.ceil(items.length / BATCH) }, (_, index) =>
    items.slice(index * BATCH, (index + 1) * BATCH)
  );
}

function query(sql: string): string {
  const result = Bun.spawnSync(
    [
      './scripts/dev/compose.sh',
      'exec',
      '-T',
      'db',
      'psql',
      '-U',
      'qafiyah',
      '-d',
      'qafiyah',
      '-v',
      'ON_ERROR_STOP=1',
      '-At',
      '-c',
      sql,
    ],
    { cwd: ROOT, stderr: 'inherit' }
  );
  if (!result.success) fail('psql failed');
  return result.stdout.toString();
}

async function readLines<Row>(path: string): Promise<Row[]> {
  if (!existsSync(path)) return [];
  return (await Bun.file(path).text())
    .split('\n')
    .filter((line) => line !== '')
    .map((line) => JSON.parse(line) as Row);
}

async function readResults(path: string): Promise<Map<string, readonly Tag[]>> {
  if (!existsSync(path)) return new Map();
  return new Map(Object.entries((await Bun.file(path).json()) as Record<string, readonly Tag[]>));
}

async function readFeelings(dir: string): Promise<Feeling[]> {
  const descriptions: Readonly<Record<string, string>> = DESCRIPTIONS;
  return (await readLines<Omit<Feeling, 'description'>>(join(dir, 'feelings.jsonl'))).map(
    (feeling) => ({
      ...feeling,
      description: descriptions[feeling.slug] ?? fail(`no description for ${feeling.slug}`),
    })
  );
}

async function withRetries<Result>(attempt: () => Promise<Result>): Promise<Result> {
  let last: unknown;
  for (let tries = 1; tries <= ATTEMPTS; tries += 1) {
    try {
      return await attempt();
    } catch (error) {
      last = error;
      await Bun.sleep(2000 * tries);
    }
  }
  throw last;
}

async function runPass<Item, Answer>(
  pass: Pass<Item, Answer>,
  todo: readonly Item[],
  spent: { usage: Usage }
): Promise<boolean> {
  const jobs = chunks(todo);
  let next = 0;
  let done = 0;
  let stopped = '';
  const worker = async (): Promise<void> => {
    while (stopped === '' && next < jobs.length) {
      if (isPeak(new Date())) {
        stopped = 'peak hours started';
        return;
      }
      const items = jobs[next] ?? [];
      next += 1;
      try {
        const { answers, usage } = await withRetries(async () => await pass.ask(items));
        appendFileSync(pass.path, `${JSON.stringify({ items, answers, usage })}\n`);
        spent.usage = addUsage(spent.usage, usage);
        done += 1;
        if (offPeakDollars(spent.usage) > BUDGET_DOLLARS)
          stopped = `the budget of $${BUDGET_DOLLARS} is spent`;
        if (done % PROGRESS_EVERY === 0)
          console.log(`${pass.name}: ${done}/${jobs.length} batches`);
      } catch (error) {
        console.error(`${pass.name}: a batch failed: ${String(error)}`);
      }
    }
  };
  await Promise.all(Array.from({ length: WORKERS }, worker));
  if (stopped !== '') console.log(`${pass.name}: ${stopped}, stopping`);
  console.log(
    `${pass.name}: ${done}/${jobs.length} batches, $${offPeakDollars(spent.usage).toFixed(2)} at off-peak prices`
  );
  return done === jobs.length;
}

async function exportRows(dir: string): Promise<void> {
  await mkdir(dir, { recursive: true });
  const verses = query(VERSES_SQL);
  const feelings = query(FEELINGS_SQL);
  await Bun.write(join(dir, 'verses.jsonl'), verses);
  await Bun.write(join(dir, 'feelings.jsonl'), feelings);
  console.log(
    `exported ${verses.split('\n').filter((line) => line !== '').length} verse rows and ${feelings.split('\n').filter((line) => line !== '').length} feelings to ${dir}`
  );
}

async function classify(resultsPath: string, dir: string): Promise<void> {
  const key = process.env['DEEPSEEK_API_KEY'] ?? '';
  if (key === '') fail('set DEEPSEEK_API_KEY');
  if (isPeak(new Date())) fail('these are peak hours, run again later');
  const feelings = await readFeelings(dir);
  const slugs = new Set(feelings.map((feeling) => feeling.slug));
  const results = await readResults(resultsPath);
  const texts = [
    ...new Set(
      (await readLines<VerseRow>(join(dir, 'verses.jsonl'))).map((verse) =>
        cleanVerse(verse.content)
      )
    ),
  ].filter((text) => !results.has(text));
  if (texts.length === 0) {
    console.log(`every verse text already has a result in ${resultsPath}`);
    return;
  }
  console.log(`${texts.length} verse texts to classify`);

  const classifyPath = join(dir, 'classify.jsonl');
  const verifyPath = join(dir, 'verify.jsonl');
  const classifyEntries = async (): Promise<Entry<string, Tag[]>[]> =>
    await readLines(classifyPath);
  const verifyEntries = async (): Promise<Entry<Pair, boolean>[]> => await readLines(verifyPath);
  const spent = {
    usage: [...(await classifyEntries()), ...(await verifyEntries())].reduce(
      (sum, entry) => addUsage(sum, entry.usage),
      NO_USAGE
    ),
  };

  const tagged = new Set((await classifyEntries()).flatMap((entry) => entry.items));
  const classified = await runPass(
    {
      name: 'classify',
      path: classifyPath,
      ask: async (verses) => {
        const reply = await chat(key, {
          ...classifyMessages(feelings, verses),
          maxTokens: CLASSIFY_MAX_TOKENS,
        });
        return {
          answers: parseClassifyReply(reply.content, verses.length, slugs),
          usage: reply.usage,
        };
      },
    },
    texts.filter((text) => !tagged.has(text)),
    spent
  );
  if (!classified) fail('the classify pass is incomplete, run again to resume');

  const tags = new Map(
    (await classifyEntries()).flatMap((entry) =>
      entry.items.map((text, index) => [text, entry.answers[index] ?? []] as const)
    )
  );
  const pairs = texts.flatMap((text) =>
    (tags.get(text) ?? [])
      .filter(([, strength]) => strength === 2)
      .map(([slug]): Pair => [text, slug])
  );
  const checked = new Set(
    (await verifyEntries()).flatMap((entry) => entry.items.map((pair) => pairKey(pair)))
  );
  const verified = await runPass(
    {
      name: 'verify',
      path: verifyPath,
      ask: async (items) => {
        const reply = await chat(key, {
          ...verifyMessages(feelings, items),
          maxTokens: VERIFY_MAX_TOKENS,
        });
        return { answers: parseVerifyReply(reply.content, items.length), usage: reply.usage };
      },
    },
    pairs.filter((pair) => !checked.has(pairKey(pair))),
    spent
  );
  if (!verified) fail('the verify pass is incomplete, run again to resume');

  const confirmed = new Set(
    (await verifyEntries()).flatMap((entry) =>
      entry.items.filter((_, index) => entry.answers[index] === true).map((pair) => pairKey(pair))
    )
  );
  for (const text of texts)
    results.set(
      text,
      keptTags(tags.get(text) ?? [], (slug) => confirmed.has(pairKey([text, slug])))
    );
  await Bun.write(resultsPath, JSON.stringify(Object.fromEntries(results)));
  rmSync(classifyPath);
  rmSync(verifyPath, { force: true });
  const withFeeling = texts.filter((text) => (results.get(text) ?? []).length > 0).length;
  console.log(
    `added ${texts.length} verse texts, ${withFeeling} with a feeling, to ${resultsPath} (${results.size} in all)`
  );
}

async function writeSql(resultsPath: string, dir: string): Promise<void> {
  const feelings = new Set(
    (await readLines<Omit<Feeling, 'description'>>(join(dir, 'feelings.jsonl'))).map(
      (feeling) => feeling.slug
    )
  );
  const { rows, unclassified } = feelingRows(
    await readLines<VerseRow>(join(dir, 'verses.jsonl')),
    await readResults(resultsPath),
    feelings
  );
  const out = join(dir, 'feelings.sql');
  await Bun.write(out, loadSql(rows));
  console.log(`${rows.length} verse feelings written to ${out}`);
  if (unclassified > 0)
    console.log(`${unclassified} verse rows have no result yet, run classify to add them`);
}

const USAGE =
  'usage: classify-feelings.ts export [dir] | classify <results.json> [dir] | sql <results.json> [dir]';
const [mode, first, second] = Bun.argv.slice(2);
const MODES: Readonly<Record<string, () => Promise<void>>> = {
  export: async () => await exportRows(first ?? DEFAULT_DIR),
  classify: async () => await classify(first ?? fail(USAGE), second ?? DEFAULT_DIR),
  sql: async () => await writeSql(first ?? fail(USAGE), second ?? DEFAULT_DIR),
};
await (MODES[mode ?? ''] ?? (() => fail(USAGE)))();
