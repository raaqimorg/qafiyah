const count = (value: number) => value.toLocaleString('en-US');

export function indexerProgress(line: string): string | undefined {
  if (!line.startsWith('{')) return undefined;
  let event: unknown;
  try {
    event = JSON.parse(line);
  } catch {
    return undefined;
  }
  if (event === null || typeof event !== 'object') return undefined;
  const { stage, index, indexed, total } = event as Record<string, unknown>;
  if (stage !== 'progress' || typeof index !== 'string') return undefined;
  if (typeof indexed !== 'number' || typeof total !== 'number') return undefined;
  return `${index} ${count(indexed)} / ${count(total)}`;
}

export function elapsed(ms: number): string {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  return minutes === 0 ? `${seconds}s` : `${minutes}m${seconds % 60}s`;
}

export async function readLines(
  stream: ReadableStream<Uint8Array>,
  onLine: (line: string) => void
): Promise<string> {
  const decoder = new TextDecoder();
  let text = '';
  let pending = '';
  for await (const chunk of stream) {
    const piece = decoder.decode(chunk, { stream: true });
    text += piece;
    pending += piece;
    const lines = pending.split('\n');
    pending = lines.pop() ?? '';
    for (const line of lines) onLine(line);
  }
  if (pending) onLine(pending);
  return text;
}
