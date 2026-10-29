#!/usr/bin/env bun

import { API_KEY_HEADER, API_V1_PREFIX, DEV_API_PORT, PROD_API_URL } from '@qafiyah/config';

import { ROOT } from '../lib/root';

const SCHEMATHESIS_IMAGE = 'schemathesis/schemathesis:4.29.1';

const CHECKS = [
  'not_a_server_error',
  'status_code_conformance',
  'content_type_conformance',
  'response_headers_conformance',
  'response_schema_conformance',
  'positive_data_acceptance',
];

const FUZZ_SEED = '407';

const target = process.argv.includes('prod') ? 'prod' : 'dev';
function option(name: string): string | undefined {
  return process.argv.find((arg) => arg.startsWith(`--${name}=`))?.slice(name.length + 3);
}

const fuzz = process.argv.includes('--fuzz');
const seed = option('seed') ?? FUZZ_SEED;
const maxExamples = option('max-examples');

if (fuzz && target === 'prod') {
  console.error('api:fuzz runs only against the dev stack');
  process.exit(2);
}

const config = fuzz ? 'schemathesis-fuzz.toml' : 'schemathesis.toml';
const run = fuzz
  ? [
      '--phases',
      'examples,coverage,fuzzing',
      '--mode',
      'all',
      '--checks',
      'all',
      '--seed',
      seed,
      ...(maxExamples ? ['--max-examples', maxExamples] : []),
    ]
  : ['--phases', 'examples', '--checks', CHECKS.join(',')];

const base =
  target === 'prod'
    ? `${PROD_API_URL}${API_V1_PREFIX}`
    : `http://host.docker.internal:${DEV_API_PORT}${API_V1_PREFIX}`;
const apiKey = process.env['SMOKE_API_KEY'] ?? process.env['API_KEY_FULL'];

const schemathesis = Bun.spawn(
  [
    'docker',
    'run',
    '--rm',
    '--add-host=host.docker.internal:host-gateway',
    '--volume',
    `${ROOT}/apps/api/generated/openapi/openapi.json:/spec/openapi.json:ro`,
    '--volume',
    `${ROOT}/scripts/api/${config}:/spec/schemathesis.toml:ro`,
    SCHEMATHESIS_IMAGE,
    '--config-file',
    '/spec/schemathesis.toml',
    'run',
    '/spec/openapi.json',
    '--url',
    base,
    ...run,
    ...(apiKey ? ['--header', `${API_KEY_HEADER}: ${apiKey}`] : []),
  ],
  { stdout: 'inherit', stderr: 'inherit' }
);

process.exit(await schemathesis.exited);
