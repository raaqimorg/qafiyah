import { captureEvent } from '@/lib/analytics/capture-event';

import { updateSettings } from './settings-store';

export function toggleTheme(): void {
  const theme = document.documentElement.classList.contains('dark') ? 'light' : 'dark';
  updateSettings({ theme });
  captureEvent('setting_changed', { setting: 'theme', value: theme });
}
