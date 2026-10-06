import { describe, expect, test } from 'bun:test';

import { POEMS_PER_PAGE } from '@qafiyah/config';

import { ROOT } from '../lib/root';

import { HALF_LINES_FIXTURE_POEM, SAMPLE_FIXTURE_POETS } from './fixtures';

type SamplePoet = { readonly slug: string; readonly poems: readonly string[] };
type SampleManifest = {
  readonly source: string;
  readonly eras: readonly string[];
  readonly poets: readonly SamplePoet[];
};

const manifest = (await Bun.file(
  `${ROOT}/data/db/0000_default/manifest.json`
).json()) as SampleManifest;

type Parameter = { readonly name: string; readonly example?: unknown };
type Spec = {
  readonly paths: Record<string, { readonly get: { readonly parameters?: readonly Parameter[] } }>;
};

const spec = (await Bun.file(`${ROOT}/apps/api/generated/openapi/openapi.json`).json()) as Spec;
const llms = await Bun.file(`${ROOT}/well-known/llms.api.md`).text();

function specExamples(wanted: (path: string, name: string) => boolean): readonly string[] {
  return Object.entries(spec.paths).flatMap(([path, item]) =>
    (item.get.parameters ?? [])
      .filter((param) => wanted(path, param.name))
      .flatMap((param) => [param.example].flat())
      .filter((example): example is string => typeof example === 'string')
  );
}

function llmsSlugs(pattern: RegExp): readonly string[] {
  return [...llms.matchAll(pattern)].flatMap((match) => (match[1] === undefined ? [] : [match[1]]));
}

const REAL_SLUG_LITERALS = [
  /POEM_DETAIL\(\s*['"`]([A-Za-z]{4})['"`]\s*\)/g,
  /\/(?:poems|poets)\/([A-Za-z]{4})(?![A-Za-z0-9])/g,
  /poetSlugs:\s*\[\s*['"`]([A-Za-z]{4})['"`]/g,
];

async function probeSources(): Promise<ReadonlyMap<string, string>> {
  const sources = new Map<string, string>();
  for await (const path of new Bun.Glob('scripts/smoke/**/*.ts').scan(ROOT)) {
    if (path.endsWith('.test.ts') || path.endsWith('/fixtures.ts')) continue;
    sources.set(path, await Bun.file(`${ROOT}/${path}`).text());
  }
  return sources;
}

describe('the committed sample dump', () => {
  test('holds every fixture poet with exactly the fixture poems, in id order', () => {
    for (const fixture of SAMPLE_FIXTURE_POETS) {
      const poet = manifest.poets.find((candidate) => candidate.slug === fixture.slug);
      expect(poet?.poems).toEqual([...fixture.poems]);
    }
  });

  test('holds the poem stored as half-lines that the verses probes read', () => {
    const poet = manifest.poets.find(
      (candidate) => candidate.slug === HALF_LINES_FIXTURE_POEM.poet
    );
    expect(poet?.poems).toContain(HALF_LINES_FIXTURE_POEM.slug);
  });

  test('stays pre-Islamic only, so it is safe to ship in plaintext', () => {
    expect(manifest.eras).toEqual(['jahili']);
  });

  test('has enough poets for the third page of the poets list', () => {
    expect(manifest.poets.length).toBeGreaterThan(2 * POEMS_PER_PAGE);
  });
});

describe('smoke probes', () => {
  test('name real poems and poets only through the fixtures the sample is built from', async () => {
    const offenders: string[] = [];
    for (const [path, source] of await probeSources()) {
      for (const pattern of REAL_SLUG_LITERALS) {
        for (const match of source.matchAll(pattern)) offenders.push(`${path}: ${match[0]}`);
      }
    }
    expect(offenders).toEqual([]);
  });
});

describe('the API docs', () => {
  test('use as examples only poems and poets the committed sample holds', () => {
    const samplePoems = new Set(manifest.poets.flatMap((poet) => poet.poems));
    const samplePoets = new Set(manifest.poets.map((poet) => poet.slug));
    const poems = [
      ...specExamples((path, name) => path === '/poems/{slug}' && name === 'slug'),
      ...llmsSlugs(/\/poems\/([A-Za-z]{4})(?![A-Za-z0-9])/g),
    ];
    const poets = [
      ...specExamples(
        (path, name) =>
          (path === '/poets/{slug}' && name === 'slug') || name === 'poet' || name === 'poetSlugs'
      ),
      ...llmsSlugs(/\/poets\/([A-Za-z]{4})(?![A-Za-z0-9])/g),
      ...llmsSlugs(/[?&]poet=([A-Za-z]{4})(?![A-Za-z0-9])/g),
    ];
    expect(poems.length).toBeGreaterThan(0);
    expect(poets.length).toBeGreaterThan(0);
    expect(poems.filter((slug) => !samplePoems.has(slug))).toEqual([]);
    expect(poets.filter((slug) => !samplePoets.has(slug))).toEqual([]);
  });
});
