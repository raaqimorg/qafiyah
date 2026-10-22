'use client';

import { X } from 'lucide-react';
import { type Ref, useId } from 'react';

import { IconButton } from '@/components/ui/icon-button';
import { captureEvent } from '@/lib/analytics/capture-event';
import { SITE_URL } from '@/lib/constants/config';
import { SHARE_NETWORK_NAMES, SHARE_TEXTS } from '@/lib/constants/copy';
import { shareIntent, type ShareNetwork } from '@/lib/urls';

type ShareLink = {
  readonly network: ShareNetwork;
  readonly label: string;
  readonly name: string;
  readonly iconPath: string;
};

const SHARE_LINKS: readonly ShareLink[] = [
  {
    network: 'telegram',
    label: SHARE_TEXTS.telegram,
    name: SHARE_NETWORK_NAMES.telegram,
    iconPath: 'M15 10l-4 4l6 6l4 -16l-18 7l4 2l2 6l3 -4',
  },
  {
    network: 'whatsapp',
    label: SHARE_TEXTS.whatsapp,
    name: SHARE_NETWORK_NAMES.whatsapp,
    iconPath:
      'M3 21l1.65 -3.8a9 9 0 1 1 3.4 2.9l-5.05 .9 M9 10a.5 .5 0 0 0 1 0v-1a.5 .5 0 0 0 -1 0v1a5 5 0 0 0 5 5h1a.5 .5 0 0 0 0 -1h-1a.5 .5 0 0 0 0 1',
  },
  {
    network: 'facebook',
    label: SHARE_TEXTS.facebook,
    name: SHARE_NETWORK_NAMES.facebook,
    iconPath: 'M7 10v4h3v7h4v-7h3l1 -4h-4v-2a1 1 0 0 1 1 -1h3v-4h-3a5 5 0 0 0 -5 5v2h-3',
  },
  {
    network: 'x',
    label: SHARE_TEXTS.x,
    name: SHARE_NETWORK_NAMES.x,
    iconPath:
      'M4 4l11.733 16h4.267l-11.733 -16l-4.267 0 M4 20l6.768 -6.768m2.46 -2.46l6.772 -6.772',
  },
];

export const SHARE_ICON = 'size-5';
export const SHARE_ICON_STROKE = 1.5;

function ShareIcon({ iconPath }: { readonly iconPath: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={SHARE_ICON_STROKE}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      className={SHARE_ICON}
    >
      <path d={iconPath} />
    </svg>
  );
}

type ShareSheetProps = {
  readonly ref: Ref<HTMLDialogElement>;
  readonly title: string;
  readonly body: string;
  readonly path: string;
};

export function ShareSheet({ ref, title, body, path }: ShareSheetProps) {
  const titleId = useId();
  const url = `${SITE_URL}${path}`;

  return (
    // oxlint-disable-next-line jsx-a11y/no-noninteractive-element-interactions -- a click on the backdrop closes the dialog, because Safari lacks closedby, and Escape already closes it from the keyboard
    <dialog
      ref={ref}
      onClick={(event) => {
        if (event.target === event.currentTarget) event.currentTarget.close();
      }}
      aria-labelledby={titleId}
      className="mx-0 mt-auto mb-0 w-full max-w-none rounded-t-3xl bg-surface-raised p-0 text-text shadow-xl backdrop:bg-black/50 motion-safe:transition-[translate,opacity] motion-safe:duration-200 motion-safe:ease-out sm:m-auto sm:w-80 sm:rounded-lg sm:border sm:border-border motion-safe:starting:open:translate-y-full sm:motion-safe:starting:open:translate-y-4 sm:motion-safe:starting:open:opacity-0"
    >
      <div className="pt-2 pb-[env(safe-area-inset-bottom)] sm:pt-0">
        <div
          aria-hidden="true"
          className="mx-auto h-1.5 w-9 rounded-full bg-text-subtle/30 sm:hidden"
        />
        <div className="flex items-center justify-between px-5 py-1 sm:py-2">
          <h2 id={titleId} className="text-text">
            {title}
          </h2>
          <form method="dialog">
            <IconButton type="submit" aria-label={SHARE_TEXTS.close} className="-me-3">
              <X className={SHARE_ICON} strokeWidth={SHARE_ICON_STROKE} />
            </IconButton>
          </form>
        </div>
        <ul className="flex flex-col px-3 py-2">
          {SHARE_LINKS.map((link) => (
            <li key={link.network}>
              <a
                href={shareIntent(link.network, { body, url })}
                target="_blank"
                rel="noopener noreferrer"
                aria-label={link.label}
                className="flex min-h-12 items-center gap-3 rounded-md px-2 text-text-muted focus-ring transition-colors hover:bg-surface-sunken hover:text-text"
                onClick={() => captureEvent('shared', { network: link.network })}
              >
                <ShareIcon iconPath={link.iconPath} />
                <span>{link.name}</span>
              </a>
            </li>
          ))}
        </ul>
      </div>
    </dialog>
  );
}
