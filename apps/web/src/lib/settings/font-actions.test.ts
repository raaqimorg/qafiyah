import { afterEach, describe, expect, it, vi } from 'vitest';

function storageWith(fontFamily: string | null): Pick<Storage, 'getItem' | 'setItem'> {
  const values = new Map<string, string>();
  if (fontFamily !== null) values.set('qafiyah:settings', JSON.stringify({ v: 1, fontFamily }));
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => {
      values.set(key, value);
    },
  };
}

async function freshModules(options: { readonly stored: string | null; readonly page?: string }) {
  vi.resetModules();
  vi.stubGlobal('window', {
    localStorage: storageWith(options.stored),
    matchMedia: () => ({ matches: false }),
    addEventListener: () => {},
  });
  const dataset: Record<string, string> = options.page === undefined ? {} : { font: options.page };
  const page = { documentElement: { dataset } };
  vi.stubGlobal('document', page);
  const actions = await import('./font-actions');
  const store = await import('./settings-store');
  return { ...actions, ...store, page };
}

describe('cycleFontFamily', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('moves from Amiri to Thmanyah', async () => {
    const { cycleFontFamily, getSettings, page } = await freshModules({
      stored: null,
      page: 'amiri',
    });
    cycleFontFamily();
    expect(getSettings().fontFamily).toBe('thmanyah');
    expect(page.documentElement.dataset['font']).toBe('thmanyah');
  });

  it('moves from Thmanyah to Scheherazade', async () => {
    const { cycleFontFamily, getSettings } = await freshModules({
      stored: 'thmanyah',
      page: 'thmanyah',
    });
    cycleFontFamily();
    expect(getSettings().fontFamily).toBe('scheherazade');
  });

  it('wraps from Scheherazade back to Amiri', async () => {
    const { cycleFontFamily, getSettings } = await freshModules({
      stored: 'scheherazade',
      page: 'scheherazade',
    });
    cycleFontFamily();
    expect(getSettings().fontFamily).toBe('amiri');
  });

  it('treats a page without a font as Amiri', async () => {
    const { cycleFontFamily, page } = await freshModules({ stored: null });
    cycleFontFamily();
    expect(page.documentElement.dataset['font']).toBe('thmanyah');
  });

  it('follows the page, not a font that another tab saved since it loaded', async () => {
    const { cycleFontFamily, page } = await freshModules({ stored: 'scheherazade', page: 'amiri' });
    cycleFontFamily();
    expect(page.documentElement.dataset['font']).toBe('thmanyah');
  });
});
