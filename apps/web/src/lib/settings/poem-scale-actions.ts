import { captureEvent } from '@/lib/analytics/capture-event';

import { clampFontScale, clampSpacingScale } from './settings-schema';
import { getSettings, updateSettings } from './settings-store';

export function changeFontScale(scale: number): void {
  const poemFontScale = clampFontScale(scale);
  const isChange = getSettings().poemFontScale !== poemFontScale;
  updateSettings({ poemFontScale });
  if (isChange)
    captureEvent('setting_changed', { setting: 'poem_font_scale', value: poemFontScale });
}

export function changeSpacingScale(scale: number): void {
  const poemSpacingScale = clampSpacingScale(scale);
  const isChange = getSettings().poemSpacingScale !== poemSpacingScale;
  updateSettings({ poemSpacingScale });
  if (isChange)
    captureEvent('setting_changed', { setting: 'poem_spacing_scale', value: poemSpacingScale });
}
