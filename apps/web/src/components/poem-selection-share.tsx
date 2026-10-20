import { Share2 } from 'lucide-react';
import { useEffect, useState, type CSSProperties, type RefObject } from 'react';

import { getPoemCopy, type PoemCopy } from '@/components/poem-copy';
import { Button } from '@/components/ui/button';
import { POEM_SELECTION_SHARE_TEXTS } from '@/lib/constants/copy';

type SelectedPoem = {
  readonly copy: PoemCopy;
  readonly top: number;
  readonly left: number;
};

type PoemSelectionShareProps = {
  readonly poemRef: RefObject<HTMLElement | null>;
  readonly poetName: string;
  readonly canonicalUrl: string;
  readonly dialogRef: RefObject<HTMLDialogElement | null>;
  readonly onShare: (copy: PoemCopy) => void;
  readonly onSelectionClear: () => void;
};

export function PoemSelectionShare({
  poemRef,
  poetName,
  canonicalUrl,
  dialogRef,
  onShare,
  onSelectionClear,
}: PoemSelectionShareProps) {
  const [selection, setSelection] = useState<SelectedPoem | null>(null);

  useEffect(() => {
    const updateSelection = () => {
      const container = poemRef.current;
      const currentSelection = document.getSelection();
      if (container === null || currentSelection === null) {
        setSelection(null);
        return;
      }
      const copy = getPoemCopy(currentSelection, container, poetName, canonicalUrl);
      if (copy === null) {
        setSelection(null);
        onSelectionClear();
        return;
      }
      if (dialogRef.current?.open === true) return;
      const range = currentSelection.getRangeAt(0);
      const bounds = range.getBoundingClientRect();
      let lastLine = bounds;
      for (const rect of range.getClientRects()) {
        if (rect.width > 0 && rect.height > 0) lastLine = rect;
      }
      const viewport = window.visualViewport;
      const viewportTop = viewport?.offsetTop ?? 0;
      const viewportLeft = viewport?.offsetLeft ?? 0;
      const viewportBottom = viewportTop + (viewport?.height ?? window.innerHeight);
      const viewportRight = viewportLeft + (viewport?.width ?? window.innerWidth);
      if (
        bounds.bottom <= viewportTop ||
        bounds.top >= viewportBottom ||
        bounds.right <= viewportLeft ||
        bounds.left >= viewportRight
      ) {
        setSelection(null);
        return;
      }
      const top = lastLine.bottom + 16;
      const left = Math.max(viewportLeft + 8, (viewportLeft + viewportRight) / 2 - 80);
      setSelection((current) =>
        current?.copy.text === copy.text && current.top === top && current.left === left
          ? current
          : { copy, top, left }
      );
    };
    const dialog = dialogRef.current;
    const viewport = window.visualViewport;
    viewport?.addEventListener('resize', updateSelection);
    viewport?.addEventListener('scroll', updateSelection);
    dialog?.addEventListener('close', updateSelection);
    document.addEventListener('selectionchange', updateSelection);
    document.addEventListener('scroll', updateSelection, true);
    window.addEventListener('resize', updateSelection);
    return () => {
      dialog?.removeEventListener('close', updateSelection);
      viewport?.removeEventListener('resize', updateSelection);
      viewport?.removeEventListener('scroll', updateSelection);
      document.removeEventListener('selectionchange', updateSelection);
      document.removeEventListener('scroll', updateSelection, true);
      window.removeEventListener('resize', updateSelection);
    };
  }, [poemRef, poetName, canonicalUrl, dialogRef, onSelectionClear]);

  const positionStyle: CSSProperties & Record<'--selection-top' | '--selection-left', string> = {
    '--selection-top': `${selection?.top ?? 0}px`,
    '--selection-left': `${selection?.left ?? 0}px`,
  };
  if (selection === null) return null;
  return (
    <div className="fixed top-(--selection-top) left-(--selection-left) z-30" style={positionStyle}>
      <Button
        type="button"
        className="min-h-11 w-40 shadow-lg"
        onPointerDown={(event) => event.preventDefault()}
        onClick={() => {
          onShare(selection.copy);
        }}
      >
        <Share2 aria-hidden="true" />
        {POEM_SELECTION_SHARE_TEXTS.share}
      </Button>
    </div>
  );
}
