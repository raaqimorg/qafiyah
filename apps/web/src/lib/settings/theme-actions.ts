import { captureEvent } from '@/lib/analytics/capture-event';

import { type Theme } from './settings-schema';
import { resolveIsDark } from './settings-storage';
import { getSettings, updateSettings } from './settings-store';

function changeTheme(theme: Theme): void {
  const isChange = getSettings().theme !== theme;
  updateSettings({ theme });
  if (isChange) captureEvent('setting_changed', { setting: 'theme', value: theme });
}

export function toggleTheme(): void {
  const isDark = resolveIsDark(getSettings().theme, (query) => window.matchMedia(query));
  changeTheme(isDark ? 'light' : 'dark');
}
