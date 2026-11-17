import { validateText } from '@/lib/search/csv-filters';
import { SEARCH_HISTORY_STORAGE_KEY } from '@/lib/settings/settings-schema';
import { safeGet, safeSet } from '@/lib/settings/settings-storage';
import { MAX_SEARCH_HISTORY } from '@qafiyah/config';

type ReadableStorage = Pick<Storage, 'getItem'>;
type WriteableStorage = Pick<Storage, 'setItem' | 'getItem'>;

export function parseSearchHistory(raw: string | null): string[] {
  if (raw === null) return [];
  let data: unknown;
  try {
    data = JSON.parse(raw);
  } catch {
    return [];
  }
  if (!Array.isArray(data)) return [];

  const unique = new Set<string>();
  const result: string[] = [];
  for (const item of data) {
    if (typeof item !== 'string') continue;
    const query = item.trim();
    if (query === '' || validateText(query) !== null) continue;
    if (unique.has(query)) continue;
    unique.add(query);
    result.push(query);
    if (result.length === MAX_SEARCH_HISTORY) break;
  }
  return result;
}

export function addSearchEntry(list: readonly string[], query: string): string[] {
  const trimmedQuery = query.trim();
  if (trimmedQuery === '' || validateText(trimmedQuery) !== null) return [...list];
  return [trimmedQuery, ...list.filter((item) => item !== trimmedQuery)].slice(
    0,
    MAX_SEARCH_HISTORY
  );
}

function removeSearchEntry(list: readonly string[], query: string): string[] {
  const trimmedQuery = query.trim();
  return list.filter((item) => item !== trimmedQuery);
}

export function readSearchHistory(storage: ReadableStorage | undefined): string[] {
  if (storage === undefined) return [];
  return parseSearchHistory(safeGet(storage, SEARCH_HISTORY_STORAGE_KEY));
}
export function filterSearchHistory(list: readonly string[], input: string): string[] {
  const prefix = input.trim();
  if (prefix === '') return [...list];
  return list.filter((item) => item.startsWith(prefix));
}

function write(storage: WriteableStorage | undefined, list: readonly string[]): string[] {
  if (storage !== undefined) {
    safeSet(storage, SEARCH_HISTORY_STORAGE_KEY, JSON.stringify(list));
  }
  return [...list];
}
export function addToSearchHistory(storage: WriteableStorage | undefined, query: string): string[] {
  return write(storage, addSearchEntry(readSearchHistory(storage), query));
}
export function removeFromSearchHistory(
  storage: WriteableStorage | undefined,
  query: string
): string[] {
  return write(storage, removeSearchEntry(readSearchHistory(storage), query));
}
export function clearSearchHistory(storage: WriteableStorage | undefined): string[] {
  return write(storage, []);
}
