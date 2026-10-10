import { browserStorage } from '@/lib/browser-storage';
import { SITE_THEME_COLOR_DARK_HEX, SITE_THEME_COLOR_HEX } from '@/lib/constants/site-meta';

import { DEFAULT_SETTINGS, type FontFamily, type Settings, type Theme } from './settings-schema';
import { readStoredSettings, resolveIsDark, writeStoredSettings } from './settings-storage';

let cache: Settings | null = null;

export function getSettings(): Settings {
  if (typeof window === 'undefined') return DEFAULT_SETTINGS;
  if (cache === null) {
    const storage = browserStorage();
    cache = storage === undefined ? DEFAULT_SETTINGS : readStoredSettings(storage);
  }
  return cache;
}

export function updateSettings(patch: Partial<Settings>): void {
  cache = { ...getSettings(), ...patch };
  const storage = browserStorage();
  if (storage !== undefined) writeStoredSettings(storage, patch);
  if (patch.theme !== undefined) applyTheme(patch.theme);
  if (patch.fontFamily !== undefined) applyFontFamily(patch.fontFamily);
}

function applyTheme(theme: Theme): void {
  if (typeof document === 'undefined') return;
  const isDark = resolveIsDark(theme, (query) => window.matchMedia(query));
  const root = document.documentElement;
  root.classList.toggle('dark', isDark);
  root.style.colorScheme = isDark ? 'dark' : 'light';
  const meta = document.querySelector('meta[name="theme-color"]');
  if (meta) meta.setAttribute('content', isDark ? SITE_THEME_COLOR_DARK_HEX : SITE_THEME_COLOR_HEX);
}

function applyFontFamily(fontFamily: FontFamily): void {
  if (typeof document === 'undefined') return;
  document.documentElement.dataset['font'] = fontFamily;
}
