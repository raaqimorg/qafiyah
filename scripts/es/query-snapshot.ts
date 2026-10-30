#!/usr/bin/env bun

import { cargoDump } from '../lib/cargo';
import { snapshot } from '../lib/snapshot';

const OUT = 'apps/api/generated/es/query.vectors.json';

const dumped = cargoDump('es-query-dump');
if (dumped.isErr()) {
  console.error(dumped.error);
  process.exit(1);
}

const rendered = dumped.value;
const vectors = JSON.parse(rendered) as {
  poems: unknown[];
  poets: unknown[];
  poetList: unknown[];
};

process.exit(
  await snapshot({
    out: OUT,
    rendered,
    summary: `${vectors.poems.length} poem, ${vectors.poets.length} poet search, and ${vectors.poetList.length} poet list bodies`,
    remedy: [
      'The Elasticsearch query builders changed. Run `bun run es:query:snapshot`',
      'and read the diff before committing it.',
    ],
    check: process.argv.includes('--check'),
  })
);
