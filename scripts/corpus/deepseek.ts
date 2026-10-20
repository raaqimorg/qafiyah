const URL = 'https://api.deepseek.com/chat/completions';
const MODEL = 'deepseek-flash';
const TIMEOUT_MS = 300_000;
const PEAK_HOURS_UTC: readonly (readonly [from: number, to: number])[] = [
  [1, 4],
  [6, 10],
];
const OFF_PEAK_DOLLARS_PER_MILLION = {
  prompt_cache_hit_tokens: 0.003,
  prompt_cache_miss_tokens: 0.15,
  completion_tokens: 0.6,
} as const;

export type Usage = {
  readonly prompt_cache_hit_tokens: number;
  readonly prompt_cache_miss_tokens: number;
  readonly completion_tokens: number;
};

export type ChatRequest = {
  readonly system: string;
  readonly user: string;
  readonly maxTokens: number;
};

type Reply = { readonly content: string; readonly usage: Usage };

const count = (value: unknown): number => (typeof value === 'number' ? value : 0);

function field(value: unknown, name: string): unknown {
  if (typeof value !== 'object' || value === null || !(name in value)) return undefined;
  const found: unknown = Reflect.get(value, name);
  return found;
}

function readReply(body: unknown): Reply {
  const choices = field(body, 'choices');
  const content = field(
    field(Array.isArray(choices) ? choices[0] : undefined, 'message'),
    'content'
  );
  if (typeof content !== 'string') throw new Error('the reply has no message');
  const usage = field(body, 'usage');
  return {
    content,
    usage: {
      prompt_cache_hit_tokens: count(field(usage, 'prompt_cache_hit_tokens')),
      prompt_cache_miss_tokens: count(field(usage, 'prompt_cache_miss_tokens')),
      completion_tokens: count(field(usage, 'completion_tokens')),
    },
  };
}

export async function chat(key: string, request: ChatRequest): Promise<Reply> {
  const response = await fetch(URL, {
    method: 'POST',
    headers: { Authorization: `Bearer ${key}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: MODEL,
      reasoning_effort: 'none',
      temperature: 0,
      response_format: { type: 'json_object' },
      max_tokens: request.maxTokens,
      messages: [
        { role: 'system', content: request.system },
        { role: 'user', content: request.user },
      ],
    }),
    signal: AbortSignal.timeout(TIMEOUT_MS),
  });
  if (!response.ok) throw new Error(`DeepSeek answered ${response.status}`);
  return readReply(await response.json());
}

export function isPeak(now: Date): boolean {
  const weekday = now.getUTCDay() >= 1 && now.getUTCDay() <= 5;
  const hour = now.getUTCHours();
  return weekday && PEAK_HOURS_UTC.some(([from, to]) => hour >= from && hour < to);
}

export function offPeakDollars(usage: Usage): number {
  return (
    (usage.prompt_cache_hit_tokens * OFF_PEAK_DOLLARS_PER_MILLION.prompt_cache_hit_tokens +
      usage.prompt_cache_miss_tokens * OFF_PEAK_DOLLARS_PER_MILLION.prompt_cache_miss_tokens +
      usage.completion_tokens * OFF_PEAK_DOLLARS_PER_MILLION.completion_tokens) /
    1_000_000
  );
}
