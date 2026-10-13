import { browserStorage } from '@/lib/browser-storage';
import { VIEWER_HINT_COOKIE, VIEWER_STORAGE_KEY } from '@/lib/constants/config';
import { readCachedViewer, type Viewer } from '@/lib/viewer-cache';
import { viewerHintCookie } from '@/lib/viewer-hint';

import type { ViewerPayload } from '@/lib/server/viewer-response';

const AVATAR_PX = 48;
const AVATAR_RETRY_MS = 5 * 60 * 1000;

export type ViewerLoad =
  | { readonly kind: 'anonymous' }
  | { readonly kind: 'signed-in'; readonly viewer: Viewer }
  | { readonly kind: 'failed' };

export function hasViewerHint(): boolean {
  return document.cookie.split('; ').includes(`${VIEWER_HINT_COOKIE}=1`);
}

function syncViewerHint(signedIn: boolean): void {
  if (hasViewerHint() !== signedIn) document.cookie = viewerHintCookie(signedIn);
  const root = document.documentElement;
  if (signedIn) root.dataset['viewer'] ??= 'signed-in';
  else delete root.dataset['viewer'];
}

function writeCachedViewer(viewer: Viewer | undefined): void {
  try {
    if (viewer === undefined) localStorage.removeItem(VIEWER_STORAGE_KEY);
    else localStorage.setItem(VIEWER_STORAGE_KEY, JSON.stringify(viewer));
  } catch {}
}

async function encodeAvatar(url: string): Promise<string | null> {
  try {
    const response = await fetch(url, {
      mode: 'cors',
      credentials: 'omit',
      referrerPolicy: 'no-referrer',
    });
    if (!response.ok) return null;
    const bitmap = await createImageBitmap(await response.blob(), {
      resizeWidth: AVATAR_PX,
      resizeHeight: AVATAR_PX,
      resizeQuality: 'high',
    });
    const canvas = document.createElement('canvas');
    canvas.width = AVATAR_PX;
    canvas.height = AVATAR_PX;
    canvas.getContext('2d')?.drawImage(bitmap, 0, 0);
    bitmap.close();
    return canvas.toDataURL('image/webp', 0.9);
  } catch {
    return null;
  }
}

function pickAvatar(viewer: Viewer): Pick<Viewer, 'avatarData' | 'avatarFailedAt'> {
  return { avatarData: viewer.avatarData, avatarFailedAt: viewer.avatarFailedAt };
}

async function resolveAvatar(payload: ViewerPayload): Promise<Viewer> {
  if (payload.avatarUrl === null) return { ...payload, avatarData: null, avatarFailedAt: null };
  const storage = browserStorage();
  const previous = storage === undefined ? undefined : readCachedViewer(storage);
  if (previous !== undefined && previous.avatarUrl === payload.avatarUrl) {
    if (previous.avatarData !== null) return { ...payload, ...pickAvatar(previous) };
    const failedAt = previous.avatarFailedAt;
    if (failedAt !== null && Date.now() - failedAt < AVATAR_RETRY_MS) {
      return { ...payload, ...pickAvatar(previous) };
    }
  }
  const avatarData = await encodeAvatar(payload.avatarUrl);
  return { ...payload, avatarData, avatarFailedAt: avatarData === null ? Date.now() : null };
}

async function fetchViewer(): Promise<ViewerLoad> {
  try {
    const response = await fetch('/api/me', { credentials: 'same-origin' });
    // oxlint-disable-next-line typescript/no-unsafe-assignment -- the /api/me response is the app's own typed contract
    const payload: ViewerPayload | undefined =
      response.status === 204 ? undefined : await response.json();
    syncViewerHint(payload !== undefined);
    if (payload === undefined) {
      writeCachedViewer(undefined);
      return { kind: 'anonymous' };
    }
    const viewer = await resolveAvatar(payload);
    writeCachedViewer(viewer);
    return { kind: 'signed-in', viewer };
  } catch {
    return { kind: 'failed' };
  }
}

let shared: Promise<ViewerLoad> | undefined;

export function loadViewer(): Promise<ViewerLoad> {
  shared ??= fetchViewer();
  return shared;
}
