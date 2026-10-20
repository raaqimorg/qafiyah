import { Check, Copy } from 'lucide-react';

import { SITE_URL } from '@/lib/constants/config';
import { FEELING_TEXTS } from '@/lib/constants/copy';
import { TYPE } from '@/lib/constants/design-tokens';
import { jumpTerm } from '@/lib/feelings/jump-term';
import { poemUrl } from '@/lib/urls';
import { cn } from '@/lib/utils';

import type { FeelingVerseRow } from '@/lib/server/feelings';

export function verseCopyText(verse: Pick<FeelingVerseRow, 'parts' | 'poet' | 'poem'>): string {
  return [...verse.parts, '', verse.poet.name, `${SITE_URL}${poemUrl(verse.poem.slug)}`].join('\n');
}

export function FeelingVerseItem({ verse }: { readonly verse: FeelingVerseRow }) {
  const halves = verse.parts.length === 2;
  return (
    <li className="group/card relative -mx-2 grid grid-cols-[auto_minmax(0,1fr)] grid-rows-[1fr_auto] gap-4 border-b border-border px-2 py-5 transition-colors duration-300 hover:bg-surface-hover">
      <p className={cn(TYPE.micro, 'self-start justify-self-center pt-1 text-highlight')}>
        {verse.feeling.name}
      </p>
      <a
        href={poemUrl(verse.poem.slug, [jumpTerm(verse.parts)])}
        className={cn(
          TYPE.caption,
          'flex w-fit max-w-full min-w-[min(16em,100%)] flex-col gap-0.5 text-text outline-none after:absolute after:inset-0 focus-visible:after:ring-1 focus-visible:after:ring-text sm:text-body'
        )}
      >
        {verse.parts.map((part, partIndex) => (
          <span
            // oxlint-disable-next-line react/no-array-index-key -- parts keep their order within the stored row and can repeat
            key={`${partIndex}|${part}`}
            lang="ar"
            className={cn(
              halves && (partIndex === 0 ? 'text-right lg:pe-12' : 'text-left lg:ps-12')
            )}
          >
            {part}
          </span>
        ))}
      </a>
      <button
        type="button"
        data-copy-verse={verseCopyText(verse)}
        data-copied-label={FEELING_TEXTS.verseCopied}
        aria-label={FEELING_TEXTS.copyVerse}
        title={FEELING_TEXTS.copyVerse}
        className="group relative z-10 flex size-8.5 shrink-0 items-center justify-center justify-self-center rounded-md text-text-subtle focus-ring transition-colors group-hover/card:text-text-muted hover:text-text"
      >
        <Copy
          aria-hidden="true"
          size={16}
          className="h-4 w-4 transition-opacity duration-300 group-data-copied:opacity-0 group-data-copied:duration-0"
        />
        <Check
          aria-hidden="true"
          size={16}
          className="absolute h-4 w-4 text-highlight opacity-0 group-data-copied:animate-copy-check motion-reduce:group-data-copied:animate-none motion-reduce:group-data-copied:opacity-100"
        />
      </button>
      <p
        className={cn(
          TYPE.micro,
          'max-w-full min-w-0 self-center justify-self-end truncate text-text-muted'
        )}
      >
        {verse.poet.name}
      </p>
    </li>
  );
}
