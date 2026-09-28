import { err, ok, type Result } from 'neverthrow';
import * as v from 'valibot';

import { type PoemSlug, poemSlugSchema } from '@/lib/api/brands';
import { API_RANDOM_POEM_PATH } from '@qafiyah/config';

const RANDOM_POEM_TIMEOUT_MS = 5000;

export type RandomPoemTransportError =
  | {
      readonly kind: 'network';
      readonly url: string;
      readonly message: string;
      readonly name?: string;
    }
  | { readonly kind: 'rate_limited'; readonly url: string }
  | { readonly kind: 'http_error'; readonly url: string; readonly status: number }
  | { readonly kind: 'empty_response'; readonly url: string };

const RANDOM_POEM_OPTIONS = ['slug', 'lines'] as const;
export type RandomPoemOption = (typeof RANDOM_POEM_OPTIONS)[number];

export function buildRandomPoemUrl(baseUrl: string, option: RandomPoemOption): string {
  return `${baseUrl}${API_RANDOM_POEM_PATH}?option=${option}`;
}

export async function fetchRandomPoemText(
  baseUrl: string,
  option: RandomPoemOption,
  signal: AbortSignal = AbortSignal.timeout(RANDOM_POEM_TIMEOUT_MS),
  headers?: Readonly<Record<string, string>>
): Promise<Result<string, RandomPoemTransportError>> {
  const url = buildRandomPoemUrl(baseUrl, option);
  try {
    const response = await fetch(url, headers ? { signal, headers } : { signal });
    if (response.status === 429) return err({ kind: 'rate_limited', url });
    if (!response.ok) return err({ kind: 'http_error', url, status: response.status });
    const text = (await response.text()).trim();
    if (!text) return err({ kind: 'empty_response', url });
    return ok(text);
  } catch (cause) {
    return err({
      kind: 'network',
      url,
      message: cause instanceof Error ? cause.message : String(cause),
      ...(cause instanceof Error && cause.name ? { name: cause.name } : {}),
    });
  }
}

type FetchRandomPoemSlugError =
  | RandomPoemTransportError
  | {
      readonly kind: 'invalid_slug';
      readonly url: string;
      readonly raw: string;
      readonly issues: readonly string[];
    };

const RANDOM_POEM_ATTEMPTS = 3;
const RANDOM_POEM_RETRY_BASE_MS = 150;
const RANDOM_POEM_ATTEMPT_TIMEOUT_MS = 3000;
const RANDOM_POEM_TOTAL_BUDGET_MS = 7000;

function delay(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve) => {
    if (signal.aborted) {
      resolve();
      return;
    }
    const onDone = () => {
      clearTimeout(timer);
      signal.removeEventListener('abort', onDone);
      resolve();
    };
    const timer = setTimeout(onDone, ms);
    signal.addEventListener('abort', onDone, { once: true });
  });
}

async function fetchRandomPoemSlug(
  baseUrl: string,
  signal?: AbortSignal,
  headers?: Readonly<Record<string, string>>
): Promise<Result<PoemSlug, FetchRandomPoemSlugError>> {
  const textResult = await fetchRandomPoemText(baseUrl, 'slug', signal, headers);
  if (textResult.isErr()) return err(textResult.error);
  const slug = textResult.value;
  const parsed = v.safeParse(poemSlugSchema, slug);
  if (!parsed.success) {
    return err({
      kind: 'invalid_slug',
      url: buildRandomPoemUrl(baseUrl, 'slug'),
      raw: slug,
      issues: parsed.issues.map((i) => i.message),
    });
  }
  return ok(parsed.output);
}

export type RandomPoemRetryOptions = {
  readonly attempts?: number;
  readonly backoffBaseMs?: number;
  readonly attemptTimeoutMs?: number;
  readonly totalBudgetMs?: number;
  readonly headers?: Readonly<Record<string, string>>;
};

export async function fetchRandomPoemSlugWithRetry(
  baseUrl: string,
  options: RandomPoemRetryOptions = {}
): Promise<Result<PoemSlug, FetchRandomPoemSlugError>> {
  const {
    attempts = RANDOM_POEM_ATTEMPTS,
    backoffBaseMs = RANDOM_POEM_RETRY_BASE_MS,
    attemptTimeoutMs = RANDOM_POEM_ATTEMPT_TIMEOUT_MS,
    totalBudgetMs = RANDOM_POEM_TOTAL_BUDGET_MS,
    headers,
  } = options;

  const startedAt = Date.now();
  const deadline = AbortSignal.timeout(totalBudgetMs);
  const attemptSignal = () =>
    AbortSignal.timeout(
      Math.max(0, Math.min(attemptTimeoutMs, totalBudgetMs - (Date.now() - startedAt)))
    );

  let result = await fetchRandomPoemSlug(baseUrl, attemptSignal(), headers);
  for (let attempt = 1; attempt < attempts && result.isErr() && !deadline.aborted; attempt++) {
    await delay(backoffBaseMs * 2 ** (attempt - 1), deadline);
    // oxlint-disable-next-line typescript/no-unnecessary-condition -- the deadline may fire during the delay above
    if (deadline.aborted) break;
    result = await fetchRandomPoemSlug(baseUrl, attemptSignal(), headers);
  }
  return result;
}
