'use client';

import { Share2 } from 'lucide-react';
import { useRef } from 'react';

import { SHARE_ICON, SHARE_ICON_STROKE, ShareSheet } from '@/components/share-sheet';
import { IconButton } from '@/components/ui/icon-button';
import { SHARE_TEXTS } from '@/lib/constants/copy';
import { cn } from '@/lib/utils';

type ShareButtonProps = {
  readonly title: string;
  readonly body: string;
  readonly path: string;
  readonly className?: string;
};

export function ShareButton({ title, body, path, className }: ShareButtonProps) {
  const dialogRef = useRef<HTMLDialogElement>(null);

  return (
    <>
      <IconButton
        onClick={() => dialogRef.current?.showModal()}
        aria-haspopup="dialog"
        aria-label={SHARE_TEXTS.open}
        className={cn('min-w-9 opacity-75 transition hover:opacity-100 sm:min-w-11', className)}
      >
        <Share2 className={SHARE_ICON} strokeWidth={SHARE_ICON_STROKE} />
      </IconButton>
      <ShareSheet ref={dialogRef} title={title} body={body} path={path} />
    </>
  );
}
