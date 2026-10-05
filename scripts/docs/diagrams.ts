#!/usr/bin/env bun

import { mkdtemp, readdir, rm, unlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { ROOT } from '../lib/root';
import { snapshot } from '../lib/snapshot';

import { generatedName, stale, withHeader } from './diagram-files';

const STRUCTURIZR_IMAGE = 'structurizr/structurizr:2026.09.19';
const PLANTUML_IMAGE = 'plantuml/plantuml:1.2026.8';
const SOURCE_DIR = join(ROOT, 'docs/architecture');
const LEGEND = 'legend.puml';
const OUT_DIR = 'docs/architecture/generated/structurizr';
const REMEDY = ['Run: bun run docs:diagrams, then review the diff.'];
const CALLER = `${process.getuid?.() ?? 0}:${process.getgid?.() ?? 0}`;

const check = process.argv.includes('--check');

async function run(cmd: readonly string[]): Promise<void> {
  const proc = Bun.spawn([...cmd], { stdout: 'pipe', stderr: 'pipe' });
  const [out, err, code] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ]);
  if (code !== 0) throw new Error(`${cmd.slice(0, 5).join(' ')} exited ${code}\n${out}${err}`);
}

async function listed(dir: string): Promise<string[]> {
  return await readdir(dir).catch(() => []);
}

async function render(work: string): Promise<Map<string, string>> {
  await run([
    'docker',
    'run',
    '--rm',
    '--user',
    CALLER,
    '-v',
    `${SOURCE_DIR}:/source:ro`,
    '-v',
    `${work}:/work`,
    STRUCTURIZR_IMAGE,
    'export',
    '-w',
    '/source/workspace.dsl',
    '-f',
    'plantuml/c4plantuml',
    '-o',
    '/work',
  ]);
  await Bun.write(join(work, LEGEND), Bun.file(join(SOURCE_DIR, LEGEND)));
  await run([
    'docker',
    'run',
    '--rm',
    '--user',
    CALLER,
    '-v',
    `${work}:/work`,
    PLANTUML_IMAGE,
    '-tsvg',
    '-failfast2',
    '-o',
    '/work',
    '/work/structurizr-*.puml',
  ]);
  const rendered = new Map<string, string>();
  for (const file of (await listed(work)).sort((a, b) => a.localeCompare(b))) {
    const name = generatedName(file);
    if (name !== undefined) rendered.set(name, withHeader(await Bun.file(join(work, file)).text()));
  }
  if (rendered.size === 0) throw new Error('The export produced no diagrams.');
  return rendered;
}

async function settle(rendered: Map<string, string>): Promise<number> {
  let failed = 0;
  for (const [name, svg] of rendered) {
    failed += await snapshot({
      out: join(OUT_DIR, name),
      rendered: svg,
      summary: `the ${name.replace('.gen.svg', '')} view of docs/architecture/workspace.dsl`,
      remedy: REMEDY,
      check,
    });
  }
  for (const name of stale(await listed(OUT_DIR), [...rendered.keys()])) {
    const path = join(OUT_DIR, name);
    if (check) {
      console.error([`${path} is no longer a view of the model.`, '', ...REMEDY, ''].join('\n'));
      failed += 1;
    } else {
      await unlink(path);
      console.log(`removed ${path}`);
    }
  }
  return failed;
}

const work = await mkdtemp(join(tmpdir(), 'qafiyah-diagrams-'));
try {
  process.exitCode = (await settle(await render(work))) === 0 ? 0 : 1;
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
} finally {
  await rm(work, { recursive: true, force: true });
}
