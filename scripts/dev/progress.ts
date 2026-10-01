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

const CONTAINER_STATE = /^\s*Container (\S+)\s+([A-Z][a-z]+)\s*$/;
const DB_INIT = /\[db-init\] (.+?)(?:\.\.\.)?\s*$/;
const SERVICE = /-([a-z]+)(?:-\d+)?$/;
const SETTLED = new Set(['healthy', 'running']);

const serviceOf = (container: string): string => SERVICE.exec(container)?.[1] ?? container;
const withoutPaths = (text: string): string => text.replaceAll(/\S*\/(\S+)/g, '$1');

export function composeProgress(): (line: string) => string | undefined {
  const states = new Map<string, string>();
  const describe = () =>
    [...states].map(([service, current]) => `${service} ${current}`).join(' · ');
  return (line) => {
    const container = CONTAINER_STATE.exec(line);
    if (container) {
      const [, name = '', state = ''] = container;
      states.set(serviceOf(name), state.toLowerCase());
      return describe();
    }
    const init = DB_INIT.exec(line);
    if (!init) return undefined;
    if (!SETTLED.has(states.get('db') ?? '')) states.set('db', withoutPaths(init[1] ?? ''));
    return describe();
  };
}

const BUILD_STEP = /^#\d+ \[\S+ (\S+)\s+(\d+)\/(\d+)\]/;

export function imageBuildProgress(line: string): string | undefined {
  const step = BUILD_STEP.exec(line);
  if (!step) return undefined;
  const [, stage, done, total] = step;
  return `building image, ${stage} step ${done}/${total}`;
}

const COMPILING = /^\s*Compiling (\S+) v/;

export function cargoProgress(): (line: string) => string | undefined {
  let crates = 0;
  return (line) => {
    const compiling = COMPILING.exec(line);
    if (!compiling) return undefined;
    crates += 1;
    return `compiling ${compiling[1]} · ${crates} crate${crates === 1 ? '' : 's'}`;
  };
}

export function fit(text: string, width: number): string {
  if (width <= 0) return '';
  return text.length <= width ? text : `${text.slice(0, width - 1)}…`;
}

export function elapsed(ms: number): string {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  return minutes === 0 ? `${seconds}s` : `${minutes}m${seconds % 60}s`;
}

export async function readLines(
  stream: ReadableStream<Uint8Array>,
  onLine: (line: string) => void
): Promise<void> {
  const decoder = new TextDecoder();
  let pending = '';
  for await (const chunk of stream) {
    pending += decoder.decode(chunk, { stream: true });
    const lines = pending.split('\n');
    pending = lines.pop() ?? '';
    for (const line of lines) onLine(line.replace(/\r$/, ''));
  }
  if (pending) onLine(pending.replace(/\r$/, ''));
}
