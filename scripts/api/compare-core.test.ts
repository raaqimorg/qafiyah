import { describe, expect, it } from 'bun:test';

import {
  diffCaptured,
  maskAccountBody,
  percentile,
  summarize,
  type Captured,
} from './compare-core';

const ok: Captured = {
  status: 200,
  headers: { 'content-type': 'application/json', etag: '"a"' },
  body: '{}',
};

describe('diffCaptured', () => {
  it('finds nothing between identical responses', () => {
    expect(diffCaptured('/v1/meters', ok, ok, false)).toEqual([]);
  });

  it('names a changed status, header, and body', () => {
    const changed: Captured = {
      status: 500,
      headers: { 'content-type': 'application/json', etag: '"b"' },
      body: '{"x":1}',
    };
    expect(
      diffCaptured('/v1/meters', ok, changed, false).map((difference) => difference.field)
    ).toEqual(['status', 'header etag', 'body']);
  });

  it('names a header present on one side only', () => {
    const extra: Captured = { ...ok, headers: { ...ok.headers, location: '/v1/poems/abcd' } };
    expect(
      diffCaptured('/v1/poems/abcd', ok, extra, false).map((difference) => difference.field)
    ).toEqual(['header location']);
  });

  it('compares only the status and content type of a status-only path', () => {
    const other: Captured = {
      status: 200,
      headers: { 'content-type': 'application/json', etag: '"z"' },
      body: 'different',
    };
    expect(diffCaptured('/v1/poems/random', ok, other, true)).toEqual([]);
    expect(diffCaptured('/v1/poems/random', ok, { ...other, status: 500 }, true)).toHaveLength(1);
  });
});

describe('maskAccountBody', () => {
  it('replaces ids, key values, prefixes, emails, and timestamps with stable markers', () => {
    const body = JSON.stringify({
      id: 41,
      email: 'compare-1759486000000-a@example.test',
      value: 'qk_live_abc',
      prefix: 'qk_ab',
      created_at: '2026-10-03T10:00:00Z',
      session: { id: 'c2Vzc2lvbg' },
    });
    expect(maskAccountBody(body)).toBe(
      JSON.stringify({
        id: '<id>',
        email: '<email>',
        value: '<key>',
        prefix: '<prefix>',
        created_at: '<time>',
        session: { id: '<id>' },
      })
    );
  });

  it('masks the session id an error echoes in its instance path', () => {
    const body = '{"status":401,"instance":"/account/sessions/Q1tuzo-ldUJ_Fgq"}';
    expect(maskAccountBody(body)).toBe('{"status":401,"instance":"/account/sessions/<session>"}');
  });

  it('leaves a body without volatile fields untouched', () => {
    expect(maskAccountBody('{"plan":"free","requests":500}')).toBe(
      '{"plan":"free","requests":500}'
    );
  });
});

describe('percentile', () => {
  it('takes the nearest rank', () => {
    const values = [10, 1, 9, 2, 8, 3, 7, 4, 6, 5];
    expect(percentile(values, 50)).toBe(5);
    expect(percentile(values, 95)).toBe(10);
    expect(percentile([], 50)).toBe(0);
  });
});

describe('summarize', () => {
  it('pools every sample of a group and compares the medians', () => {
    const groups = summarize([
      { path: '/v1/meters/a', group: 'taxonomy detail', a: [2, 2, 2], b: [1, 1, 1] },
      { path: '/v1/meters/b', group: 'taxonomy detail', a: [4, 4, 4], b: [2, 2, 2] },
      { path: '/v1/poems', group: 'poems list', a: [10], b: [20] },
    ]);
    expect(groups).toEqual([
      {
        group: 'poems list',
        urls: 1,
        samples: 1,
        aP50: 10,
        bP50: 20,
        aP95: 10,
        bP95: 20,
        ratio: 2,
      },
      {
        group: 'taxonomy detail',
        urls: 2,
        samples: 6,
        aP50: 2,
        bP50: 1,
        aP95: 4,
        bP95: 2,
        ratio: 0.5,
      },
    ]);
  });
});
