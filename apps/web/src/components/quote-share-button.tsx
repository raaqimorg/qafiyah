'use client';

import { Share2 } from 'lucide-react';
import { useEffect, useRef } from 'react';

import { Button } from '@/components/ui/button';
import { SHARE_TEXTS } from '@/lib/constants/copy';

type QuoteShareButtonProps = {
  readonly onShare: () => void;
  readonly onDismiss: () => void;
};

export function QuoteShareButton({ onShare, onDismiss }: QuoteShareButtonProps) {
  const buttonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const dismissOnPointer = (event: PointerEvent) => {
      if (event.target instanceof Node && buttonRef.current?.contains(event.target) === true) {
        return;
      }
      onDismiss();
    };
    const dismissOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onDismiss();
    };
    document.addEventListener('pointerdown', dismissOnPointer);
    document.addEventListener('keydown', dismissOnEscape);
    return () => {
      document.removeEventListener('pointerdown', dismissOnPointer);
      document.removeEventListener('keydown', dismissOnEscape);
    };
  }, [onDismiss]);

  return (
    <Button
      ref={buttonRef}
      type="button"
      size="sm"
      aria-haspopup="dialog"
      onClick={onShare}
      className="absolute inset-x-0 top-full z-10 mx-auto mt-2 w-fit shadow-md select-none"
    >
      <Share2 aria-hidden="true" />
      {SHARE_TEXTS.quote}
    </Button>
  );
}
