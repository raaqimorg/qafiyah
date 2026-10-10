import { describe, expect, it } from 'vitest';

import { SETTINGS_STORAGE_KEY } from './settings-schema';
import { readStoredSettings, resolveIsDark, safeGet, safeSet } from './settings-storage';

function memory(entries: Readonly<Record<string, string>> = {}) {
  const store = new Map(Object.entries(entries));
  return {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => {
      store.set(key, value);
    },
  };
}

describe('safeGet and safeSet', () => {
  it('get returns null when storage throws', () => {
    const throwing = {
      getItem: () => {
        throw new Error('denied');
      },
    };
    expect(safeGet(throwing, 'k')).toBeNull();
  });

  it('set swallows a storage error', () => {
    const throwing = {
      setItem: () => {
        throw new Error('quota');
      },
    };
    expect(() => safeSet(throwing, 'k', 'v')).not.toThrow();
  });
});

describe('readStoredSettings', () => {
  it('round-trips through the storage keys', () => {
    const s = memory({
      [SETTINGS_STORAGE_KEY]: JSON.stringify({ v: 1, theme: 'dark', fontFamily: 'thmanyah' }),
    });
    expect(readStoredSettings(s)).toEqual({ theme: 'dark', fontFamily: 'thmanyah' });
  });
});

describe('resolveIsDark', () => {
  it('returns true for dark and false for light regardless of the system', () => {
    expect(resolveIsDark('dark', () => ({ matches: false }))).toBe(true);
    expect(resolveIsDark('light', () => ({ matches: true }))).toBe(false);
  });

  it('defers to the system preference for system theme', () => {
    expect(resolveIsDark('system', () => ({ matches: true }))).toBe(true);
    expect(resolveIsDark('system', () => ({ matches: false }))).toBe(false);
  });
});
