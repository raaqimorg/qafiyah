import { describe, expect, it, vi } from 'vitest';

import {
  parseSearchHistory,
  addSearchEntry,
  addToSearchHistory,
  readSearchHistory,
} from '@/lib/search/search-history';
import { MAX_QUERY_LENGTH, MAX_SEARCH_HISTORY } from '@qafiyah/config';

const INVALID = 'a'.repeat(MAX_QUERY_LENGTH + 1);
const createList = (size: number): string[] =>
  Array.from({ length: size }, (_, index) => `item${index}`);
describe('order', () => {
  it('puts newest entry first', () => {
    expect(addSearchEntry(['a', 'b'], 'c')).toEqual(['c', 'a', 'b']);
  });

  it('preserves order when parsing', () => {
    expect(parseSearchHistory(JSON.stringify(['c', 'a', 'b']))).toEqual(['c', 'a', 'b']);
  });
});

describe('dedup', () => {
  it('moves a repeated entry to the top without duplicating it', () => {
    expect(addSearchEntry(['a', 'b', 'c'], 'c')).toEqual(['c', 'a', 'b']);
  });
  it('remove duplicates when parsing', () => {
    expect(parseSearchHistory(JSON.stringify(['a', 'b', 'c', 'a', 'a ']))).toEqual(['a', 'b', 'c']);
  });
});

describe('cap', () => {
  it('drops the oldest entry when limit is exceeded', () => {
    const result = addSearchEntry(createList(MAX_SEARCH_HISTORY), 'new');
    expect(result).toHaveLength(MAX_SEARCH_HISTORY);
    expect(result[0]).toBe('new');
    expect(result).not.toContain(`item${MAX_SEARCH_HISTORY - 1}`);
  });
  it('caps oversized lists when parsing', () => {
    expect(parseSearchHistory(JSON.stringify(createList(MAX_SEARCH_HISTORY + 1)))).toHaveLength(
      MAX_SEARCH_HISTORY
    );
  });
});

describe('bad JSON', () => {
  it('returns an empty list for null and empty strings', () => {
    expect(parseSearchHistory(null)).toEqual([]);
    expect(parseSearchHistory('')).toEqual([]);
  });
  it('returns an empty list for bad JSON', () => {
    expect(parseSearchHistory('{bad JSON')).toEqual([]);
  });
  it('returns an empty list when the JSON is not an array', () => {
    for (const raw of ['null', '10', '"foo"', 'true', '{}']) {
      expect(parseSearchHistory(raw)).toEqual([]);
    }
  });
});

describe('tampered entries', () => {
  it('drops items that are not strings', () => {
    expect(parseSearchHistory(JSON.stringify([1, null, {}, 'string']))).toEqual(['string']);
  });

  it('drops empty and whitespace-only strings and trims the rest', () => {
    expect(parseSearchHistory(JSON.stringify(['', '  ', ' string ']))).toEqual(['string']);
  });

  it('drops items that fail validateText', () => {
    expect(parseSearchHistory(JSON.stringify(['a', INVALID, 'b']))).toEqual(['a', 'b']);
  });

  it('does not count dropped entries towards the cap', () => {
    expect(
      parseSearchHistory(JSON.stringify([INVALID, INVALID, INVALID, ...createList(9)]))
    ).toEqual(createList(9));
  });
  it('reject invalid entries when adding', () => {
    expect(addSearchEntry(['a', 'b'], '')).toEqual(['a', 'b']);
    expect(addSearchEntry(['a', 'b'], '   ')).toEqual(['a', 'b']);
    expect(addSearchEntry(['a', 'b'], INVALID)).toEqual(['a', 'b']);
  });
});

describe('storage that throws', () => {
  it('does not throw when setItem throws', () => {
    const storage = {
      getItem: vi.fn(() => null),
      setItem: vi.fn(() => {
        throw new Error('QuotaExceededError');
      }),
    };
    expect(() => addToSearchHistory(storage, 'string')).not.toThrow();
  });
  it('returns an empty list when getItem throws', () => {
    const storage = {
      getItem: vi.fn(() => {
        throw new Error('SecurityError');
      }),
    };
    expect(readSearchHistory(storage)).toEqual([]);
  });
});
