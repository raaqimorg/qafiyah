import { describe, expect, it } from 'vitest';

import {
  clampFontScale,
  clampSpacingScale,
  DEFAULT_SETTINGS,
  FONT_SCALE_MAX,
  FONT_SCALE_MIN,
  parseSettings,
  SETTINGS_VERSION,
  serializeSettings,
  SPACING_SCALE_MAX,
  SPACING_SCALE_MIN,
} from './settings-schema';

describe('parseSettings', () => {
  it('returns defaults when nothing is stored', () => {
    expect(parseSettings(null, null)).toEqual(DEFAULT_SETTINGS);
  });

  it('returns defaults for unparseable or non-object JSON', () => {
    expect(parseSettings('{not json', null)).toEqual(DEFAULT_SETTINGS);
    expect(parseSettings('"a string"', null)).toEqual(DEFAULT_SETTINGS);
    expect(parseSettings('[1,2,3]', null)).toEqual(DEFAULT_SETTINGS);
    expect(parseSettings('null', null)).toEqual(DEFAULT_SETTINGS);
  });

  it('round-trips stored values', () => {
    const raw = JSON.stringify({
      v: SETTINGS_VERSION,
      theme: 'dark',
      poemFontScale: 1.2,
      poemSpacingScale: 0.6,
    });
    expect(parseSettings(raw, null)).toEqual({
      theme: 'dark',
      poemFontScale: 1.2,
      poemSpacingScale: 0.6,
    });
  });

  it('falls back per field, keeping the other valid fields intact', () => {
    const raw = JSON.stringify({ v: SETTINGS_VERSION, theme: 'chartreuse', poemFontScale: 1.3 });
    expect(parseSettings(raw, null)).toEqual({
      ...DEFAULT_SETTINGS,
      poemFontScale: 1.3,
    });

    const raw2 = JSON.stringify({ v: SETTINGS_VERSION, theme: 'dark', poemFontScale: 'huge' });
    expect(parseSettings(raw2, null)).toEqual({ ...DEFAULT_SETTINGS, theme: 'dark' });
  });

  it('fills in fields that are absent entirely, so new settings can be added later', () => {
    const raw = JSON.stringify({ v: SETTINGS_VERSION, theme: 'light' });
    expect(parseSettings(raw, null)).toEqual({ ...DEFAULT_SETTINGS, theme: 'light' });
  });

  it('still reads known fields written by a newer version', () => {
    const raw = JSON.stringify({
      v: SETTINGS_VERSION + 5,
      theme: 'dark',
      poemFontScale: 1.1,
      somethingWeHaveNeverHeardOf: { nested: true },
    });
    expect(parseSettings(raw, null)).toEqual({
      ...DEFAULT_SETTINGS,
      theme: 'dark',
      poemFontScale: 1.1,
    });
  });

  it('rejects an out-of-range font scale rather than trusting it', () => {
    const raw = JSON.stringify({ v: SETTINGS_VERSION, poemFontScale: 900 });
    expect(parseSettings(raw, null).poemFontScale).toBe(DEFAULT_SETTINGS.poemFontScale);
  });

  it('rejects an out-of-range spacing scale rather than trusting it', () => {
    const raw = JSON.stringify({ v: SETTINGS_VERSION, poemSpacingScale: 0 });
    expect(parseSettings(raw, null).poemSpacingScale).toBe(DEFAULT_SETTINGS.poemSpacingScale);
  });

  it('migrates the legacy theme key when no settings object exists yet', () => {
    expect(parseSettings(null, 'dark')).toEqual({ ...DEFAULT_SETTINGS, theme: 'dark' });
    expect(parseSettings(null, 'light')).toEqual({ ...DEFAULT_SETTINGS, theme: 'light' });
    expect(parseSettings(null, 'nonsense')).toEqual(DEFAULT_SETTINGS);
  });

  it('falls back to the legacy key when the stored value is unreadable', () => {
    expect(parseSettings('{corrupt', 'dark').theme).toBe('dark');
  });

  it('prefers the settings object over the legacy key once it exists', () => {
    const raw = JSON.stringify({ v: SETTINGS_VERSION, theme: 'light' });
    expect(parseSettings(raw, 'dark').theme).toBe('light');
  });
});

describe('serializeSettings', () => {
  it('writes a versioned object', () => {
    const out = JSON.parse(serializeSettings(null, { theme: 'dark' }));
    expect(out).toEqual({ v: SETTINGS_VERSION, theme: 'dark' });
  });

  it('merges into existing values instead of replacing them', () => {
    const existing = JSON.stringify({ v: SETTINGS_VERSION, theme: 'dark', poemFontScale: 1.2 });
    const out = JSON.parse(serializeSettings(existing, { poemFontScale: 0.9 }));
    expect(out).toEqual({ v: SETTINGS_VERSION, theme: 'dark', poemFontScale: 0.9 });
  });

  it('preserves keys it does not understand, so a newer build loses nothing', () => {
    const existing = JSON.stringify({
      v: SETTINGS_VERSION,
      theme: 'dark',
      futureSetting: 'keep me',
    });
    const out = JSON.parse(serializeSettings(existing, { theme: 'light' }));
    expect(out.futureSetting).toBe('keep me');
    expect(out.theme).toBe('light');
  });

  it('never downgrades a newer version marker', () => {
    const existing = JSON.stringify({ v: SETTINGS_VERSION + 3, theme: 'dark' });
    const out = JSON.parse(serializeSettings(existing, { theme: 'light' }));
    expect(out.v).toBe(SETTINGS_VERSION + 3);
  });

  it('recovers from corrupt existing data without throwing', () => {
    const out = JSON.parse(serializeSettings('{corrupt', { theme: 'dark' }));
    expect(out).toEqual({ v: SETTINGS_VERSION, theme: 'dark' });
  });
});

describe('clampFontScale', () => {
  it('passes a scale inside the range through, rounded to one decimal', () => {
    expect(clampFontScale(1)).toBe(1);
    expect(clampFontScale(1.24)).toBe(1.2);
  });

  it('clamps to the minimum and maximum', () => {
    expect(clampFontScale(0.1)).toBe(FONT_SCALE_MIN);
    expect(clampFontScale(9)).toBe(FONT_SCALE_MAX);
  });
});

describe('clampSpacingScale', () => {
  it('passes a scale inside the range through, rounded to one decimal', () => {
    expect(clampSpacingScale(1)).toBe(1);
    expect(clampSpacingScale(0.6000000001)).toBe(0.6);
  });

  it('clamps to the minimum and maximum', () => {
    expect(clampSpacingScale(0)).toBe(SPACING_SCALE_MIN);
    expect(clampSpacingScale(9)).toBe(SPACING_SCALE_MAX);
  });
});
