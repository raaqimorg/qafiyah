'use client';

import { Check, Copy, Link2, Minus, Plus, Share2, X } from 'lucide-react';
import { useEffect, useRef, useState, type ReactNode, type RefObject } from 'react';

import { PoemSelectionShare } from '@/components/poem-selection-share';
import { IconButton } from '@/components/ui/icon-button';
import { captureEvent } from '@/lib/analytics/capture-event';
import { toArabicDigits } from '@/lib/arabic';
import {
  POEM_SELECTION_SHARE_TEXTS,
  SHARE_BAR_TEXTS,
  SHARE_BUTTON_TEXTS,
} from '@/lib/constants/copy';
import { shareIntent, shareUrl, type ShareNetwork } from '@/lib/urls';
import { cn } from '@/lib/utils';
import { PROD_SITE_URL } from '@qafiyah/config';

import type { PoemCopy } from '@/components/poem-copy';

type ShareBarProps = {
  readonly path: string;
  readonly className?: string;
};

type ShareLink = {
  readonly network: ShareNetwork;
  readonly label: string;
  readonly iconPath: string;
};

const SHARE_LINKS: readonly ShareLink[] = [
  {
    network: 'x',
    label: SHARE_BAR_TEXTS.x,
    iconPath:
      'M14.234 10.162 22.977 0h-2.072l-7.591 8.824L7.251 0H.258l9.168 13.343L.258 24H2.33l8.016-9.318L16.749 24h6.993zm-2.837 3.299-.929-1.329L3.076 1.56h3.182l5.965 8.532.929 1.329 7.754 11.09h-3.182z',
  },
  {
    network: 'facebook',
    label: SHARE_BAR_TEXTS.facebook,
    iconPath:
      'M9.101 23.691v-7.98H6.627v-3.667h2.474v-1.58c0-4.085 1.848-5.978 5.858-5.978.401 0 .955.042 1.468.103a8.68 8.68 0 0 1 1.141.195v3.325a8.623 8.623 0 0 0-.653-.036 26.805 26.805 0 0 0-.733-.009c-.707 0-1.259.096-1.675.309a1.686 1.686 0 0 0-.679.622c-.258.42-.374.995-.374 1.752v1.297h3.919l-.386 2.103-.287 1.564h-3.246v8.245C19.396 23.238 24 18.179 24 12.044c0-6.627-5.373-12-12-12s-12 5.373-12 12c0 5.628 3.874 10.35 9.101 11.647Z',
  },
  {
    network: 'whatsapp',
    label: SHARE_BAR_TEXTS.whatsapp,
    iconPath:
      'M17.472 14.382c-.297-.149-1.758-.867-2.03-.967-.273-.099-.471-.148-.67.15-.197.297-.767.966-.94 1.164-.173.199-.347.223-.644.075-.297-.15-1.255-.463-2.39-1.475-.883-.788-1.48-1.761-1.653-2.059-.173-.297-.018-.458.13-.606.134-.133.298-.347.446-.52.149-.174.198-.298.298-.497.099-.198.05-.371-.025-.52-.075-.149-.669-1.612-.916-2.207-.242-.579-.487-.5-.669-.51-.173-.008-.371-.01-.57-.01-.198 0-.52.074-.792.372-.272.297-1.04 1.016-1.04 2.479 0 1.462 1.065 2.875 1.213 3.074.149.198 2.096 3.2 5.077 4.487.709.306 1.262.489 1.694.625.712.227 1.36.195 1.871.118.571-.085 1.758-.719 2.006-1.413.248-.694.248-1.289.173-1.413-.074-.124-.272-.198-.57-.347m-5.421 7.403h-.004a9.87 9.87 0 01-5.031-1.378l-.361-.214-3.741.982.998-3.648-.235-.374a9.86 9.86 0 01-1.51-5.26c.001-5.45 4.436-9.884 9.888-9.884 2.64 0 5.122 1.03 6.988 2.898a9.825 9.825 0 012.893 6.994c-.003 5.45-4.437 9.884-9.885 9.884m8.413-18.297A11.815 11.815 0 0012.05 0C5.495 0 .16 5.335.157 11.892c0 2.096.547 4.142 1.588 5.945L.057 24l6.305-1.654a11.882 11.882 0 005.683 1.448h.005c6.554 0 11.89-5.335 11.893-11.893a11.821 11.821 0 00-3.48-8.413Z',
  },
  {
    network: 'telegram',
    label: SHARE_BAR_TEXTS.telegram,
    iconPath:
      'M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z',
  },
  {
    network: 'reddit',
    label: SHARE_BAR_TEXTS.reddit,
    iconPath:
      'M12 0C5.373 0 0 5.373 0 12c0 3.314 1.343 6.314 3.515 8.485l-2.286 2.286C.775 23.225 1.097 24 1.738 24H12c6.627 0 12-5.373 12-12S18.627 0 12 0Zm4.388 3.199c1.104 0 1.999.895 1.999 1.999 0 1.105-.895 2-1.999 2-.946 0-1.739-.657-1.947-1.539v.002c-1.147.162-2.032 1.15-2.032 2.341v.007c1.776.067 3.4.567 4.686 1.363.473-.363 1.064-.58 1.707-.58 1.547 0 2.802 1.254 2.802 2.802 0 1.117-.655 2.081-1.601 2.531-.088 3.256-3.637 5.876-7.997 5.876-4.361 0-7.905-2.617-7.998-5.87-.954-.447-1.614-1.415-1.614-2.538 0-1.548 1.255-2.802 2.803-2.802.645 0 1.239.218 1.712.585 1.275-.79 2.881-1.291 4.64-1.365v-.01c0-1.663 1.263-3.034 2.88-3.207.188-.911.993-1.595 1.959-1.595Zm-8.085 8.376c-.784 0-1.459.78-1.506 1.797-.047 1.016.64 1.429 1.426 1.429.786 0 1.371-.369 1.418-1.385.047-1.017-.553-1.841-1.338-1.841Zm7.406 0c-.786 0-1.385.824-1.338 1.841.047 1.017.634 1.385 1.418 1.385.785 0 1.473-.413 1.426-1.429-.046-1.017-.721-1.797-1.506-1.797Zm-3.703 4.013c-.974 0-1.907.048-2.77.135-.147.015-.241.168-.183.305.483 1.154 1.622 1.964 2.953 1.964 1.33 0 2.47-.81 2.953-1.964.057-.137-.037-.29-.184-.305-.863-.087-1.795-.135-2.769-.135Z',
  },
  {
    network: 'linkedin',
    label: SHARE_BAR_TEXTS.linkedin,
    iconPath:
      'M20.447 20.452h-3.554v-5.569c0-1.328-.027-3.037-1.852-3.037-1.853 0-2.136 1.445-2.136 2.939v5.667H9.351V9h3.414v1.561h.046c.477-.9 1.637-1.85 3.37-1.85 3.601 0 4.267 2.37 4.267 5.455v6.286zM5.337 7.433c-1.144 0-2.063-.926-2.063-2.065 0-1.138.92-2.063 2.063-2.063 1.14 0 2.064.925 2.064 2.063 0 1.139-.925 2.065-2.064 2.065zm1.782 13.019H3.555V9h3.564v11.452zM22.225 0H1.771C.792 0 0 .774 0 1.729v20.542C0 23.227.792 24 1.771 24h20.451C23.2 24 24 23.227 24 22.271V1.729C24 .774 23.2 0 22.222 0h.003z',
  },
];

