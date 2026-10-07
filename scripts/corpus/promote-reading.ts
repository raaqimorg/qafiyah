#!/usr/bin/env bun

import { MUALLAQAT_PRIMARIES, promotionSql, type Reading, type Verse } from './reading-promotion';

const SOURCE = 'رواية الأنباري، بضبط فيصل المنصور';

const [input, output] = Bun.argv.slice(2);
if (input === undefined || output === undefined) {
  console.error('usage: promote-reading.ts <transcription.json> <out.sql>');
  process.exit(1);
}

const poems = (await Bun.file(input).json()) as Record<string, readonly Verse[]>;
const keys = Object.keys(poems);
const known = Object.keys(MUALLAQAT_PRIMARIES);
if (keys.length !== known.length || !known.every((key) => keys.includes(key))) {
  console.error(`expected exactly these poems: ${known.join(', ')}; found: ${keys.join(', ')}`);
  process.exit(1);
}
const readings: Reading[] = Object.entries(MUALLAQAT_PRIMARIES).map(([key, primarySlug]) => ({
  primarySlug,
  verses: (poems[key] ?? []).map(({ n, sadr, ajuz }) => ({ n, sadr, ajuz })),
}));
await Bun.write(output, promotionSql(readings, SOURCE));
console.log(
  `${readings.length} readings, ${readings.reduce((sum, reading) => sum + reading.verses.length, 0)} verses, written to ${output}`
);
