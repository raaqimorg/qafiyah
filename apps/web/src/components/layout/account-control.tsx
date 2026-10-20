import * as React from 'react';

import { browserStorage } from '@/lib/browser-storage';
import { VIEWER_STORAGE_KEY } from '@/lib/constants/config';
import { cn } from '@/lib/utils';
import { initialOf, stateFromSnapshot, type ViewerState } from '@/lib/viewer-cache';
import { hasViewerHint, loadViewer } from '@/lib/viewer-session';

const AVATAR =
  'bg-surface-sunken text-text-muted inline-flex shrink-0 items-center justify-center overflow-hidden rounded-full bg-cover bg-center leading-none';

const SLOT = 'hidden h-8 w-28 items-center justify-end in-data-viewer:inline-flex';
const NAV_ITEM =
  'text-text-muted hover:bg-surface-hover hover:text-text -my-1 -ml-2 inline-flex items-center gap-2 rounded-md px-2 py-1 whitespace-nowrap transition-colors duration-200 focus-ring';
const COMPACT_SLOT =
  'hidden size-5 items-center justify-center xxs:size-6 in-data-viewer:inline-flex';

type Props = {
  readonly compact?: boolean;
};

function readSnapshot(): string {
  if (!hasViewerHint()) return 'anonymous';
  try {
    return localStorage.getItem(VIEWER_STORAGE_KEY) ?? 'loading';
  } catch {
    return 'loading';
  }
}

const subscribeToNothing = () => () => {};

export function AccountControl({ compact = false }: Props) {
  const snapshot = React.useSyncExternalStore(subscribeToNothing, readSnapshot, () => null);
  const [fetched, setFetched] = React.useState<ViewerState | undefined>(undefined);

  React.useEffect(() => {
    let active = true;
    const load = async () => {
      const result = await loadViewer();
      if (!active) return;
      if (result.kind === 'failed') {
        if (!hasViewerHint()) setFetched({ kind: 'anonymous' });
        return;
      }
      setFetched(result);
    };
    void load();
    return () => {
      active = false;
    };
  }, []);

  const state = fetched ?? stateFromSnapshot(browserStorage, snapshot);
  if (state.kind === 'anonymous') return null;

  return <span className={compact ? COMPACT_SLOT : SLOT}>{renderState(state, compact)}</span>;
}

function renderKnownViewer(compact: boolean): React.ReactNode {
  const shown = 'hidden in-data-[viewer=signed-in]:inline-flex';
  const avatar = cn(AVATAR, 'bg-(image:--viewer-avatar) before:content-(--viewer-initial)');
  if (compact) {
    return (
      <a
        href="/account"
        rel="nofollow"
        aria-label="حسابي"
        className={cn(shown, avatar, 'size-full text-xs focus-ring')}
      />
    );
  }
  return (
    <a href="/account" rel="nofollow" className={cn(NAV_ITEM, shown)}>
      <span aria-hidden="true" className={cn(avatar, 'size-5 text-[10px]')} />
      حسابي
    </a>
  );
}

function renderSkeleton(compact: boolean, onlyWhileLoading: boolean): React.ReactNode {
  const shown = onlyWhileLoading ? 'hidden in-data-[viewer=loading]:inline-flex' : 'inline-flex';
  if (compact) {
    return (
      <span
        aria-hidden="true"
        className={cn(shown, 'size-full animate-pulse rounded-full bg-surface-sunken')}
      />
    );
  }
  return (
    <span aria-hidden="true" className={cn(shown, '-ml-2 items-center gap-2 px-2 py-1')}>
      <span className="size-5 animate-pulse rounded-full bg-surface-sunken" />
      <span className="h-4 w-10 animate-pulse rounded bg-surface-sunken" />
    </span>
  );
}

function renderState(
  state: Exclude<ViewerState, { readonly kind: 'anonymous' }>,
  compact: boolean
): React.ReactNode {
  if (state.kind === 'pending') {
    return (
      <>
        {renderKnownViewer(compact)}
        {renderSkeleton(compact, true)}
      </>
    );
  }
  if (state.kind === 'loading') return renderSkeleton(compact, false);

  const { avatarData } = state.viewer;
  const avatarStyle = avatarData === null ? undefined : { backgroundImage: `url("${avatarData}")` };
  const initial = avatarData === null ? initialOf(state.viewer) : null;

  if (compact) {
    return (
      <a
        href="/account"
        rel="nofollow"
        aria-label="حسابي"
        className={cn(AVATAR, 'size-full text-xs focus-ring')}
        style={avatarStyle}
      >
        {initial}
      </a>
    );
  }

  return (
    <a href="/account" rel="nofollow" className={NAV_ITEM}>
      <span aria-hidden="true" className={cn(AVATAR, 'size-5 text-[10px]')} style={avatarStyle}>
        {initial}
      </span>
      حسابي
    </a>
  );
}
