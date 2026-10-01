import { describe, expect, test } from 'bun:test';

import {
  cargoProgress,
  composeProgress,
  elapsed,
  fit,
  imageBuildProgress,
  indexerProgress,
  readLines,
} from './progress';

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

  test("shows the database's init phase from its log, shortening paths", () => {
    const progress = composeProgress();
    progress(' Container qafiyah-dev-es Healthy ');
    expect(progress('[db-init] restoring /tmp/qafiyah_public.dump into qafiyah...')).toBe(
      'es healthy · db restoring qafiyah_public.dump into qafiyah'
    );
    expect(
      progress('qafiyah-dev-db  | [db-init] filling random_poem_pool from the restored data...')
    ).toBe('es healthy · db filling random_poem_pool from the restored data');
    progress(' Container qafiyah-dev-db Healthy ');
    expect(progress('[db-init] provisioning read-only role qafiyah_api...')).toBe(
      'es healthy · db healthy'
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

describe('imageBuildProgress', () => {
  test('shows the build step of the image being built', () => {
    expect(
      imageBuildProgress('#14 [search-indexer build  4/10] COPY Cargo.toml Cargo.lock ./')
    ).toBe('building image, build step 4/10');
    expect(imageBuildProgress('#21 [search-indexer runtime 2/3] COPY --from=build')).toBe(
      'building image, runtime step 2/3'
    );
  });

  test('ignores build lines without a step', () => {
    expect(imageBuildProgress('#3 [search-indexer internal] load .dockerignore')).toBeUndefined();
    expect(imageBuildProgress('#14 DONE 0.1s')).toBeUndefined();
  });
});

describe('cargoProgress', () => {
  test('names the crate being compiled and counts the ones started so far', () => {
    const progress = cargoProgress();
    expect(progress('   Compiling proc-macro2 v1.0.95')).toBe('compiling proc-macro2 · 1 crate');
    expect(progress('   Compiling sqlx-postgres v0.8.6')).toBe(
      'compiling sqlx-postgres · 2 crates'
    );
    expect(
      progress('    Finished `dev` profile [unoptimized + debuginfo] target(s) in 52s')
    ).toBeUndefined();
  });
});

describe('fit', () => {
  test('keeps text that fits and cuts longer text to the width with an ellipsis', () => {
    expect(fit('poems 10,000 / 348,691', 40)).toBe('poems 10,000 / 348,691');
    expect(fit('db ensuring the taxonomy stats tables exist', 20)).toBe('db ensuring the tax…');
    expect(fit('anything', 0)).toBe('');
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
  test('hands over whole lines even when a chunk ends mid-line', async () => {
    const chunks = ['first li', 'ne\nsecond\nthi', 'rd'];
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        for (const chunk of chunks) controller.enqueue(new TextEncoder().encode(chunk));
        controller.close();
      },
    });
    const lines: string[] = [];
    await readLines(stream, (line) => lines.push(line));
    expect(lines).toEqual(['first line', 'second', 'third']);
  });

  test('drops the carriage return of a CRLF line', async () => {
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(new TextEncoder().encode('one\r\ntwo\r\n'));
        controller.close();
      },
    });
    const lines: string[] = [];
    await readLines(stream, (line) => lines.push(line));
    expect(lines).toEqual(['one', 'two']);
  });
});
