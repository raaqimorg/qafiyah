import { INLINE_LINK, TYPE } from '@/lib/constants/design-tokens';
import { poemUrl } from '@/lib/urls';
import { cn } from '@/lib/utils';

import type { FeelingVerse } from '@/lib/feelings/feeling-verses';

type Segment = { readonly id: string; readonly text: string; readonly marked: boolean };
type Hemistich = { readonly id: string; readonly segments: readonly Segment[] };

const MARK_TAG = /(<mark>|<\/mark>)/u;

function hemistichsOf(row: string): readonly Hemistich[] {
  const parts: Segment[][] = [[]];
  let marked = false;
  for (const piece of row.split(MARK_TAG)) {
    if (piece === '<mark>' || piece === '</mark>') {
      marked = piece === '<mark>';
      continue;
    }
    const pieces = piece.split('*');
    for (let index = 0; index < pieces.length; index += 1) {
      if (index > 0) parts.push([]);
      const text = pieces[index] ?? '';
      const current = parts.at(-1);
      if (current !== undefined && text.trim() !== '') {
        current.push({ id: `${parts.length}:${current.length}`, text, marked });
      }
    }
  }
  return parts
    .filter((segments) => segments.length > 0)
    .map((segments) => ({ id: segments.map((segment) => segment.id).join('|'), segments }));
}

export function FeelingVerseItem({ verse }: { readonly verse: FeelingVerse }) {
  return (
    <li className="flex flex-col gap-1.5 border-b border-border py-5 last:border-b-0">
      <p className={cn(TYPE.subheading, 'flex flex-wrap gap-x-11 leading-loose text-text')}>
        {hemistichsOf(verse.row).map((hemistich) => (
          <span key={hemistich.id}>
            {hemistich.segments.map((segment) =>
              segment.marked ? (
                <mark key={segment.id} className="rounded-sm bg-highlight/15 text-inherit">
                  {segment.text}
                </mark>
              ) : (
                <span key={segment.id}>{segment.text}</span>
              )
            )}
          </span>
        ))}
      </p>
      <p className={cn(TYPE.caption, 'flex flex-wrap gap-x-3.5 text-text-muted')}>
        <span>{verse.poet === '' ? 'شاعر غير معروف' : verse.poet}</span>
        {verse.era !== '' && <span>{verse.era}</span>}
        <a href={poemUrl(verse.poem)} className={INLINE_LINK}>
          {verse.title}
        </a>
      </p>
    </li>
  );
}
