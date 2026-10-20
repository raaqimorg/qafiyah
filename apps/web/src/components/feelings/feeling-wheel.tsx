import {
  FONT_SIZE_BY_DEPTH,
  STEM_TOP_Y,
  TRUNK_LABEL_Y,
  edgePath,
  layoutUpright,
  type UprightLayout,
} from '@/lib/feelings/upright-layout';
import { feelingUrl } from '@/lib/urls';
import { cn } from '@/lib/utils';

import type { Feeling } from '@/lib/feelings/feeling-tree';

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

const TRUNK_LABEL = 'شجرة المشاعر';
const TRUNK_FONT_SIZE = 26;
const LINK_CLASS = cn(
  'transition-opacity outline-none focus-visible:[&>text]:underline',
  'data-leaving:pointer-events-none data-leaving:opacity-0 data-leaving:duration-(--leave-ms) data-leaving:ease-out'
);

const startsWith = (path: readonly number[], prefix: readonly number[]): boolean =>
  prefix.length <= path.length && prefix.every((value, index) => path[index] === value);

function statusOf(nodePath: readonly number[], path: readonly number[]): Status {
  if (startsWith(path, nodePath)) return 'chosen';
  const parent = nodePath.slice(0, -1);
  if (startsWith(path, parent)) return parent.length === path.length ? 'next' : 'sibling';
  if (path.length === 0) return 'resting';
  return nodePath[0] === path[0] ? 'near' : 'far';
}

type Variant = 'wide' | 'narrow';

type FeelingTreeProps = {
  readonly variant: Variant;
  readonly layout: UprightLayout;
  readonly path: readonly number[];
  readonly className: string;
};

function FeelingTree({ variant, layout, path, className }: FeelingTreeProps) {
  const { viewBox } = layout;
  const nodeId = (part: string, key: string): string => `feeling-${variant}-${part}-${key}`;
  return (
    <div
      id={`feeling-frame-${variant}`}
      style={{ aspectRatio: `${viewBox.w} / ${layout.height}` }}
      className={cn(
        'overflow-hidden motion-safe:transition-[aspect-ratio] motion-safe:duration-700 motion-safe:ease-in-out',
        className
      )}
    >
      <svg
        id={`feeling-tree-${variant}`}
        viewBox={`${viewBox.x} ${viewBox.y} ${viewBox.w} ${viewBox.h}`}
        direction="rtl"
        role="group"
        aria-label="شجرة المشاعر"
        className="block h-auto w-full select-none"
      >
        <g
          id={`feeling-zoom-${variant}`}
          className="motion-safe:transition-transform motion-safe:duration-700 motion-safe:ease-in-out"
          style={{
            transform: layout.transform,
            transformOrigin: '0 0',
            transformBox: 'view-box',
          }}
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
              id={nodeId('edge', item.key)}
              data-feeling-node=""
              d={edgePath(item)}
              fill="none"
              pathLength={1}
              strokeDasharray={1}
              strokeLinecap="round"
              style={{ animationDelay: `${item.depth === 1 ? 0 : 120}ms` }}
              className={cn(
                'transition-opacity duration-500 motion-safe:data-entering:animate-feeling-draw',
                'data-leaving:pointer-events-none data-leaving:opacity-0 data-leaving:duration-(--leave-ms) data-leaving:ease-out',
                EDGE_CLASS[statusOf(item.path, path)]
              )}
            />
          ))}
          {layout.items.map((item) => (
            <a
              key={item.key}
              id={nodeId('label', item.key)}
              data-feeling-node=""
              href={feelingUrl(item.slug)}
              aria-current={
                item.path.length === path.length && startsWith(path, item.path) ? 'page' : undefined
              }
              className={LINK_CLASS}
            >
              <text
                x={item.point.x}
                y={item.point.y}
                textAnchor="middle"
                dominantBaseline="central"
                fontSize={FONT_SIZE_BY_DEPTH[item.depth]}
                strokeWidth={6}
                strokeLinejoin="round"
                style={{
                  paintOrder: 'stroke',
                  animationDelay: `${item.depth === 1 ? 250 : 380}ms`,
                }}
                className={cn(
                  'cursor-pointer stroke-surface transition-[opacity,fill] duration-500',
                  'motion-safe:in-data-entering:animate-feeling-fade',
                  LABEL_CLASS[statusOf(item.path, path)]
                )}
              >
                {item.name}
              </text>
            </a>
          ))}
          <a href={feelingUrl()} className={LINK_CLASS}>
            <text
              x={0}
              y={TRUNK_LABEL_Y}
              textAnchor="middle"
              dominantBaseline="central"
              fontSize={TRUNK_FONT_SIZE}
              className="cursor-pointer fill-text-muted hover:fill-text"
            >
              {TRUNK_LABEL}
            </text>
          </a>
        </g>
      </svg>
    </div>
  );
}

export function FeelingWheel({
  tree,
  path,
}: {
  readonly tree: readonly Feeling[];
  readonly path: readonly number[];
}) {
  return (
    <div className="@container">
      <FeelingTree
        variant="wide"
        layout={layoutUpright(tree, path, false)}
        path={path}
        className="hidden @min-[560px]:block"
      />
      <FeelingTree
        variant="narrow"
        layout={layoutUpright(tree, path, true)}
        path={path}
        className="@min-[560px]:hidden"
      />
    </div>
  );
}
