'use client';

import {
  Fragment,
  type CSSProperties,
  type ReactNode,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';

import { getPoemSelectionText, lastSelectedRow } from '@/components/poem-copy';
import { PoemToolbar } from '@/components/poem-toolbar';
import { QuoteShareButton } from '@/components/quote-share-button';
import { ShareButton } from '@/components/share-button';
import { ShareSheet } from '@/components/share-sheet';
import { formatArabicCount, stripTashkeel } from '@/lib/arabic';
import { SHARE_QUOTE_MAX_LENGTH, SITE_URL } from '@/lib/constants/config';
import { formatPoemQuote, formatPoemText, SHARE_TEXTS } from '@/lib/constants/copy';
import {
  CLASSICAL_LAYOUT_POEM_TYPES,
  FREE_VERSE_POEM_TYPE,
  VERSES_NOUN_FORMS,
} from '@/lib/constants/taxonomy-data';
import { buildHighlightRegex, highlightSegments, parseHighlightTerms } from '@/lib/highlight';
import { halvesSpacing, POEM_SCALE } from '@/lib/poem-scale';
import { poetsUrl, poetUrl } from '@/lib/urls';
import { cn } from '@/lib/utils';

import type { Poem } from '@/lib/api/result-types';
import type { ClipboardEvent } from 'react';

const VERSE_GAP = 'gap-[calc(2.5rem*var(--poem-spacing))] sm:gap-[calc(3rem*var(--poem-spacing))]';
const HEMISTICH_GAP =
  'gap-[calc(1rem*var(--poem-halves-spacing))] sm:gap-[calc(1.25rem*var(--poem-halves-spacing))]';

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
  readonly sharePath: string;
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
  sharePath,
}: PoemDisplayProps) {
  const columnRef = useRef<HTMLDivElement>(null);
  const quoteSheetRef = useRef<HTMLDialogElement>(null);
  const [quote, setQuote] = useState<string | null>(null);
  const [quoteRow, setQuoteRow] = useState<number | null>(null);
  const handleCopy = (event: ClipboardEvent<HTMLElement>) => {
    if (event.defaultPrevented) return;
    const container = event.currentTarget;
    const selection = container.ownerDocument.getSelection();
    const text = getPoemSelectionText(selection, container);
    if (selection === null || text === null) return;
    event.clipboardData.setData('text/plain', formatPoemQuote(text, `${SITE_URL}${sharePath}`));
    event.preventDefault();
    const rows = columnRef.current?.children;
    const row =
      rows === undefined || text.length > SHARE_QUOTE_MAX_LENGTH
        ? null
        : lastSelectedRow(selection.getRangeAt(0), rows);
    if (row !== null) setQuote(text);
    setQuoteRow(row);
  };
  const isClassical = CLASSICAL_LAYOUT_POEM_TYPES.has(poemType.slug);
  const lineByLine =
    poemType.slug === FREE_VERSE_POEM_TYPE || verses.every((entry) => entry.length === 1);
  const [fontScale, setFontScale] = useState<number>(POEM_SCALE.font.initial);
  const [spacingScale, setSpacingScale] = useState<number>(POEM_SCALE.spacing.initial);
  const [showTashkeel, setShowTashkeel] = useState(true);
  const shownVerses = useMemo(
    () => (showTashkeel ? verses : verses.map((entry) => entry.map((part) => stripTashkeel(part)))),
    [verses, showTashkeel]
  );
  const highlightTerms = useHighlightTerms();
  const highlightRegex = useMemo(
    () =>
      buildHighlightRegex(
        showTashkeel ? highlightTerms : highlightTerms.map((term) => stripTashkeel(term))
      ),
    [highlightTerms, showTashkeel]
  );
  const columnStyle: CSSProperties &
    Record<'--poem-scale' | '--poem-spacing' | '--poem-halves-spacing', number> = {
    '--poem-scale': fontScale,
    '--poem-spacing': spacingScale,
    '--poem-halves-spacing': halvesSpacing(spacingScale),
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
          fontScale={fontScale}
          spacingScale={spacingScale}
          spacingMin={lineByLine ? POEM_SCALE.spacing.halvesMin : POEM_SCALE.spacing.min}
          showTashkeel={showTashkeel}
          copyText={`${formatPoemText(shownVerses, poet.name)}\n${SITE_URL}${sharePath}`}
          onFontScaleChange={setFontScale}
          onSpacingScaleChange={setSpacingScale}
          onToggleTashkeel={() => setShowTashkeel((shown) => !shown)}
        >
          <ShareButton
            title={SHARE_TEXTS.poemTitle}
            body={formatPoemText(shownVerses.slice(0, 1), poet.name)}
            path={sharePath}
          />
        </PoemToolbar>
      </header>

      <div className="relative flex w-full flex-col items-center justify-between">
        <article className="flex w-full flex-col items-center p-6 md:p-8" onCopy={handleCopy}>
          <div
            ref={columnRef}
            className={cn(
              'flex w-full flex-col text-verse font-normal',
              lineByLine ? HEMISTICH_GAP : VERSE_GAP,
              isClassical
                ? 'w-fit max-w-full min-w-[min(calc(16em*var(--poem-scale)),100%)]'
                : 'px-(--poem-gutter)'
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
                    'relative flex w-full flex-col items-center justify-center text-center',
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
                        style={{ fontSize: `${fontScale}em` }}
                        lang="ar"
                        dir="rtl"
                        className={cn(
                          halves && (partIndex === 0 ? 'text-right sm:pe-12' : 'text-left sm:ps-12')
                        )}
                      >
                        {highlightRegex ? highlightVerse(text, highlightRegex) : text}
                      </p>
                    );
                  })}
                  {quoteRow === index && (
                    <QuoteShareButton
                      onShare={() => {
                        setQuoteRow(null);
                        quoteSheetRef.current?.showModal();
                      }}
                      onDismiss={() => setQuoteRow(null)}
                    />
                  )}
                </div>
              );
            })}
          </div>
        </article>
        {quote !== null && (
          <ShareSheet
            ref={quoteSheetRef}
            title={SHARE_TEXTS.quoteTitle}
            body={formatPoemQuote(quote, poet.name)}
            path={sharePath}
          />
        )}
      </div>
    </>
  );
}
