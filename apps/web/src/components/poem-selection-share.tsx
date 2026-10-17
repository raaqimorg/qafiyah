import { Share2 } from 'lucide-react';
import { useEffect, useState, type CSSProperties, type RefObject } from 'react';

import { getPoemCopyText } from '@/components/poem-copy';
import { Button } from '@/components/ui/button';
import { POEM_SELECTION_SHARE_TEXTS } from '@/lib/constants/copy';

type SelectedPoem = {
  readonly text: string;
  readonly top: number;
  readonly left: number;
};

type PoemSelectionShareProps = {
  readonly poemRef: RefObject<HTMLElement | null>;
  readonly signature: string;
  readonly dialogRef: RefObject<HTMLDialogElement | null>;
  readonly onShare: (text: string) => void;
  readonly onSelectionClear: () => void;
};

export function PoemSelectionShare({
  poemRef,
  signature,
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
      const text = getPoemCopyText(currentSelection, container, signature);
      if (text === null) {
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
        current?.text === text && current.top === top && current.left === left
          ? current
          : { text, top, left }
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
  }, [poemRef, signature, dialogRef, onSelectionClear]);

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
          onShare(selection.text);
        }}
      >
        <Share2 aria-hidden="true" />
        {POEM_SELECTION_SHARE_TEXTS.share}
      </Button>
    </div>
  );
}
