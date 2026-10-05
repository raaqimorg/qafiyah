#!/usr/bin/env bun

import { spawnSync } from 'node:child_process';

const args = process.argv.slice(2);

const ROOT = `${import.meta.dir}/../..`;
const PLACEHOLDER = 'build-check-not-a-real-secret';

const env = {
  ...process.env,
  POSTGRES_PASSWORD: process.env['POSTGRES_PASSWORD'] ?? PLACEHOLDER,
  ELASTIC_PASSWORD: process.env['ELASTIC_PASSWORD'] ?? PLACEHOLDER,
  ES_READER_PASSWORD: process.env['ES_READER_PASSWORD'] ?? PLACEHOLDER,
  PG_READER_PASSWORD: process.env['PG_READER_PASSWORD'] ?? PLACEHOLDER,
  PG_MONITOR_PASSWORD: process.env['PG_MONITOR_PASSWORD'] ?? PLACEHOLDER,
  ES_MONITOR_PASSWORD: process.env['ES_MONITOR_PASSWORD'] ?? PLACEHOLDER,
};

const config = spawnSync(`${ROOT}/scripts/dev/compose.sh`, ['config', '--format', 'json'], {
  cwd: ROOT,
  env,
  encoding: 'utf8',
});
if (config.status !== 0) {
  console.error(config.stderr || 'compose could not read the configuration');
  process.exit(config.status ?? 1);
}
const services = Object.entries(
  (JSON.parse(config.stdout) as { services: Record<string, { build?: unknown }> }).services
)
  .filter(([, service]) => service.build)
  .map(([name]) => name)
  .sort();
if (services.length === 0) {
  console.error('compose reports no service with a build section');
  process.exit(1);
}

if (args.includes('--list')) {
  console.log(JSON.stringify(services));
  process.exit(0);
}

const requested = args.filter((arg) => !arg.startsWith('--'));
const unknown = requested.filter((name) => !services.includes(name));
if (unknown.length > 0) {
  console.error(
    `compose reports no buildable service named ${unknown.join(', ')}. It has: ${services.join(', ')}.`
  );
  process.exit(1);
}
const targets = requested.length > 0 ? requested : services;

for (const service of targets) {
  console.log(`\n── building ${service} ──`);
  const result = spawnSync(`${ROOT}/scripts/dev/compose.sh`, ['build', service], {
    cwd: ROOT,
    stdio: 'inherit',
    env,
  });
  if (result.status !== 0) {
    console.error(`\nThe ${service} image does not build.`);
    process.exit(result.status ?? 1);
  }
}

console.log(
  targets.length === services.length
    ? `\nEvery deployable image builds: ${targets.join(', ')}.`
    : `\nBuilds: ${targets.join(', ')}.`
);
