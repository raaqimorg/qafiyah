import { afterEach, describe, expect, it, vi } from 'vitest';

import { failedBody } from '@/test/fetch';

import {
  buildRandomPoemUrl,
  fetchRandomPoemSlugWithRetry,
  fetchRandomPoemText,
} from './random-poem';

const BASE = 'https://api.test';

type FetchInit = { signal?: AbortSignal; headers?: Readonly<Record<string, string>> };
type Step = (init?: FetchInit) => Promise<Response>;

function stubFetch(impl: (url: string, init?: FetchInit) => Promise<Response>) {
  const mock = vi.fn(impl);
  vi.stubGlobal('fetch', mock);
  return mock;
}

function hangUntilAborted(init?: FetchInit): Promise<Response> {
  return new Promise<Response>((_resolve, reject) => {
    const signal = init?.signal;
    if (!signal) return;
    if (signal.aborted) {
      reject(signal.reason);
      return;
    }
    signal.addEventListener('abort', () => reject(signal.reason), { once: true });
  });
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('buildRandomPoemUrl', () => {
  it('encodes the option in the query string', () => {
    expect(buildRandomPoemUrl(BASE, 'slug')).toBe(`${BASE}/v1/poems/random?option=slug`);
    expect(buildRandomPoemUrl(BASE, 'lines')).toBe(`${BASE}/v1/poems/random?option=lines`);
  });
});

describe('fetchRandomPoemText outcomes', () => {
  it('returns the trimmed body on a 2xx response', async () => {
    stubFetch(async () => new Response('  abcd \n', { status: 200 }));
    const result = await fetchRandomPoemText(BASE, 'slug');
    expect(result.isOk()).toBe(true);
    expect(result._unsafeUnwrap()).toBe('abcd');
  });

  it('passes the requested option through to the URL', async () => {
    const mock = stubFetch(async () => new Response('abcd', { status: 200 }));
    await fetchRandomPoemText(BASE, 'lines');
    expect(mock.mock.calls[0]?.[0]).toBe(`${BASE}/v1/poems/random?option=lines`);
  });

  it('forwards custom headers to fetch when provided', async () => {
    const mock = stubFetch(async () => new Response('abcd', { status: 200 }));
    await fetchRandomPoemText(BASE, 'slug', undefined, { 'x-api-key': 'k' });
    expect(mock.mock.calls[0]?.[1]?.headers).toEqual({ 'x-api-key': 'k' });
  });

  it('omits the headers key entirely when none are provided', async () => {
    const mock = stubFetch(async () => new Response('abcd', { status: 200 }));
    await fetchRandomPoemText(BASE, 'slug');
    expect(mock.mock.calls[0]?.[1]?.headers).toBeUndefined();
  });

  it('maps 429 to rate_limited', async () => {
    stubFetch(async () => new Response('slow down', { status: 429 }));
    const error = (await fetchRandomPoemText(BASE, 'slug'))._unsafeUnwrapErr();
    expect(error.kind).toBe('rate_limited');
  });

  it('maps other non-2xx to http_error with the status', async () => {
    stubFetch(async () => new Response('boom', { status: 503 }));
    const error = (await fetchRandomPoemText(BASE, 'slug'))._unsafeUnwrapErr();
    expect(error).toMatchObject({ kind: 'http_error', status: 503 });
  });

  it('maps a 2xx with a blank body to empty_response', async () => {
    stubFetch(async () => new Response('   \n  ', { status: 200 }));
    const error = (await fetchRandomPoemText(BASE, 'slug'))._unsafeUnwrapErr();
    expect(error.kind).toBe('empty_response');
  });

  it('maps a thrown fetch (offline/DNS) to network', async () => {
    stubFetch(() => Promise.reject(new TypeError('Failed to fetch')));
    const error = (await fetchRandomPoemText(BASE, 'slug'))._unsafeUnwrapErr();
    expect(error).toMatchObject({ kind: 'network', message: 'Failed to fetch' });
  });

  it.each([
    new TypeError('Connection closed'),
    new DOMException('The operation was aborted', 'AbortError'),
    new DOMException('The operation timed out', 'TimeoutError'),
  ])('maps a failed response body to network and preserves $name', async (cause) => {
    stubFetch(async () => failedBody(cause));
    const error = (await fetchRandomPoemText(BASE, 'slug'))._unsafeUnwrapErr();
    expect(error).toEqual({
      kind: 'network',
      url: `${BASE}/v1/poems/random?option=slug`,
      message: cause.message,
      name: cause.name,
    });
  });

  it('maps a non-Error body rejection to a network error without a name', async () => {
    stubFetch(async () => failedBody('Connection closed'));
    const error = (await fetchRandomPoemText(BASE, 'lines'))._unsafeUnwrapErr();
    expect(error).toEqual({
      kind: 'network',
      url: `${BASE}/v1/poems/random?option=lines`,
      message: 'Connection closed',
    });
  });
});

describe('fetchRandomPoemText timeout', () => {
  it('aborts a hung request via the default-style signal and reports network', async () => {
    stubFetch((_url, init) => hangUntilAborted(init));
    const started = Date.now();
    const error = (
      await fetchRandomPoemText(BASE, 'slug', AbortSignal.timeout(40))
    )._unsafeUnwrapErr();
    expect(error.kind).toBe('network');
    expect(Date.now() - started).toBeGreaterThanOrEqual(35);
  });

  it('aborts immediately when handed an already-aborted signal', async () => {
    const mock = stubFetch((_url, init) => hangUntilAborted(init));
    const error = (await fetchRandomPoemText(BASE, 'slug', AbortSignal.abort()))._unsafeUnwrapErr();
    expect(error.kind).toBe('network');
    expect(mock).toHaveBeenCalledTimes(1);
  });
});

const FAST = { attemptTimeoutMs: 1000, totalBudgetMs: 10_000, backoffBaseMs: 5 } as const;

const ok =
  (slug = 'abcd'): Step =>
  async () =>
    new Response(slug, { status: 200 });
const withStatus =
  (code: number): Step =>
  async () =>
    new Response('x', { status: code });
const emptyBody = (): Step => async () => new Response('   ', { status: 200 });
const rejectWith =
  (error: unknown): Step =>
  () =>
    Promise.reject(error);
const hang = (): Step => (init) =>
  new Promise<Response>((_resolve, reject) => {
    const signal = init?.signal;
    if (!signal) return;
    if (signal.aborted) {
      reject(signal.reason);
      return;
    }
    signal.addEventListener('abort', () => reject(signal.reason), { once: true });
  });

function stubFetchSteps(steps: readonly Step[]) {
  const startedAt = Date.now();
  const at: number[] = [];
  const mock = vi.fn((_url: string, init?: FetchInit) => {
    at.push(Date.now() - startedAt);
    const step = steps[Math.min(at.length - 1, steps.length - 1)];
    if (!step) throw new Error('stubFetch: no step configured');
    return step(init);
  });
  vi.stubGlobal('fetch', mock);
  return { mock, at };
}

describe('fetchRandomPoemSlugWithRetry retry logic', () => {
  it('resolves on the first success without retrying', async () => {
    const { mock } = stubFetchSteps([ok('abcd')]);
    const result = await fetchRandomPoemSlugWithRetry(BASE, FAST);
    expect(result.isOk()).toBe(true);
    expect(result._unsafeUnwrap()).toBe('abcd');
    expect(mock).toHaveBeenCalledTimes(1);
  });

  it('retries after failures and returns the eventual success', async () => {
    const { mock } = stubFetchSteps([withStatus(500), withStatus(503), ok('wxyz')]);
    const result = await fetchRandomPoemSlugWithRetry(BASE, FAST);
    expect(result._unsafeUnwrap()).toBe('wxyz');
    expect(mock).toHaveBeenCalledTimes(3);
  });

  it('gives up after `attempts` failures and returns the last error', async () => {
    const { mock } = stubFetchSteps([withStatus(500)]);
    const result = await fetchRandomPoemSlugWithRetry(BASE, { ...FAST, attempts: 3 });
    expect(result._unsafeUnwrapErr()).toMatchObject({ kind: 'http_error', status: 500 });
    expect(mock).toHaveBeenCalledTimes(3);
  });

  it('returns the last network error after all response bodies fail', async () => {
    const { mock } = stubFetchSteps([
      async () => failedBody(new TypeError('First connection closed')),
      async () => failedBody(new TypeError('Last connection closed')),
    ]);
    const result = await fetchRandomPoemSlugWithRetry(BASE, { ...FAST, attempts: 3 });
    expect(result._unsafeUnwrapErr()).toMatchObject({
      kind: 'network',
      message: 'Last connection closed',
    });
    expect(mock).toHaveBeenCalledTimes(3);
  });

  it('stops issuing requests the moment one succeeds', async () => {
    const { mock } = stubFetchSteps([withStatus(500), withStatus(500), ok('abcd')]);
    const result = await fetchRandomPoemSlugWithRetry(BASE, { ...FAST, attempts: 10 });
    expect(result.isOk()).toBe(true);
    expect(mock).toHaveBeenCalledTimes(3);
  });

  it('surfaces invalid_slug when the body is not a 4-letter slug', async () => {
    stubFetchSteps([ok('toolong')]);
    const result = await fetchRandomPoemSlugWithRetry(BASE, { ...FAST, attempts: 1 });
    expect(result._unsafeUnwrapErr()).toMatchObject({ kind: 'invalid_slug', raw: 'toolong' });
  });

  const retryableFailures: readonly (readonly [string, Step])[] = [
    ['network', rejectWith(new TypeError('offline'))],
    ['response body', async () => failedBody(new TypeError('Connection closed'))],
    ['rate_limited', withStatus(429)],
    ['http_error', withStatus(500)],
    ['empty_response', emptyBody()],
    ['invalid_slug', ok('toolong')],
  ];

  it.each(retryableFailures)('retries after a %s failure', async (_kind, failingStep) => {
    const { mock } = stubFetchSteps([failingStep, ok('abcd')]);
    const result = await fetchRandomPoemSlugWithRetry(BASE, FAST);
    expect(result.isOk()).toBe(true);
    expect(mock).toHaveBeenCalledTimes(2);
  });
});

describe('fetchRandomPoemSlugWithRetry backoff', () => {
  it('spaces retries with exponential backoff', async () => {
    const { at } = stubFetchSteps([withStatus(500)]);
    await fetchRandomPoemSlugWithRetry(BASE, {
      attempts: 3,
      backoffBaseMs: 40,
      attemptTimeoutMs: 1000,
      totalBudgetMs: 10_000,
    });
    expect(at).toHaveLength(3);
    const firstGap = at[1]! - at[0]!;
    const secondGap = at[2]! - at[1]!;
    expect(firstGap).toBeGreaterThanOrEqual(36);
    expect(secondGap).toBeGreaterThanOrEqual(76);
    expect(secondGap).toBeGreaterThan(firstGap);
  });
});

describe('fetchRandomPoemSlugWithRetry timeout and budget', () => {
  it('times out a single hung attempt and maps it to a network error', async () => {
    const startedAt = Date.now();
    const { mock } = stubFetchSteps([hang()]);
    const result = await fetchRandomPoemSlugWithRetry(BASE, {
      attempts: 1,
      attemptTimeoutMs: 40,
      totalBudgetMs: 10_000,
      backoffBaseMs: 5,
    });
    expect(result._unsafeUnwrapErr().kind).toBe('network');
    expect(mock).toHaveBeenCalledTimes(1);
    expect(Date.now() - startedAt).toBeGreaterThanOrEqual(35);
  });

  it('keeps retrying hung attempts up to the limit when the budget allows', async () => {
    const { mock } = stubFetchSteps([hang()]);
    const result = await fetchRandomPoemSlugWithRetry(BASE, {
      attempts: 3,
      attemptTimeoutMs: 30,
      totalBudgetMs: 10_000,
      backoffBaseMs: 5,
    });
    expect(result._unsafeUnwrapErr().kind).toBe('network');
    expect(mock).toHaveBeenCalledTimes(3);
  });

  it('stops once the total budget is exhausted, well short of the attempt limit', async () => {
    const startedAt = Date.now();
    const { mock } = stubFetchSteps([hang()]);
    const result = await fetchRandomPoemSlugWithRetry(BASE, {
      attempts: 50,
      attemptTimeoutMs: 20,
      totalBudgetMs: 200,
      backoffBaseMs: 5,
    });
    const elapsed = Date.now() - startedAt;
    expect(result._unsafeUnwrapErr().kind).toBe('network');
    expect(mock.mock.calls.length).toBeGreaterThanOrEqual(2);
    expect(mock.mock.calls.length).toBeLessThanOrEqual(15);
    expect(elapsed).toBeGreaterThanOrEqual(190);
    expect(elapsed).toBeLessThan(450);
  });

  it('cuts an in-flight attempt short when the total budget runs out first', async () => {
    const startedAt = Date.now();
    const { mock } = stubFetchSteps([hang()]);
    const result = await fetchRandomPoemSlugWithRetry(BASE, {
      attempts: 3,
      attemptTimeoutMs: 5000,
      totalBudgetMs: 100,
      backoffBaseMs: 5,
    });
    const elapsed = Date.now() - startedAt;
    expect(result._unsafeUnwrapErr().kind).toBe('network');
    expect(mock).toHaveBeenCalledTimes(1);
    expect(elapsed).toBeGreaterThanOrEqual(90);
    expect(elapsed).toBeLessThan(1000);
  });

  it('works in browsers without AbortSignal.any (Chrome before 116, Safari before 17.4)', async () => {
    const any = Object.getOwnPropertyDescriptor(AbortSignal, 'any');
    Reflect.deleteProperty(AbortSignal, 'any');
    try {
      stubFetchSteps([ok('abcd')]);
      const result = await fetchRandomPoemSlugWithRetry(BASE, FAST);
      expect(result._unsafeUnwrap()).toBe('abcd');
    } finally {
      if (any) Object.defineProperty(AbortSignal, 'any', any);
    }
  });
});
