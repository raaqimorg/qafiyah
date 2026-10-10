'use client';

import { Minus, Plus, X } from 'lucide-react';
import { type RefObject, useEffect, useRef } from 'react';

import { IconButton } from '@/components/ui/icon-button';
import { SETTINGS_TEXTS } from '@/lib/constants/copy';
import { changeFontScale } from '@/lib/settings/poem-scale-actions';
import {
  DEFAULT_SETTINGS,
  FONT_SCALE_MAX,
  FONT_SCALE_MIN,
  FONT_SCALE_STEP,
  type Theme,
} from '@/lib/settings/settings-schema';
import { changeTheme } from '@/lib/settings/theme-actions';
import { useSettings } from '@/lib/settings/use-settings';
import { cn } from '@/lib/utils';

const THEME_OPTIONS: readonly { readonly value: Theme; readonly label: string }[] = [
  { value: 'system', label: SETTINGS_TEXTS.themeSystem },
  { value: 'light', label: SETTINGS_TEXTS.themeLight },
  { value: 'dark', label: SETTINGS_TEXTS.themeDark },
];

const TRACK = 'bg-surface-sunken flex gap-1 rounded-lg p-1';

function useLightDismiss(dialogRef: RefObject<HTMLDialogElement | null>): void {
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    let pressedBackdrop = false;
    const onPointerDown = (event: PointerEvent) => {
      pressedBackdrop = event.target === dialog;
    };
    const onClick = (event: MouseEvent) => {
      if (pressedBackdrop && event.target === dialog) dialog.close();
      pressedBackdrop = false;
    };

    dialog.addEventListener('pointerdown', onPointerDown);
    dialog.addEventListener('click', onClick);
    return () => {
      dialog.removeEventListener('pointerdown', onPointerDown);
      dialog.removeEventListener('click', onClick);
    };
  }, [dialogRef]);
}

export function SettingsDialog({ className }: { readonly className?: string }) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const settings = useSettings();
  useLightDismiss(dialogRef);

  return (
    <>
      <button
        type="button"
        onClick={() => dialogRef.current?.showModal()}
        className={cn(
          'rounded-sm text-text-muted focus-ring transition-colors hover:text-text',
          className
        )}
      >
        {SETTINGS_TEXTS.title}
      </button>

      <dialog
        ref={dialogRef}
        dir="rtl"
        className="m-auto w-[calc(100%-2rem)] max-w-sm rounded-xl border border-border bg-surface-raised p-0 text-text shadow-xl backdrop:bg-black/50 backdrop:backdrop-blur-sm"
      >
        <div className="flex flex-col gap-7 p-6">
          <div className="flex items-center justify-between">
            <h2 className="text-interface-heading text-text">{SETTINGS_TEXTS.title}</h2>
            <IconButton
              onClick={() => dialogRef.current?.close()}
              aria-label={SETTINGS_TEXTS.close}
              className="-ml-3"
            >
              <X className="h-5 w-5" strokeWidth={1.5} />
            </IconButton>
          </div>

          <section className="flex flex-col gap-3">
            <h3 className="text-sm text-text-muted">{SETTINGS_TEXTS.themeLabel}</h3>
            <div className={TRACK}>
              {THEME_OPTIONS.map((option) => (
                <button
                  key={option.value}
                  type="button"
                  aria-pressed={settings.theme === option.value}
                  onClick={() => changeTheme(option.value)}
                  className={cn(
                    'h-9 flex-1 rounded-md text-sm focus-ring transition-colors',
                    settings.theme === option.value
                      ? 'bg-surface text-text shadow-sm'
                      : 'text-text-subtle hover:text-text'
                  )}
                >
                  {option.label}
                </button>
              ))}
            </div>
          </section>

          <section className="flex flex-col gap-3">
            <div className="flex min-h-5 items-center justify-between">
              <h3 className="text-sm text-text-muted">{SETTINGS_TEXTS.fontSizeLabel}</h3>
              {settings.poemFontScale !== DEFAULT_SETTINGS.poemFontScale && (
                <button
                  type="button"
                  onClick={() => changeFontScale(DEFAULT_SETTINGS.poemFontScale)}
                  className="rounded-sm text-xs text-text-subtle focus-ring transition-colors hover:text-text"
                >
                  {SETTINGS_TEXTS.reset}
                </button>
              )}
            </div>
            <div className={cn(TRACK, 'items-center justify-between')}>
              <IconButton
                onClick={() => changeFontScale(settings.poemFontScale - FONT_SCALE_STEP)}
                disabled={settings.poemFontScale <= FONT_SCALE_MIN}
                aria-label={SETTINGS_TEXTS.fontSizeDecrease}
                className="min-h-9 min-w-9 hover:bg-surface"
              >
                <Minus className="h-4 w-4" strokeWidth={1.5} />
              </IconButton>
              <p className="text-sm text-text tabular-nums">
                {Math.round(settings.poemFontScale * 100)}%
              </p>
              <IconButton
                onClick={() => changeFontScale(settings.poemFontScale + FONT_SCALE_STEP)}
                disabled={settings.poemFontScale >= FONT_SCALE_MAX}
                aria-label={SETTINGS_TEXTS.fontSizeIncrease}
                className="min-h-9 min-w-9 hover:bg-surface"
              >
                <Plus className="h-4 w-4" strokeWidth={1.5} />
              </IconButton>
            </div>
          </section>
        </div>
      </dialog>
    </>
  );
}
