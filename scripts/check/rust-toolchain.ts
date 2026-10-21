#!/usr/bin/env bun

import { readFileSync } from 'node:fs';

import { ROOT } from '../lib/root';

const toolchain = readFileSync(`${ROOT}/rust-toolchain.toml`, 'utf8');
const channel = /^channel\s*=\s*"([^"]+)"/m.exec(toolchain)?.[1];
if (!channel) {
  console.error('rust-toolchain.toml has no channel');
  process.exit(1);
}

const TOOLCHAIN_KEYS = new Set(['channel', 'components', 'targets', 'profile', 'path']);

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

const manifest: unknown = Bun.TOML.parse(toolchain);
const topLevel = isRecord(manifest) ? manifest : {};
const section = isRecord(topLevel['toolchain']) ? topLevel['toolchain'] : {};
const unknownKeys = [
  ...Object.keys(topLevel).filter((key) => key !== 'toolchain'),
  ...Object.keys(section)
    .filter((key) => !TOOLCHAIN_KEYS.has(key))
    .map((key) => `toolchain.${key}`),
];
if (unknownKeys.length > 0) {
  console.error(`rust-toolchain.toml has keys rustup ignores silently: ${unknownKeys.join(', ')}`);
  process.exit(1);
}

const DOCKERFILES = [
  'apps/api/Dockerfile',
  'apps/mcp/Dockerfile',
  'apps/search-indexer/Dockerfile',
];
const drift: string[] = [];

for (const file of DOCKERFILES) {
  const content = readFileSync(`${ROOT}/${file}`, 'utf8');
  const tag = /^FROM rust:([^\s-]+)/m.exec(content)?.[1];
  if (!tag) {
    drift.push(`${file}: no "FROM rust:<version>" line`);
    continue;
  }
  if (!(channel === tag || channel.startsWith(`${tag}.`))) {
    drift.push(`${file}: builds with rust ${tag}, but rust-toolchain.toml pins ${channel}`);
  }
}

if (drift.length > 0) {
  console.error('Rust versions disagree:\n');
  for (const line of drift) console.error(`  ${line}`);
  console.error('\nUpdate the Dockerfile tag and rust-toolchain.toml together.\n');
  process.exit(1);
}

console.log(`Rust ${channel} everywhere: rust-toolchain.toml and ${DOCKERFILES.length} images.`);
