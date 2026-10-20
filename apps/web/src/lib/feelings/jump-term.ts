import { MAX_HIGHLIGHT_TERM_LENGTH } from '@/lib/highlight';

export function jumpTerm(parts: readonly string[]): string {
  const first = (parts[0] ?? '').trim();
  if (first.length <= MAX_HIGHLIGHT_TERM_LENGTH) return first;
  const cut = first.slice(0, MAX_HIGHLIGHT_TERM_LENGTH + 1);
  return cut.slice(0, cut.lastIndexOf(' ')).trim();
}
