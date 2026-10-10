import { afterEach, describe, expect, it, vi } from 'vitest';

function windowWithDevice(prefersDark: boolean) {
  return {
    get localStorage(): Storage {
      throw new DOMException('Access is denied for this document.', 'SecurityError');
    },
    matchMedia: () => ({ matches: prefersDark }),
    addEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  };
}

async function freshModules() {
  vi.resetModules();
  const actions = await import('./theme-actions');
  const store = await import('./settings-store');
  return { ...actions, ...store };
}

describe('the theme actions', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('switches the system theme on a dark device to light', async () => {
    vi.stubGlobal('window', windowWithDevice(true));
    const { toggleTheme, getSettings } = await freshModules();
    toggleTheme();
    expect(getSettings().theme).toBe('light');
  });

  it('switches the system theme on a light device to dark', async () => {
    vi.stubGlobal('window', windowWithDevice(false));
    const { toggleTheme, getSettings } = await freshModules();
    toggleTheme();
    expect(getSettings().theme).toBe('dark');
  });

  it('switches a dark theme back to light', async () => {
    vi.stubGlobal('window', windowWithDevice(false));
    const { changeTheme, toggleTheme, getSettings } = await freshModules();
    changeTheme('dark');
    toggleTheme();
    expect(getSettings().theme).toBe('light');
  });
});
