'use client';

import {
  AArrowDown,
  AArrowUp,
  ListChevronsDownUp,
  ListChevronsUpDown,
  Moon,
  Sun,
} from 'lucide-react';

import { IconButton } from '@/components/ui/icon-button';
import { POEM_TOOLBAR_TEXTS, SETTINGS_TEXTS } from '@/lib/constants/copy';
import { changeFontScale, changeSpacingScale } from '@/lib/settings/poem-scale-actions';
import {
  FONT_SCALE_MAX,
  FONT_SCALE_MIN,
  FONT_SCALE_STEP,
  SPACING_SCALE_MAX,
  SPACING_SCALE_MIN,
  SPACING_SCALE_STEP,
} from '@/lib/settings/settings-schema';
import { toggleTheme } from '@/lib/settings/theme-actions';
import { useSettings } from '@/lib/settings/use-settings';
import { cn } from '@/lib/utils';

const DIMMED = 'opacity-75 transition hover:opacity-100';
const ICON = 'size-5';
const ICON_STROKE = 1.5;

type PoemToolbarProps = {
  readonly showTashkeel: boolean;
  readonly onToggleTashkeel: () => void;
};

export function PoemToolbar({ showTashkeel, onToggleTashkeel }: PoemToolbarProps) {
  const { poemFontScale, poemSpacingScale } = useSettings();
  return (
    <div
      role="group"
      aria-label={POEM_TOOLBAR_TEXTS.label}
      className="flex flex-wrap items-center justify-center gap-1"
    >
      <IconButton
        onClick={() => changeFontScale(poemFontScale - FONT_SCALE_STEP)}
        disabled={poemFontScale <= FONT_SCALE_MIN}
        aria-label={SETTINGS_TEXTS.fontSizeDecrease}
        className={DIMMED}
      >
        <AArrowDown className={ICON} strokeWidth={ICON_STROKE} />
      </IconButton>
      <IconButton
        onClick={() => changeFontScale(poemFontScale + FONT_SCALE_STEP)}
        disabled={poemFontScale >= FONT_SCALE_MAX}
        aria-label={SETTINGS_TEXTS.fontSizeIncrease}
        className={DIMMED}
      >
        <AArrowUp className={ICON} strokeWidth={ICON_STROKE} />
      </IconButton>
      <IconButton
        onClick={() => changeSpacingScale(poemSpacingScale - SPACING_SCALE_STEP)}
        disabled={poemSpacingScale <= SPACING_SCALE_MIN}
        aria-label={POEM_TOOLBAR_TEXTS.spacingDecrease}
        className={DIMMED}
      >
        <ListChevronsDownUp className={ICON} strokeWidth={ICON_STROKE} />
      </IconButton>
      <IconButton
        onClick={() => changeSpacingScale(poemSpacingScale + SPACING_SCALE_STEP)}
        disabled={poemSpacingScale >= SPACING_SCALE_MAX}
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
        <Moon className={cn(ICON, 'dark:hidden')} strokeWidth={ICON_STROKE} />
        <Sun className={cn(ICON, 'hidden dark:block')} strokeWidth={ICON_STROKE} />
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
