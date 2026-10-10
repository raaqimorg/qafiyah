import * as v from 'valibot';

export const SETTINGS_STORAGE_KEY = 'qafiyah:settings';

export const SETTINGS_VERSION = 1;

export const FONT_SCALE_MIN = 0.7;
export const FONT_SCALE_MAX = 1.5;
export const FONT_SCALE_STEP = 0.1;

export const SPACING_SCALE_MIN = 0.4;
export const SPACING_SCALE_MAX = 1.8;
export const SPACING_SCALE_STEP = 0.2;

export type Theme = 'system' | 'light' | 'dark';

export type Settings = {
  readonly theme: Theme;
  readonly poemFontScale: number;
  readonly poemSpacingScale: number;
};

export const DEFAULT_SETTINGS: Settings = {
  theme: 'system',
  poemFontScale: 1,
  poemSpacingScale: 1,
};

const themeSchema = v.picklist(['system', 'light', 'dark']);
const legacyThemeSchema = v.picklist(['light', 'dark']);
const fontScaleSchema = v.pipe(v.number(), v.minValue(FONT_SCALE_MIN), v.maxValue(FONT_SCALE_MAX));
const spacingScaleSchema = v.pipe(
  v.number(),
  v.minValue(SPACING_SCALE_MIN),
  v.maxValue(SPACING_SCALE_MAX)
);

type StoredRecord = Record<string, unknown>;

function isRecord(value: unknown): value is StoredRecord {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function toRecord(raw: string | null | undefined): StoredRecord | null {
  if (raw === null || raw === undefined || raw === '') return null;
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!isRecord(parsed)) return null;
    return parsed;
  } catch {
    return null;
  }
}

function pick<T>(schema: v.GenericSchema<unknown, T>, value: unknown, fallback: T): T {
  const result = v.safeParse(schema, value);
  return result.success ? result.output : fallback;
}

export function parseSettings(raw: string | null, legacyTheme: string | null): Settings {
  const stored = toRecord(raw);

  if (stored === null) {
    return {
      ...DEFAULT_SETTINGS,
      theme: pick(legacyThemeSchema, legacyTheme, DEFAULT_SETTINGS.theme),
    };
  }

  return {
    theme: pick(themeSchema, stored['theme'], DEFAULT_SETTINGS.theme),
    poemFontScale: pick(fontScaleSchema, stored['poemFontScale'], DEFAULT_SETTINGS.poemFontScale),
    poemSpacingScale: pick(
      spacingScaleSchema,
      stored['poemSpacingScale'],
      DEFAULT_SETTINGS.poemSpacingScale
    ),
  };
}

export function serializeSettings(raw: string | null, patch: Partial<Settings>): string {
  const stored = toRecord(raw) ?? {};
  const storedVersion = pick(v.pipe(v.number(), v.integer()), stored['v'], 0);

  return JSON.stringify({
    ...stored,
    ...patch,
    v: Math.max(storedVersion, SETTINGS_VERSION),
  });
}

function clampScale(scale: number, min: number, max: number): number {
  const clamped = Math.min(max, Math.max(min, scale));
  return Math.round(clamped * 10) / 10;
}

export function clampFontScale(scale: number): number {
  return clampScale(scale, FONT_SCALE_MIN, FONT_SCALE_MAX);
}

export function clampSpacingScale(scale: number): number {
  return clampScale(scale, SPACING_SCALE_MIN, SPACING_SCALE_MAX);
}
