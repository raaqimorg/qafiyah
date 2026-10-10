import { THEME_STORAGE_KEY } from '@/lib/constants/config';

import {
  parseSettings,
  serializeSettings,
  SETTINGS_STORAGE_KEY,
  type Settings,
  type Theme,
} from './settings-schema';

export function safeGet(storage: Pick<Storage, 'getItem'>, key: string): string | null {
  try {
    return storage.getItem(key);
  } catch {
    return null;
  }
}

export function safeSet(storage: Pick<Storage, 'setItem'>, key: string, value: string): void {
  try {
    storage.setItem(key, value);
  } catch {}
}

export function readStoredSettings(storage: Pick<Storage, 'getItem'>): Settings {
  return parseSettings(safeGet(storage, SETTINGS_STORAGE_KEY), safeGet(storage, THEME_STORAGE_KEY));
}

export function writeStoredSettings(
  storage: Pick<Storage, 'getItem' | 'setItem'>,
  patch: Partial<Settings>
): void {
  safeSet(
    storage,
    SETTINGS_STORAGE_KEY,
    serializeSettings(safeGet(storage, SETTINGS_STORAGE_KEY), patch)
  );
}

export function resolveIsDark(
  theme: Theme,
  matchMedia: (query: string) => { readonly matches: boolean }
): boolean {
  if (theme === 'dark') return true;
  if (theme === 'light') return false;
  return matchMedia('(prefers-color-scheme: dark)').matches;
}
