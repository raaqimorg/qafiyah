import { describe, expect, test } from 'bun:test';

import { elapsed, indexerProgress, readLines } from './progress';

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