const ICON = 'size-5';
const ICON_STROKE = 1.5;
const DIMMED = 'opacity-75 transition hover:opacity-100';
const STEP_BUTTON = cn(DIMMED, 'aria-disabled:pointer-events-none aria-disabled:opacity-50');
const LINK_CLASS =
  'inline-flex min-h-11 min-w-11 items-center justify-center rounded-md text-text-subtle transition-colors hover:text-text focus-visible:ring-1 focus-visible:ring-text';

function ShareIcon({ iconPath }: { readonly iconPath: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="currentColor"
      aria-hidden="true"
      focusable="false"
      className={ICON}
    >
      <path d={iconPath} />
    </svg>
  );
}

function CopyButton({
  text,
  label,
  copiedLabel,
  icon,
}: {
  readonly text: string;
  readonly label: string;
  readonly copiedLabel: string;
  readonly icon: ReactNode;
}) {
  const [status, setStatus] = useState<'idle' | 'copied' | 'failed'>('idle');
  useEffect(() => {
    if (status !== 'copied') return;
    const timeout = window.setTimeout(() => setStatus('idle'), 2000);
    return () => window.clearTimeout(timeout);
  }, [status]);
  const statusText = { idle: '', copied: copiedLabel, failed: SHARE_BAR_TEXTS.copyFailed }[status];
  const handleCopy = async () => {
    setStatus('idle');
    try {
      await navigator.clipboard.writeText(text);
      setStatus('copied');
    } catch {
      setStatus('failed');
    }
  };
  return (
    <div className="flex flex-col items-center">
      <IconButton
        onClick={() => {
          void handleCopy();
        }}
        aria-label={status === 'copied' ? copiedLabel : label}
      >
        {status === 'copied' ? <Check className={ICON} strokeWidth={ICON_STROKE} /> : icon}
      </IconButton>
      <span role="status" className={status === 'failed' ? 'text-xs text-text' : 'sr-only'}>
        {statusText}
      </span>
    </div>
  );
}

