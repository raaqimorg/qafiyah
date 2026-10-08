'use client';

import { Loader2 } from 'lucide-react';
import { useEffect, useState } from 'react';
import { match } from 'ts-pattern';

import { captureEvent } from '@/lib/analytics/capture-event';
import { fetchRandomPoemSlugWithRetry } from '@/lib/api/random-poem';
import { WEB_API_PROXY_PREFIX } from '@/lib/constants/config';
import { reportError } from '@/lib/observability/report-error';
import { poemUrl } from '@/lib/urls';

import type { MouseEvent } from 'react';

const RANDOM_POEM_HREF = '/poems/random';

type RandomPoemStatus =
  | { readonly kind: 'idle' }
  | { readonly kind: 'loading' }
  | { readonly kind: 'error' };

export function RandomPoemButton() {
  const [status, setStatus] = useState<RandomPoemStatus>({ kind: 'idle' });

  useEffect(() => {
    const handlePageShow = (event: PageTransitionEvent) => {
      if (event.persisted) setStatus({ kind: 'idle' });
    };
    window.addEventListener('pageshow', handlePageShow);
    return () => window.removeEventListener('pageshow', handlePageShow);
  }, []);

  const handleClick = async (event: MouseEvent<HTMLAnchorElement>) => {
    if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) {
      return;
    }
    event.preventDefault();
    if (status.kind === 'loading') return;
    captureEvent('random_poem_requested');
    setStatus({ kind: 'loading' });
    const result = await fetchRandomPoemSlugWithRetry(WEB_API_PROXY_PREFIX);
    if (result.isErr()) {
      console.error('fetchRandomPoemSlug failed', result.error);
      reportError('fetchRandomPoemSlug failed', result.error, {
        feature: 'random-poem',
        tags: { surface: 'client', kind: result.error.kind },
      });
      setStatus({ kind: 'error' });
      return;
    }
    window.location.href = poemUrl(result.value);
  };

  const isLoading = status.kind === 'loading';

  return (
    <a
      href={RANDOM_POEM_HREF}
      onClick={(event) => {
        void handleClick(event);
      }}
      aria-busy={isLoading}
      aria-label="قصيدة عشوائية"
      className="-ml-1 inline-flex min-h-11 items-center justify-center rounded-md px-1 focus-ring"
    >
      {match(status)
        .with({ kind: 'error' }, () => (
          <span className="text-danger">تعذّر التحميل، اضغط لإعادة المحاولة</span>
        ))
        .with({ kind: 'loading' }, () => (
          <Loader2 className="h-4 w-4 animate-spin" aria-hidden="true" />
        ))
        .with({ kind: 'idle' }, () => <span>قصيدة عشوائية</span>)
        .exhaustive()}
    </a>
  );
}
