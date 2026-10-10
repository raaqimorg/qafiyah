import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  FONT_SCALE_MAX,
  FONT_SCALE_MIN,
  SPACING_SCALE_MAX,
  SPACING_SCALE_MIN,
} from './settings-schema';

function windowWithBlockedStorage() {
  return {
    get localStorage(): Storage {
      throw new DOMException('Access is denied for this document.', 'SecurityError');
    },
    addEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  };
}

async function freshModules() {
  vi.resetModules();
  const actions = await import('./poem-scale-actions');
  const store = await import('./settings-store');
  return { ...actions, ...store };
}

describe('the poem scale actions', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('keeps the font scale within its limits', async () => {
    vi.stubGlobal('window', windowWithBlockedStorage());
    const { changeFontScale, getSettings } = await freshModules();
    changeFontScale(FONT_SCALE_MAX + 1);
    expect(getSettings().poemFontScale).toBe(FONT_SCALE_MAX);
    changeFontScale(FONT_SCALE_MIN - 1);
    expect(getSettings().poemFontScale).toBe(FONT_SCALE_MIN);
  });

  it('rounds a stepped font scale to one decimal place', async () => {
    vi.stubGlobal('window', windowWithBlockedStorage());
    const { changeFontScale, getSettings } = await freshModules();
    changeFontScale(1 + 0.1 + 0.1);
    expect(getSettings().poemFontScale).toBe(1.2);
  });

  it('keeps the spacing scale within its limits', async () => {
    vi.stubGlobal('window', windowWithBlockedStorage());
    const { changeSpacingScale, getSettings } = await freshModules();
    changeSpacingScale(SPACING_SCALE_MAX + 1);
    expect(getSettings().poemSpacingScale).toBe(SPACING_SCALE_MAX);
    changeSpacingScale(SPACING_SCALE_MIN - 1);
    expect(getSettings().poemSpacingScale).toBe(SPACING_SCALE_MIN);
  });

  it('changes the spacing without touching the font scale', async () => {
    vi.stubGlobal('window', windowWithBlockedStorage());
    const { changeSpacingScale, getSettings } = await freshModules();
    changeSpacingScale(1.4);
    expect(getSettings().poemSpacingScale).toBe(1.4);
    expect(getSettings().poemFontScale).toBe(1);
  });
});
