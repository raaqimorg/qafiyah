import { captureEvent } from '@/lib/analytics/capture-event';

import { DEFAULT_SETTINGS, FONT_FAMILIES, type FontFamily } from './settings-schema';
import { updateSettings } from './settings-store';

function pageFontFamily(): FontFamily {
  const current = document.documentElement.dataset['font'];
  return FONT_FAMILIES.find((family) => family === current) ?? DEFAULT_SETTINGS.fontFamily;
}

export function cycleFontFamily(): void {
  const next = FONT_FAMILIES.indexOf(pageFontFamily()) + 1;
  const fontFamily = FONT_FAMILIES[next % FONT_FAMILIES.length] ?? DEFAULT_SETTINGS.fontFamily;
  updateSettings({ fontFamily });
  captureEvent('setting_changed', { setting: 'font_family', value: fontFamily });
}
