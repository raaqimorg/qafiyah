import { describe, expect, it } from 'vitest';

import {
  DEFAULT_SETTINGS,
  parseSettings,
  SETTINGS_VERSION,
  serializeSettings,
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
    const raw = JSON.stringify({ v: SETTINGS_VERSION, theme: 'dark', fontFamily: 'scheherazade' });
    expect(parseSettings(raw, null)).toEqual({ theme: 'dark', fontFamily: 'scheherazade' });
  });

  it('falls back to the default for an unknown theme', () => {
    const raw = JSON.stringify({ v: SETTINGS_VERSION, theme: 'chartreuse' });
    expect(parseSettings(raw, null)).toEqual(DEFAULT_SETTINGS);
  });

  it('falls back to the default font family for an unknown font, keeping the theme', () => {
    const raw = JSON.stringify({ v: SETTINGS_VERSION, theme: 'dark', fontFamily: 'comic-sans' });
    expect(parseSettings(raw, null)).toEqual({ ...DEFAULT_SETTINGS, theme: 'dark' });
  });

  it('fills in fields that are absent entirely, so new settings can be added later', () => {
    const raw = JSON.stringify({ v: SETTINGS_VERSION });
    expect(parseSettings(raw, null)).toEqual(DEFAULT_SETTINGS);
  });

  it('ignores the font size and spacing that older builds saved', () => {
    const raw = JSON.stringify({ v: SETTINGS_VERSION, theme: 'dark', poemFontScale: 1.3 });
    expect(parseSettings(raw, null)).toEqual({ ...DEFAULT_SETTINGS, theme: 'dark' });
  });

  it('still reads known fields written by a newer version', () => {
    const raw = JSON.stringify({
      v: SETTINGS_VERSION + 5,
      theme: 'dark',
      somethingWeHaveNeverHeardOf: { nested: true },
    });
    expect(parseSettings(raw, null)).toEqual({ ...DEFAULT_SETTINGS, theme: 'dark' });
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
    const out = JSON.parse(serializeSettings(existing, { theme: 'light' }));
    expect(out).toEqual({ v: SETTINGS_VERSION, theme: 'light', poemFontScale: 1.2 });
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
