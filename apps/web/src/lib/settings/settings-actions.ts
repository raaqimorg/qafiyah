import { captureEvent } from '@/lib/analytics/capture-event';

import { clampFontScale, type Theme } from './settings-schema';
import { resolveIsDark } from './settings-storage';
import { getSettings, updateSettings } from './settings-store';

export function changeTheme(theme: Theme): void {
  const isChange = getSettings().theme !== theme;
  updateSettings({ theme });
  if (isChange) captureEvent('setting_changed', { setting: 'theme', value: theme });
}

export function toggleTheme(): void {
  const isDark = resolveIsDark(getSettings().theme, (query) => window.matchMedia(query));
  changeTheme(isDark ? 'light' : 'dark');
}

export function changeFontScale(scale: number): void {
  const poemFontScale = clampFontScale(scale);
  const isChange = getSettings().poemFontScale !== poemFontScale;
  updateSettings({ poemFontScale });
  if (isChange)
    captureEvent('setting_changed', { setting: 'poem_font_scale', value: poemFontScale });
}
