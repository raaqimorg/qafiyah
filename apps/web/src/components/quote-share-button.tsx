'use client';

import { Share2 } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { SHARE_TEXTS } from '@/lib/constants/copy';

type QuoteShareButtonProps = {
  readonly top: number;
  readonly onShare: () => void;
};

export function QuoteShareButton({ top, onShare }: QuoteShareButtonProps) {
  return (
    <Button
      type="button"
      size="sm"
      aria-haspopup="dialog"
      onMouseDown={(event) => event.preventDefault()}
      onClick={onShare}
      style={{ top }}
      className="absolute inset-x-0 z-10 mx-auto mt-2 w-fit shadow-lg select-none motion-safe:transition-opacity motion-safe:duration-200 motion-safe:starting:opacity-0"
    >
      <Share2 aria-hidden="true" />
      {SHARE_TEXTS.quote}
    </Button>
  );
}
