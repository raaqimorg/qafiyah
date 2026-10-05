import { describe, expect, it } from 'vitest';

import { methodLabel, timeRequest, type RequestAttributes } from './request-timing';

function clock(...readings: number[]): () => number {
  let i = 0;
  return () => readings[Math.min(i++, readings.length - 1)] ?? 0;
}

function recorder() {
  const calls: [number, RequestAttributes][] = [];
  const record = (seconds: number, attributes: RequestAttributes): void => {
    calls.push([seconds, attributes]);
  };
  return { calls, record };
}

function streamOf(...chunks: string[]): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder();
  return new ReadableStream({
    start(controller) {
      for (const chunk of chunks) controller.enqueue(encoder.encode(chunk));
      controller.close();
    },
  });
}

const request = { method: 'GET', route: '/poems/[slug]' };

function expected(status: number): RequestAttributes {
  return {
    'http.request.method': 'GET',
    'http.route': '/poems/[slug]',
    'http.response.status_code': status,
  };
}

describe('methodLabel', () => {
  it('keeps a known method and folds anything else into _OTHER', () => {
    expect(methodLabel('GET')).toBe('GET');
    expect(methodLabel('OPTIONS')).toBe('OPTIONS');
    expect(methodLabel('FOO')).toBe('_OTHER');
    expect(methodLabel('get')).toBe('_OTHER');
  });
});

describe('timeRequest', () => {
  it('records once, after the whole body is read, with the route pattern and status', async () => {
    const { calls, record } = recorder();
    const response = await timeRequest(
      record,
      request,
      async () => new Response(streamOf('<p>', 'hi</p>'), { status: 200, headers: { 'x-a': 'b' } }),
      clock(1000, 1250)
    );
    expect(calls).toEqual([]);
    expect(response.headers.get('x-a')).toBe('b');
    expect(await response.text()).toBe('<p>hi</p>');
    expect(calls).toEqual([[0.25, expected(200)]]);
  });

  it('records at once for a response without a body and for a 304', async () => {
    const { calls, record } = recorder();
    await timeRequest(
      record,
      request,
      async () => new Response(null, { status: 204 }),
      clock(0, 10)
    );
    await timeRequest(
      record,
      request,
      async () => new Response(null, { status: 304 }),
      clock(0, 20)
    );
    expect(calls).toEqual([
      [0.01, expected(204)],
      [0.02, expected(304)],
    ]);
  });

  it('records once when the client cancels the body', async () => {
    const { calls, record } = recorder();
    const response = await timeRequest(
      record,
      request,
      async () => new Response(streamOf('a', 'b'), { status: 200 }),
      clock(0, 5)
    );
    await response.body?.cancel();
    await response.body?.cancel();
    expect(calls).toEqual([[0.005, expected(200)]]);
  });

  it('records a 500 and rethrows when rendering throws', async () => {
    const { calls, record } = recorder();
    const failure = new Error('render failed');
    await expect(
      timeRequest(
        record,
        request,
        async () => {
          throw failure;
        },
        clock(0, 30)
      )
    ).rejects.toBe(failure);
    expect(calls).toEqual([[0.03, expected(500)]]);
  });
});
