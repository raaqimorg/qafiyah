import { captureEvent } from '@/lib/analytics/capture-event';

import { type FontFamily } from './settings-schema';
import { updateSettings } from './settings-store';

const NEXT_FONT_FAMILY = {
  amiri: 'thmanyah',
  thmanyah: 'plex',
  plex: 'amiri',
} as const satisfies Record<FontFamily, FontFamily>;

function pageFontFamily(): FontFamily {
  const fontFamily = document.documentElement.dataset['font'];
  return fontFamily === 'thmanyah' || fontFamily === 'plex' ? fontFamily : 'amiri';
}

export function cycleFontFamily(): void {
  const fontFamily = NEXT_FONT_FAMILY[pageFontFamily()];
  updateSettings({ fontFamily });
  captureEvent('setting_changed', { setting: 'font_family', value: fontFamily });
}
