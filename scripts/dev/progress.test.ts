import { describe, expect, test } from 'bun:test';

import { composeProgress, elapsed, indexerProgress, readLines } from './progress';

describe('indexerProgress', () => {
  test('turns a progress event into a count of the total', () => {
    const line =
      '{"index":"poems","indexed":120000,"source":"search-indexer","stage":"progress","total":348691}';
    expect(indexerProgress(line)).toBe('poems 120,000 / 348,691');
  });

  test('ignores every other line the indexer or compose prints', () => {
    for (const line of [
      '{"lastError":null,"lastReindexAt":null,"source":"search-indexer","stage":"done"}',
      '{"poems":{"count":3,"index":"poems_v1"},"source":"search-indexer","stage":"reindex"}',
      ' Container qafiyah-dev-search-indexer-run-1 Created',
      '{"stage":"progress","index":"poems"}',
      '',
    ]) {
      expect(indexerProgress(line)).toBeUndefined();
    }
  });
});

describe('composeProgress', () => {
  test('keeps the latest state of each container, named by its service', () => {
    const progress = composeProgress();
    expect(progress(' Container qafiyah-dev-db-0037 Waiting ')).toBe('db waiting');
    expect(progress(' Container qafiyah-dev-es Waiting ')).toBe('db waiting · es waiting');
    expect(progress(' Container qafiyah-dev-db-0037 Healthy ')).toBe('db healthy · es waiting');
  });

  test('names a worktree container by its service too', () => {
    expect(composeProgress()(' Container qafiyah-dev-qafiyah-wt-75-db Started ')).toBe(
      'db started'
    );
  });

  test('ignores lines that are not container states', () => {
    const progress = composeProgress();
    for (const line of [
      ' Network qafiyah-dev_default Created ',
      '#5 [api build 1/3] RUN cargo',
      '',
    ]) {
      expect(progress(line)).toBeUndefined();
    }
  });
});

describe('elapsed', () => {
  test('shows whole seconds under a minute and minutes past it', () => {
    expect(elapsed(0)).toBe('0s');
    expect(elapsed(5_900)).toBe('5s');
    expect(elapsed(72_000)).toBe('1m12s');
    expect(elapsed(284_000)).toBe('4m44s');
  });
});

describe('readLines', () => {
  test('hands over whole lines even when a chunk ends mid-line, and returns all the text', async () => {
    const chunks = ['first li', 'ne\nsecond\nthi', 'rd'];
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        for (const chunk of chunks) controller.enqueue(new TextEncoder().encode(chunk));
        controller.close();
      },
    });
    const lines: string[] = [];
    const text = await readLines(stream, (line) => lines.push(line));
    expect(lines).toEqual(['first line', 'second', 'third']);
    expect(text).toBe('first line\nsecond\nthird');
  });
});
