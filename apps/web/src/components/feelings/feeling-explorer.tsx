import { useEffect, useRef, useState } from 'react';

import { FeelingVerseItem } from '@/components/feelings/feeling-verse-item';
import { TYPE } from '@/lib/constants/design-tokens';
import { FEELING_VERSES } from '@/lib/feelings/feeling-verses';
import {
  FONT_SIZE_BY_DEPTH,
  NARROW_BELOW_PX,
  STEM_TOP_Y,
  TRUNK_LABEL_Y,
  edgePath,
  layoutUpright,
} from '@/lib/feelings/upright-layout';
import { cn } from '@/lib/utils';

import type { KeyboardEvent } from 'react';

type Status = 'chosen' | 'next' | 'sibling' | 'near' | 'far' | 'resting';

const LABEL_CLASS = {
  chosen: 'fill-highlight',
  next: 'fill-text hover:fill-highlight',
  sibling: 'fill-text opacity-55 hover:fill-highlight hover:opacity-100',
  near: 'fill-text opacity-30 hover:fill-highlight hover:opacity-100',
  far: 'fill-text opacity-10 hover:opacity-60',
  resting: 'fill-text hover:fill-highlight',
} as const satisfies Record<Status, string>;

const EDGE_CLASS = {
  chosen: 'stroke-highlight [stroke-width:2]',
  next: 'stroke-text-subtle opacity-85',
  sibling: 'stroke-text-subtle opacity-45',
  near: 'stroke-text-subtle opacity-25',
  far: 'stroke-text-subtle opacity-10',
  resting: 'stroke-text-subtle opacity-50',
} as const satisfies Record<Status, string>;

const TRUNK_LABEL = 'بمَ تشعر؟';
const TRUNK_FONT_SIZE = 26;

const startsWith = (path: readonly number[], prefix: readonly number[]): boolean =>
  prefix.length <= path.length && prefix.every((value, index) => path[index] === value);

function statusOf(nodePath: readonly number[], path: readonly number[]): Status {
  if (startsWith(path, nodePath)) return 'chosen';
  const parent = nodePath.slice(0, -1);
  if (startsWith(path, parent)) return parent.length === path.length ? 'next' : 'sibling';
  if (path.length === 0) return 'resting';
  return nodePath[0] === path[0] ? 'near' : 'far';
}

const onActivate =
  (action: () => void) =>
  (event: KeyboardEvent<SVGTextElement>): void => {
    if (event.key !== 'Enter' && event.key !== ' ') return;
    event.preventDefault();
    action();
  };

