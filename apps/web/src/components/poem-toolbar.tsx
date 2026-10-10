'use client';

import { ListChevronsDownUp, ListChevronsUpDown, Minus, Plus, SunMoon } from 'lucide-react';

import { IconButton } from '@/components/ui/icon-button';
import { captureEvent } from '@/lib/analytics/capture-event';
import { POEM_TOOLBAR_TEXTS } from '@/lib/constants/copy';
import { FONT_SCALE, SPACING_SCALE, stepScale } from '@/lib/poem-scale';
import { toggleTheme } from '@/lib/settings/theme-actions';
import { cn } from '@/lib/utils';

const DIMMED = 'opacity-75 transition hover:opacity-100';
const ICON = 'size-5';
const ICON_STROKE = 1.5;

type PoemToolbarProps = {
  readonly fontScale: number;
  readonly spacingScale: number;
  readonly showTashkeel: boolean;
  readonly onFontScaleChange: (scale: number) => void;
  readonly onSpacingScaleChange: (scale: number) => void;
  readonly onToggleTashkeel: () => void;
};

export function PoemToolbar({
  fontScale,
  spacingScale,
  showTashkeel,
  onFontScaleChange,
  onSpacingScaleChange,
  onToggleTashkeel,
}: PoemToolbarProps) {
  const stepFontScale = (direction: 1 | -1) => {
    const next = stepScale(fontScale, direction, FONT_SCALE);
    onFontScaleChange(next);
    captureEvent('setting_changed', { setting: 'poem_font_scale', value: next });
  };
  const stepSpacingScale = (direction: 1 | -1) => {
    const next = stepScale(spacingScale, direction, SPACING_SCALE);
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
        disabled={fontScale <= FONT_SCALE.min}
        aria-label={POEM_TOOLBAR_TEXTS.fontSizeDecrease}
        className={DIMMED}
      >
        <Minus className={ICON} strokeWidth={ICON_STROKE} />
      </IconButton>
      <IconButton
        onClick={() => stepFontScale(1)}
        disabled={fontScale >= FONT_SCALE.max}
        aria-label={POEM_TOOLBAR_TEXTS.fontSizeIncrease}
        className={DIMMED}
      >
        <Plus className={ICON} strokeWidth={ICON_STROKE} />
      </IconButton>
      <IconButton
        onClick={() => stepSpacingScale(-1)}
        disabled={spacingScale <= SPACING_SCALE.min}
        aria-label={POEM_TOOLBAR_TEXTS.spacingDecrease}
        className={DIMMED}
      >
        <ListChevronsDownUp className={ICON} strokeWidth={ICON_STROKE} />
      </IconButton>
      <IconButton
        onClick={() => stepSpacingScale(1)}
        disabled={spacingScale >= SPACING_SCALE.max}
        aria-label={POEM_TOOLBAR_TEXTS.spacingIncrease}
        className={DIMMED}
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