export function ShareBar({ path, className }: ShareBarProps) {
  return (
    <div
      role="group"
      aria-label={SHARE_BAR_TEXTS.label}
      className={cn('flex flex-wrap items-center justify-center select-none', className)}
    >
      {SHARE_LINKS.map((link) => (
        <a
          key={link.network}
          href={shareUrl(link.network, path)}
          target="_blank"
          rel="noopener noreferrer"
          aria-label={link.label}
          title={link.label}
          className={LINK_CLASS}
        >
          <ShareIcon iconPath={link.iconPath} />
        </a>
      ))}
      <CopyButton
        text={`${PROD_SITE_URL}${path}`}
        label={SHARE_BAR_TEXTS.copyLink}
        copiedLabel={SHARE_BAR_TEXTS.copied}
        icon={<Link2 className={ICON} strokeWidth={ICON_STROKE} />}
      />
    </div>
  );
}

type ShareButtonProps = {
  readonly title: string;
  readonly poetName: string;
  readonly eraName: string;
  readonly verses: readonly (readonly string[])[];
  readonly poemRef: RefObject<HTMLElement | null>;
  readonly canonicalUrl: string;
  readonly path: string;
};

export function ShareButton({
  title,
  poetName,
  eraName,
  verses,
  path,
  poemRef,
  canonicalUrl,
}: ShareButtonProps) {
  const [verseCount, setVerseCount] = useState(0);
  const [selectedPoem, setSelectedPoem] = useState<PoemCopy | null>(null);
  const [hasShareFailed, setHasShareFailed] = useState(false);
  const isSharingRef = useRef(false);
  const dialogRef = useRef<HTMLDialogElement>(null);

  const header = `${title}\n${poetName} (${eraName})`;
  const verseBlocks = verses.slice(0, verseCount).map((entry) => entry.join('\n'));
  const body = verseBlocks.length > 0 ? `${header}\n\n${verseBlocks.join('\n\n')}` : header;
  const url = selectedPoem?.url ?? `${PROD_SITE_URL}${path}`;
  const fullText = selectedPoem?.text ?? `${body}\n\n${url}`;
  const handleSelectionShare = async (copy: PoemCopy) => {
    if (isSharingRef.current) return;
    const { text } = copy;
    setSelectedPoem(copy);
    setHasShareFailed(false);
    if (typeof navigator.share === 'function') {
      isSharingRef.current = true;
      try {
        if (typeof navigator.canShare !== 'function' || navigator.canShare({ text })) {
          await navigator.share({ text });
          return;
        }
      } catch (error) {
        if (error instanceof DOMException && error.name === 'AbortError') return;
        setHasShareFailed(true);
      } finally {
        isSharingRef.current = false;
      }
    }
    dialogRef.current?.showModal();
  };

  return (
    <>
      <IconButton
        onClick={() => {
          setSelectedPoem(null);
          setHasShareFailed(false);
          dialogRef.current?.showModal();
        }}
        aria-haspopup="dialog"
        aria-label={SHARE_BUTTON_TEXTS.open}
        className={DIMMED}
      >
        <Share2 className={ICON} strokeWidth={ICON_STROKE} />
      </IconButton>
      <PoemSelectionShare
        poemRef={poemRef}
        poetName={poetName}
        canonicalUrl={canonicalUrl}
        dialogRef={dialogRef}
        onSelectionClear={() => {
          if (selectedPoem !== null) dialogRef.current?.close();
        }}
        onShare={(copy) => {
          void handleSelectionShare(copy);
        }}
      />
      <dialog
        ref={dialogRef}
        onPointerDown={(event) => {
          if (
            selectedPoem !== null &&
            event.target instanceof Element &&
            event.target.closest('button, a') !== null
          ) {
            event.preventDefault();
          }
        }}
        aria-label={
          selectedPoem === null ? SHARE_BUTTON_TEXTS.label : POEM_SELECTION_SHARE_TEXTS.title
        }
        className="m-auto max-h-[calc(100dvh-2rem)] w-[min(26rem,calc(100vw-2rem))] overflow-y-auto rounded-md border border-border bg-surface-raised p-4 text-text backdrop:bg-black/20"
      >
        <form method="dialog" className="mb-2 flex justify-end">
          <IconButton type="submit" aria-label={SHARE_BUTTON_TEXTS.close}>
            <X className={ICON} strokeWidth={ICON_STROKE} />
          </IconButton>
        </form>
        <div
          dir="rtl"
          className="max-h-72 overflow-auto rounded-sm bg-surface-sunken p-3 text-base leading-8 whitespace-pre-wrap text-text select-text"
        >
          {fullText}
        </div>
        {hasShareFailed && (
          <p role="alert" className="mt-3 text-sm">
            {POEM_SELECTION_SHARE_TEXTS.shareFailed}
          </p>
        )}
        {selectedPoem === null && (
          <div className="mt-3 flex items-center justify-between">
            <span className="text-xs text-text-muted">
              {SHARE_BUTTON_TEXTS.verses} {toArabicDigits(verseCount)}
            </span>
            <div className="flex items-center gap-1">
              <IconButton
                aria-disabled={verseCount <= 0}
                aria-label={SHARE_BUTTON_TEXTS.removeVerse}
                onClick={() => setVerseCount((current) => Math.max(0, current - 1))}
                className={STEP_BUTTON}
              >
                <Minus className="size-4" strokeWidth={ICON_STROKE} />
              </IconButton>
              <IconButton
                aria-disabled={verseCount >= verses.length}
                aria-label={SHARE_BUTTON_TEXTS.addVerse}
                onClick={() => setVerseCount((current) => Math.min(verses.length, current + 1))}
                className={STEP_BUTTON}
              >
                <Plus className="size-4" strokeWidth={ICON_STROKE} />
              </IconButton>
            </div>
          </div>
        )}
        <div className="mt-3 flex flex-wrap items-center justify-center gap-1">
          {SHARE_LINKS.map((link) => (
            <a
              key={link.network}
              href={shareIntent(link.network, {
                body: selectedPoem?.text ?? body,
                title,
                url,
                text: fullText,
              })}
              target="_blank"
              rel="noopener noreferrer"
              aria-label={link.label}
              title={link.label}
              className={LINK_CLASS}
              onClick={() => captureEvent('shared', { network: link.network })}
            >
              <ShareIcon iconPath={link.iconPath} />
            </a>
          ))}
        </div>
        <div className="mt-3 flex items-center justify-center gap-2">
          <CopyButton
            key={fullText}
            text={fullText}
            label={SHARE_BUTTON_TEXTS.copyText}
            copiedLabel={SHARE_BUTTON_TEXTS.copiedText}
            icon={<Copy className={ICON} strokeWidth={ICON_STROKE} />}
          />
          <CopyButton
            text={url}
            label={SHARE_BUTTON_TEXTS.copyLink}
            copiedLabel={SHARE_BUTTON_TEXTS.copiedLink}
            icon={<Link2 className={ICON} strokeWidth={ICON_STROKE} />}
          />
        </div>
      </dialog>
    </>
  );
}
