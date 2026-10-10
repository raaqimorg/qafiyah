import { captureEvent } from '@/lib/analytics/capture-event';

import { DEFAULT_SETTINGS, FONT_FAMILIES, type FontFamily } from './settings-schema';
import { updateSettings } from './settings-store';

const NEXT_FONT_FAMILY = {
  amiri: 'thmanyah',
  thmanyah: 'scheherazade',
  scheherazade: 'amiri',
} as const satisfies Record<FontFamily, FontFamily>;

function pageFontFamily(): FontFamily {
  const current = document.documentElement.dataset['font'];
  return FONT_FAMILIES.find((family) => family === current) ?? DEFAULT_SETTINGS.fontFamily;
}

export function cycleFontFamily(): void {
  const fontFamily = NEXT_FONT_FAMILY[pageFontFamily()];
  updateSettings({ fontFamily });
  captureEvent('setting_changed', { setting: 'font_family', value: fontFamily });
}
