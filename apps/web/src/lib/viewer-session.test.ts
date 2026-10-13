import { afterEach, describe, expect, test, vi } from 'vitest';

import { VIEWER_HINT_COOKIE, VIEWER_STORAGE_KEY } from '@/lib/constants/config';

const SIGNED_IN = JSON.stringify({ displayName: 'أحمد', avatarUrl: null });
const CACHED = JSON.stringify({
  displayName: 'أحمد',
  avatarUrl: null,
  avatarData: null,
  avatarFailedAt: null,
});

function fakeBrowser(cookie: string, cached?: string) {
  const store = new Map<string, string>();
  if (cached !== undefined) store.set(VIEWER_STORAGE_KEY, cached);
  const storage = {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: vi.fn((key: string, value: string) => {
      store.set(key, value);
    }),
    removeItem: vi.fn((key: string) => {
      store.delete(key);
    }),
  };
  const document = { cookie, documentElement: { dataset: {} as Record<string, string> } };
  vi.stubGlobal('document', document);
  vi.stubGlobal('localStorage', storage);
  vi.stubGlobal('window', { localStorage: storage });
  return { document, storage, store };
}

function respondWith(...answers: (Response | Error)[]) {
  const fetch = vi.fn(async () => {
    const next = answers.shift();
    if (next === undefined) throw new Error('no answer left');
    if (next instanceof Error) throw next;
    return next;
  });
  vi.stubGlobal('fetch', fetch);
  return fetch;
}

async function freshSession() {
  vi.resetModules();
  return await import('./viewer-session');
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('loadViewer', () => {
  test('sends one /api/me request however many controls ask', async () => {
    fakeBrowser('');
    const fetch = respondWith(new Response(null, { status: 204 }));
    const { loadViewer } = await freshSession();

    const [first, second] = await Promise.all([loadViewer(), loadViewer()]);
    await loadViewer();

    expect(fetch).toHaveBeenCalledTimes(1);
    expect(fetch).toHaveBeenCalledWith('/api/me', { credentials: 'same-origin' });
    expect(second).toBe(first);
    expect(first).toEqual({ kind: 'anonymous' });
  });

  test('a signed-out answer clears the hint, the viewer attribute, and the cached viewer', async () => {
    const { document, storage, store } = fakeBrowser(`${VIEWER_HINT_COOKIE}=1`, CACHED);
    document.documentElement.dataset['viewer'] = 'signed-in';
    respondWith(new Response(null, { status: 204 }));
    const { loadViewer } = await freshSession();

    expect(await loadViewer()).toEqual({ kind: 'anonymous' });
    expect(document.cookie).toBe(`${VIEWER_HINT_COOKIE}=; Path=/; Secure; SameSite=Lax; Max-Age=0`);
    expect(document.documentElement.dataset['viewer']).toBeUndefined();
    expect(storage.removeItem).toHaveBeenCalledTimes(1);
    expect(store.has(VIEWER_STORAGE_KEY)).toBe(false);
  });

  test('a signed-in answer sets the hint and caches the viewer once for both controls', async () => {
    const { document, storage, store } = fakeBrowser('');
    respondWith(new Response(SIGNED_IN, { status: 200 }));
    const { loadViewer } = await freshSession();

    const [first] = await Promise.all([loadViewer(), loadViewer()]);

    expect(first).toEqual({
      kind: 'signed-in',
      viewer: { displayName: 'أحمد', avatarUrl: null, avatarData: null, avatarFailedAt: null },
    });
    expect(document.cookie).toContain(`${VIEWER_HINT_COOKIE}=1;`);
    expect(document.documentElement.dataset['viewer']).toBe('signed-in');
    expect(storage.setItem).toHaveBeenCalledTimes(1);
    expect(store.get(VIEWER_STORAGE_KEY)).toBe(CACHED);
  });

  test('fetches the avatar once for both controls', async () => {
    fakeBrowser('');
    const fetch = respondWith(
      new Response(JSON.stringify({ displayName: 'أحمد', avatarUrl: 'https://avatars.test/a' }), {
        status: 200,
      }),
      new Response(null, { status: 404 })
    );
    const { loadViewer } = await freshSession();

    const [first] = await Promise.all([loadViewer(), loadViewer()]);

    expect(fetch).toHaveBeenCalledTimes(2);
    expect(first).toMatchObject({
      kind: 'signed-in',
      viewer: { avatarUrl: 'https://avatars.test/a', avatarData: null },
    });
  });

  test('a failed request leaves the hint and the cached viewer alone', async () => {
    const { document, storage } = fakeBrowser(`${VIEWER_HINT_COOKIE}=1`, CACHED);
    respondWith(new TypeError('Failed to fetch'));
    const { loadViewer } = await freshSession();

    expect(await loadViewer()).toEqual({ kind: 'failed' });
    expect(document.cookie).toBe(`${VIEWER_HINT_COOKIE}=1`);
    expect(storage.setItem).not.toHaveBeenCalled();
    expect(storage.removeItem).not.toHaveBeenCalled();
  });

  test('an error page that is not JSON counts as failed', async () => {
    const { document, storage } = fakeBrowser(`${VIEWER_HINT_COOKIE}=1`, CACHED);
    respondWith(new Response('<html>Bad gateway</html>', { status: 502 }));
    const { loadViewer } = await freshSession();

    expect(await loadViewer()).toEqual({ kind: 'failed' });
    expect(document.cookie).toBe(`${VIEWER_HINT_COOKIE}=1`);
    expect(storage.removeItem).not.toHaveBeenCalled();
  });
});
