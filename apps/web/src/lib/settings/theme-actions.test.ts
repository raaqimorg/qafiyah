import { afterEach, describe, expect, it, vi } from 'vitest';

function storageWith(theme: string | null): Pick<Storage, 'getItem' | 'setItem'> {
  const values = new Map<string, string>();
  if (theme !== null) values.set('qafiyah:settings', JSON.stringify({ v: 1, theme }));
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => {
      values.set(key, value);
    },
  };
}

function pageShowing(isDark: boolean) {
  const classes = new Set(isDark ? ['dark'] : []);
  return {
    documentElement: {
      classList: {
        contains: (name: string) => classes.has(name),
        toggle: (name: string, force: boolean) => {
          if (force) classes.add(name);
          else classes.delete(name);
          return force;
        },
      },
      style: {},
    },
    querySelector: () => null,
  };
}

async function freshModules(options: { readonly stored: string | null; readonly dark: boolean }) {
  vi.resetModules();
  vi.stubGlobal('window', {
    localStorage: storageWith(options.stored),
    matchMedia: () => ({ matches: false }),
    addEventListener: () => {},
  });
  const page = pageShowing(options.dark);
  vi.stubGlobal('document', page);
  const actions = await import('./theme-actions');
  const store = await import('./settings-store');
  return { ...actions, ...store, page };
}

describe('toggleTheme', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('switches a light page to dark', async () => {
    const { toggleTheme, getSettings, page } = await freshModules({ stored: null, dark: false });
    toggleTheme();
    expect(getSettings().theme).toBe('dark');
    expect(page.documentElement.classList.contains('dark')).toBe(true);
  });

  it('switches a dark page to light', async () => {
    const { toggleTheme, getSettings, page } = await freshModules({ stored: 'dark', dark: true });
    toggleTheme();
    expect(getSettings().theme).toBe('light');
    expect(page.documentElement.classList.contains('dark')).toBe(false);
  });

  it('follows the page, not a theme that another tab saved since it loaded', async () => {
    const { toggleTheme, page } = await freshModules({ stored: 'dark', dark: false });
    toggleTheme();
    expect(page.documentElement.classList.contains('dark')).toBe(true);
  });
});
