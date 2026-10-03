import { useCallback, useEffect, useRef } from 'react';

import type React from 'react';

declare global {
  interface Window {
    posthog?: { readonly capture: (event: string) => void };
  }
}

export function isPlainClick(event: React.MouseEvent): boolean {
  return event.button === 0 && !event.metaKey && !event.ctrlKey && !event.shiftKey && !event.altKey;
}

export function moveToPageTop(): void {
  window.scrollTo({ top: 0 });
  document.querySelector<HTMLElement>('#main')?.focus({ preventScroll: true });
}

export function usePageviews(): () => void {
  const lastSearchRef = useRef<string | null>(null);

  const capturePageview = useCallback(() => {
    const search = window.location.search;
    if (search === lastSearchRef.current) return;
    lastSearchRef.current = search;
    window.posthog?.capture('$pageview');
  }, []);

  useEffect(() => {
    lastSearchRef.current = window.location.search;
    window.addEventListener('popstate', capturePageview);
    return () => window.removeEventListener('popstate', capturePageview);
  }, [capturePageview]);

  return capturePageview;
}
