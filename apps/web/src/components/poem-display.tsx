'use client';

import { Fragment, type CSSProperties, type ReactNode, useEffect, useMemo, useState } from 'react';

import { PoemToolbar } from '@/components/poem-toolbar';
import { formatArabicCount, stripTashkeel } from '@/lib/arabic';
import {
  CLASSICAL_LAYOUT_POEM_TYPES,
  FREE_VERSE_POEM_TYPE,
  VERSES_NOUN_FORMS,
} from '@/lib/constants/taxonomy-data';
import { buildHighlightRegex, highlightSegments, parseHighlightTerms } from '@/lib/highlight';
import { HALVES_SPACING_MIN } from '@/lib/settings/settings-schema';
import { useSettings } from '@/lib/settings/use-settings';
import { poetsUrl, poetUrl } from '@/lib/urls';
import { cn } from '@/lib/utils';

import type { Poem } from '@/lib/api/result-types';

const VERSE_GAP = 'gap-[calc(2.5rem*var(--poem-spacing))] sm:gap-[calc(3rem*var(--poem-spacing))]';
const HEMISTICH_GAP =
  'gap-[calc(1rem*var(--poem-halves-spacing))] sm:gap-[calc(1.25rem*var(--poem-halves-spacing))]';

export function halvesSpacing(spacingScale: number): number {
  return Math.max(spacingScale, HALVES_SPACING_MIN);
}

function useHighlightTerms(): readonly string[] {
  const [terms, setTerms] = useState<readonly string[]>([]);

  useEffect(() => {
    const parsed = parseHighlightTerms(window.location.hash);
    // oxlint-disable-next-line react/set-state-in-effect
    if (parsed.length > 0) setTerms(parsed);
  }, []);

  useEffect(() => {
    if (terms.length === 0) return;
    requestAnimationFrame(() => {
      const el = document.querySelector('[data-highlight]');
      el?.scrollIntoView({ behavior: 'smooth', block: 'center' });
    });
  }, [terms]);

  return terms;
}

function highlightVerse(text: string, regex: RegExp): ReactNode {
  const segments = highlightSegments(text, regex);
  if (segments.length === 1 && segments[0]?.highlighted !== true) return text;
  return (
    <>
      {segments.map((segment, index) => {
        const key = `${segment.text}-${index}`;
        return segment.highlighted ? (
          <span
            key={key}
            data-highlight=""
            className="rounded-sm bg-highlight box-decoration-clone px-1 py-0.5 text-surface"
          >
            {segment.text}
          </span>
        ) : (
          <Fragment key={key}>{segment.text}</Fragment>
        );
      })}
    </>
  );
}

type PoemDisplayProps = {
  readonly title: string;
  readonly poet: Poem['poet'];
  readonly era: Poem['era'];
  readonly meter: Poem['meter'];
  readonly theme: Poem['theme'];
  readonly verses: Poem['verses'];
  readonly verseCount: Poem['verseCount'];
  readonly poemType: Poem['poemType'];
};

export function PoemDisplay({
  title,
  poet,
  era,
  meter,
  theme,
  verses,
  verseCount,
  poemType,
}: PoemDisplayProps) {
  const isClassical = CLASSICAL_LAYOUT_POEM_TYPES.has(poemType.slug);
  const lineByLine =
    poemType.slug === FREE_VERSE_POEM_TYPE || verses.every((entry) => entry.length === 1);
  const { poemFontScale, poemSpacingScale } = useSettings();
  const [showTashkeel, setShowTashkeel] = useState(true);
  const highlightTerms = useHighlightTerms();
  const highlightRegex = useMemo(() => buildHighlightRegex(highlightTerms), [highlightTerms]);
  const columnStyle: CSSProperties &
    Record<'--poem-scale' | '--poem-spacing' | '--poem-halves-spacing', number> = {
    '--poem-scale': poemFontScale,
    '--poem-spacing': poemSpacingScale,
    '--poem-halves-spacing': halvesSpacing(poemSpacingScale),
  };
  return (
    <>
      <header className="flex w-full flex-col items-center justify-center gap-4 text-center xxs:gap-6">
        <div className="flex flex-col gap-2 xxs:gap-4">
          <h1 className="text-poem-title text-text">{title}</h1>

          <h2 className="text-poem-byline text-text-muted">
            <a href={poetUrl(poet.slug)} className="rounded-sm focus-ring hover:underline">
              {poet.name}
            </a>{' '}
            <a href={poetsUrl({ era: era.slug })} className="rounded-sm focus-ring hover:underline">
              {`(${era.name})`}
            </a>
          </h2>
        </div>

        <div className="flex w-full items-center justify-between px-2.5 text-poem-meta text-text-subtle md:w-8/12 md:px-8 lg:px-16">
          <p className="flex-1 border-l py-0.5 md:py-1 lg:py-1.5">{meter.name}</p>
          <p className="flex-1 border-l py-0.5 md:py-1 lg:py-1.5">
            {formatArabicCount({ count: verseCount, nounForms: VERSES_NOUN_FORMS })}
          </p>
          <p className="flex-1 py-0.5 md:py-1 lg:py-1.5">{theme.name}</p>
        </div>

        <PoemToolbar
          showTashkeel={showTashkeel}
          onToggleTashkeel={() => setShowTashkeel((shown) => !shown)}
        />
      </header>

      <div className="relative flex w-full flex-col items-center justify-between">
        <article className="flex w-full flex-col items-center p-6 md:p-8">
          <div
            className={cn(
              'flex w-full flex-col text-verse font-normal',
              lineByLine ? HEMISTICH_GAP : VERSE_GAP,
              isClassical ? 'max-w-[calc(16em*var(--poem-scale))]' : 'px-(--poem-gutter)'
            )}
            style={columnStyle}
          >
            {verses.map((entry, index) => {
              const halves = isClassical && entry.length === 2;
              return (
                <div
                  // oxlint-disable-next-line react/no-array-index-key -- rows keep their stored order and a refrain repeats the same text
                  key={`${index}|${entry.join('|')}`}
                  className={cn(
                    'flex w-full flex-col items-center justify-center text-center',
                    HEMISTICH_GAP,
                    isClassical && 'items-stretch'
                  )}
                >
                  {entry.map((part, partIndex) => {
                    const text = showTashkeel ? part : stripTashkeel(part);
                    return (
                      <p
                        // oxlint-disable-next-line react/no-array-index-key -- parts keep their order within the stored row and can repeat
                        key={`${partIndex}|${part}`}
                        style={{ fontSize: `${poemFontScale}em` }}
                        lang="ar"
                        dir="rtl"
                        className={cn(
                          halves && (partIndex === 0 ? 'pe-12 text-right' : 'ps-12 text-left')
                        )}
                      >
                        {highlightRegex ? highlightVerse(text, highlightRegex) : text}
                      </p>
                    );
                  })}
                </div>
              );
            })}
          </div>
        </article>
      </div>
    </>
  );
}