export function FeelingExplorer() {
  const [path, setPath] = useState<readonly number[]>([]);
  const [narrow, setNarrow] = useState(false);
  const stageRef = useRef<HTMLDivElement>(null);
  const resultRef = useRef<HTMLDivElement>(null);

  const layout = layoutUpright(path, narrow);
  const chosen = layout.items.filter((item) => startsWith(path, item.path));
  const names = chosen.map((item) => item.name);
  const isComplete = names.length === 3;
  const verses = isComplete ? (FEELING_VERSES[chosen.at(-1)?.slug ?? ''] ?? []) : [];

  useEffect(() => {
    const stage = stageRef.current;
    if (stage === null) return undefined;
    const observer = new ResizeObserver(([entry]) => {
      if (entry !== undefined) setNarrow(entry.contentRect.width < NARROW_BELOW_PX);
    });
    observer.observe(stage);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (!isComplete) return;
    const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    resultRef.current?.scrollIntoView({
      behavior: reduceMotion ? 'auto' : 'smooth',
      block: 'nearest',
    });
  }, [isComplete]);

  const reset = () => setPath([]);
  const { viewBox } = layout;

  return (
    <div className="flex flex-col gap-8">
      <div ref={stageRef}>
        <svg
          viewBox={`${viewBox.x} ${viewBox.y} ${viewBox.w} ${viewBox.h}`}
          direction="rtl"
          role="group"
          aria-label="شجرة المشاعر"
          className="h-auto w-full select-none"
        >
          <g
            style={{
              transform: layout.transform,
              transformOrigin: '0 0',
              transformBox: 'view-box',
            }}
            className="motion-safe:transition-transform motion-safe:duration-500 motion-safe:ease-out"
          >
            <path
              d={`M0 ${STEM_TOP_Y} L0 0`}
              fill="none"
              strokeLinecap="round"
              className="stroke-highlight [stroke-width:2]"
            />
            {layout.items.map((item) => (
              <path
                key={item.key}
                d={edgePath(item)}
                fill="none"
                pathLength={1}
                strokeDasharray={1}
                strokeLinecap="round"
                style={{ animationDelay: `${item.depth === 1 ? 0 : 120}ms` }}
                className={cn(
                  'transition-opacity duration-500',
                  'motion-safe:animate-[feeling-draw_600ms_ease-out_backwards]',
                  EDGE_CLASS[statusOf(item.path, path)]
                )}
              />
            ))}
            {layout.items.map((item) => {
              const status = statusOf(item.path, path);
              const choose = () => setPath(item.path);
              return (
                <text
                  key={item.key}
                  x={item.point.x}
                  y={item.point.y}
                  textAnchor="middle"
                  dominantBaseline="central"
                  fontSize={FONT_SIZE_BY_DEPTH[item.depth]}
                  strokeWidth={6}
                  strokeLinejoin="round"
                  role="button"
                  tabIndex={0}
                  aria-pressed={status === 'chosen'}
                  onClick={choose}
                  onKeyDown={onActivate(choose)}
                  style={{
                    paintOrder: 'stroke',
                    animationDelay: `${item.depth === 1 ? 250 : 380}ms`,
                  }}
                  className={cn(
                    'cursor-pointer stroke-surface transition-[opacity,fill] duration-500 outline-none focus-visible:underline',
                    'motion-safe:animate-[feeling-fade_500ms_ease-out_backwards]',
                    LABEL_CLASS[status]
                  )}
                >
                  {item.name}
                </text>
              );
            })}
            <text
              x={0}
              y={TRUNK_LABEL_Y}
              textAnchor="middle"
              dominantBaseline="central"
              fontSize={TRUNK_FONT_SIZE}
              role="button"
              tabIndex={0}
              aria-label="من جديد"
              onClick={reset}
              onKeyDown={onActivate(reset)}
              className="cursor-pointer fill-text-muted outline-none hover:fill-text focus-visible:underline"
            >
              {TRUNK_LABEL}
            </text>
          </g>
        </svg>
      </div>

      {isComplete && (
        <div
          ref={resultRef}
          className="flex flex-col gap-3 border-t border-border pt-8 motion-safe:animate-[feeling-fade_400ms_ease-out_backwards]"
        >
          <p className={cn(TYPE.caption, 'text-text-subtle')}>{names.join(' ← ')}</p>
          <p className={cn(TYPE.heading, 'text-highlight')}>{names.at(-1)}</p>
          {verses.length > 0 ? (
            <ul className="flex flex-col">
              {verses.map((verse) => (
                <FeelingVerseItem key={`${verse.poem}:${verse.row}`} verse={verse} />
              ))}
            </ul>
          ) : (
            <p className={cn(TYPE.body, 'text-text-muted')}>لم نجد لهذا الشعور أبياتًا بعد.</p>
          )}
          <p className={cn(TYPE.caption, 'text-text-subtle')}>
            اختيرت هذه الأبيات من نتائج البحث عن الكلمة نفسها، لا عن معناها، فقد تفوتها أبيات تعبّر عن
            الشعور بألفاظ أخرى.
          </p>
          <button
            type="button"
            onClick={reset}
            className={cn(
              TYPE.body,
              'self-start rounded-sm text-text-muted underline underline-offset-4 focus-ring hover:text-text'
            )}
          >
            من جديد
          </button>
        </div>
      )}
    </div>
  );
}
