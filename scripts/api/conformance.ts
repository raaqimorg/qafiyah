#!/usr/bin/env bun

import { API_KEY_HEADER, API_V1_PREFIX, DEV_API_PORT, PROD_API_URL } from '@qafiyah/config';

import { resolveWorktreeIdentity } from '../dev/worktree';
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

const target = process.argv.includes('prod') ? 'prod' : 'dev';

let offset = 0;
if (process.argv.includes('--worktree')) {
  const identity = await resolveWorktreeIdentity();
  if (!identity.isWorktree) {
    console.error('--worktree passed but this is the primary checkout, nothing to isolate against');
    process.exit(2);
  }
  offset = identity.offset;
}

const base =
  target === 'prod'
    ? `${PROD_API_URL}${API_V1_PREFIX}`
    : `http://host.docker.internal:${DEV_API_PORT + offset}${API_V1_PREFIX}`;
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
    `${ROOT}/scripts/api/schemathesis.toml:/spec/schemathesis.toml:ro`,
    SCHEMATHESIS_IMAGE,
    '--config-file',
    '/spec/schemathesis.toml',
    'run',
    '/spec/openapi.json',
    '--url',
    base,
    '--phases',
    'examples',
    '--checks',
    CHECKS.join(','),
    ...(apiKey ? ['--header', `${API_KEY_HEADER}: ${apiKey}`] : []),
  ],
  { stdout: 'inherit', stderr: 'inherit' }
);

process.exit(await schemathesis.exited);
