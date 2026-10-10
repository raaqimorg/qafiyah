'use client';

import { ListChevronsDownUp, ListChevronsUpDown, Minus, Plus, SunMoon, Type } from 'lucide-react';

import { IconButton } from '@/components/ui/icon-button';
import { captureEvent } from '@/lib/analytics/capture-event';
import { POEM_TOOLBAR_TEXTS } from '@/lib/constants/copy';
import { POEM_SCALE, stepScale } from '@/lib/poem-scale';
import { cycleFontFamily } from '@/lib/settings/font-actions';
import { toggleTheme } from '@/lib/settings/theme-actions';
import { cn } from '@/lib/utils';

const DIMMED = 'opacity-75 transition hover:opacity-100';
const STEP_BUTTON = cn(DIMMED, 'aria-disabled:pointer-events-none aria-disabled:opacity-50');
const ICON = 'size-5';
const ICON_STROKE = 1.5;

type PoemToolbarProps = {
  readonly fontScale: number;
  readonly spacingScale: number;
  readonly spacingMin: number;
  readonly showTashkeel: boolean;
  readonly onFontScaleChange: (scale: number) => void;
  readonly onSpacingScaleChange: (scale: number) => void;
  readonly onToggleTashkeel: () => void;
};

export function PoemToolbar({
  fontScale,
  spacingScale,
  spacingMin,
  showTashkeel,
  onFontScaleChange,
  onSpacingScaleChange,
  onToggleTashkeel,
}: PoemToolbarProps) {
  const spacingRange = { ...POEM_SCALE.spacing, min: spacingMin };
  const stepFontScale = (direction: 1 | -1) => {
    const next = stepScale(fontScale, direction, POEM_SCALE.font);
    if (next === fontScale) return;
    onFontScaleChange(next);
    captureEvent('setting_changed', { setting: 'poem_font_scale', value: next });
  };
  const stepSpacingScale = (direction: 1 | -1) => {
    const next = stepScale(spacingScale, direction, spacingRange);
    if (next === spacingScale) return;
    onSpacingScaleChange(next);
    captureEvent('setting_changed', { setting: 'poem_spacing_scale', value: next });
  };
  return (
    <div
      role="group"
      aria-label={POEM_TOOLBAR_TEXTS.label}
      className="flex flex-wrap items-center justify-center gap-1 select-none"
    >
      <IconButton
        onClick={() => stepFontScale(-1)}
        aria-disabled={fontScale <= POEM_SCALE.font.min}
        aria-label={POEM_TOOLBAR_TEXTS.fontSizeDecrease}
        className={STEP_BUTTON}
      >
        <Minus className={ICON} strokeWidth={ICON_STROKE} />
      </IconButton>
      <IconButton
        onClick={() => stepFontScale(1)}
        aria-disabled={fontScale >= POEM_SCALE.font.max}
        aria-label={POEM_TOOLBAR_TEXTS.fontSizeIncrease}
        className={STEP_BUTTON}
      >
        <Plus className={ICON} strokeWidth={ICON_STROKE} />
      </IconButton>
      <IconButton
        onClick={() => stepSpacingScale(-1)}
        aria-disabled={spacingScale <= spacingMin}
        aria-label={POEM_TOOLBAR_TEXTS.spacingDecrease}
        className={STEP_BUTTON}
      >
        <ListChevronsDownUp className={ICON} strokeWidth={ICON_STROKE} />
      </IconButton>
      <IconButton
        onClick={() => stepSpacingScale(1)}
        aria-disabled={spacingScale >= POEM_SCALE.spacing.max}
        aria-label={POEM_TOOLBAR_TEXTS.spacingIncrease}
        className={STEP_BUTTON}
      >
        <ListChevronsUpDown className={ICON} strokeWidth={ICON_STROKE} />
      </IconButton>
      <IconButton
        onClick={toggleTheme}
        aria-label={POEM_TOOLBAR_TEXTS.toggleTheme}
        className={DIMMED}
      >
        <SunMoon className={ICON} strokeWidth={ICON_STROKE} />
      </IconButton>
      <IconButton
        onClick={cycleFontFamily}
        aria-label={POEM_TOOLBAR_TEXTS.cycleFontFamily}
        className={DIMMED}
      >
        <Type className={ICON} strokeWidth={ICON_STROKE} />
      </IconButton>
      <button
        type="button"
        aria-pressed={showTashkeel}
        onClick={onToggleTashkeel}
        className={cn(
          'min-h-11 rounded-md px-3 text-sm focus-ring hover:text-text',
          showTashkeel ? 'bg-surface-sunken text-text-muted' : cn('text-text-subtle', DIMMED)
        )}
      >
        {POEM_TOOLBAR_TEXTS.tashkeel}
      </button>
    </div>
  );
}
