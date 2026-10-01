import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { analyzeBayt } from '../scripts/analyze-bayt.js';

const readJson = (name) =>
  JSON.parse(readFileSync(new URL(`./fixtures/${name}`, import.meta.url), 'utf8'));

const fixture = readJson('sample-100-bayt.json');
const golden = readJson('dart-golden-100-bayt.json');

test('the golden file covers every bayt of the fixture in the same order', () => {
  assert.deepEqual(
    golden.map((entry) => entry.verseId),
    fixture.map((item) => item.verseId)
  );
});

for (const [index, item] of fixture.entries()) {
  test(`bayt ${item.verseId} (${item.poemSlug}) produces exactly what the Dart engine produced`, () => {
    assert.deepEqual({ verseId: item.verseId, ...analyzeBayt(item.content) }, golden[index]);
  });
}
