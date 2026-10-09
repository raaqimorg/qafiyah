import { FEELING_TREE, type Feeling } from '@/lib/feelings/feeling-tree';

export type Depth = 1 | 2 | 3;

type Point = { readonly x: number; readonly y: number };

type ViewBox = {
  readonly x: number;
  readonly y: number;
  readonly w: number;
  readonly h: number;
};

export type PlacedFeeling = {
  readonly key: string;
  readonly path: readonly number[];
  readonly slug: string;
  readonly name: string;
  readonly depth: Depth;
  readonly point: Point;
  readonly from: Point;
};

export type UprightLayout = {
  readonly viewBox: ViewBox;
  readonly items: readonly PlacedFeeling[];
  readonly transform: string;
};

export const NARROW_BELOW_PX = 560;

export const FONT_SIZE_BY_DEPTH = { 1: 26, 2: 22, 3: 21 } as const satisfies Record<Depth, number>;

export const TRUNK_LABEL_Y = -74;
export const STEM_TOP_Y = -52;

const VIEW_BOX = {
  narrow: { x: -400, y: -180, w: 800, h: 900 },
  wide: { x: -490, y: -100, w: 980, h: 620 },
} as const satisfies Record<'narrow' | 'wide', ViewBox>;

const ORIGIN: Point = { x: 0, y: 0 };
const TRUNK_BOUNDS: readonly Point[] = [
  { x: -70, y: -96 },
  { x: 70, y: -96 },
];
const MAX_ZOOM = 1.7;
const FIT_MARGIN = 0.94;
const GLYPH_WIDTH_RATIO = 0.55;
const LEAF_DROP = 140;

const round = (value: number): number => Math.round(value * 10) / 10;

const dome = (x: number, base: number, lift: number, half: number): number =>
  round(base - lift * Math.sqrt(Math.max(0, 1 - (x / half) ** 2)));

function rowPositionsRightToLeft(
  count: number,
  center: number,
  step: number,
  xMax: number
): number[] {
  const start = Math.max(
    -xMax,
    Math.min(center - (step * (count - 1)) / 2, xMax - step * (count - 1))
  );
  return Array.from({ length: count }, (_, index) => round(start + step * (count - 1 - index)));
}

function fitTransform(viewBox: ViewBox, points: readonly Point[]): string {
  const xs = points.map((point) => point.x);
  const ys = points.map((point) => point.y);
  const [minX, maxX, minY, maxY] = [
    Math.min(...xs),
    Math.max(...xs),
    Math.min(...ys),
    Math.max(...ys),
  ];
  const scale = Math.min(
    MAX_ZOOM,
    (viewBox.w * FIT_MARGIN) / Math.max(1, maxX - minX),
    (viewBox.h * FIT_MARGIN) / Math.max(1, maxY - minY)
  );
  const centerX = (minX + maxX) / 2;
  const centerY = (minY + maxY) / 2;
  return `translate(${viewBox.x + viewBox.w / 2}px, ${viewBox.y + viewBox.h / 2}px) scale(${scale}) translate(${-centerX}px, ${-centerY}px)`;
}

const flip = (point: Point): Point => ({ x: point.x, y: -point.y });

export function layoutUpright(path: readonly number[], narrow: boolean): UprightLayout {
  const viewBox = narrow ? VIEW_BOX.narrow : VIEW_BOX.wide;
  const xMax = narrow ? 300 : 440;
  const step = narrow ? { 1: 66, 2: 58, 3: 70 } : { 1: 128, 2: 118, 3: 128 };
  const items: PlacedFeeling[] = [];
  const place = (
    feeling: Feeling,
    at: readonly number[],
    depth: Depth,
    point: Point,
    from: Point
  ) => {
    items.push({
      key: at.join('-'),
      path: at,
      slug: feeling.slug,
      name: feeling.name,
      depth,
      point: flip(point),
      from: flip(from),
    });
  };

  // Right-to-left: the first core feeling sits on the right.
  rowPositionsRightToLeft(FEELING_TREE.length, 0, step[1], xMax).forEach((x, i) => {
    const core = FEELING_TREE[i];
    if (core === undefined) return;
    const corePoint = {
      x,
      y: dome(x, -120, narrow ? 30 : 90, 470) - (narrow && i % 2 === 1 ? 54 : 0),
    };
    place(core, [i], 1, corePoint, ORIGIN);
    if (path[0] !== i) return;

    const subs = core.children;
    const stagger = narrow ? subs.length > 2 : subs.length > 6;
    const subStep = !narrow && stagger ? 98 : step[2] * (narrow && !stagger ? 2 : 1);
    rowPositionsRightToLeft(subs.length, x, subStep, xMax).forEach((subX, j) => {
      const sub = subs[j];
      if (sub === undefined) return;
      const staggerLift = narrow ? 48 : 34;
      const lift = stagger && j % 2 === 1 ? staggerLift : 0;
      const subPoint = { x: subX, y: dome(subX, narrow ? -310 : -300, 40, 520) - lift };
      place(sub, [i, j], 2, subPoint, corePoint);
      if (path[1] !== j) return;

      const leaves = sub.children;
      const leafStagger = narrow && leaves.length > 2;
      const leafStep = leafStagger ? step[3] : step[3] * (narrow ? 1.6 : 1);
      rowPositionsRightToLeft(leaves.length, subX, leafStep, xMax).forEach((leafX, k) => {
        const leaf = leaves[k];
        if (leaf === undefined) return;
        const leafPoint = {
          x: leafX,
          y: subPoint.y - LEAF_DROP - (leafStagger && k % 2 === 1 ? 44 : 0),
        };
        place(leaf, [i, j, k], 3, leafPoint, subPoint);
      });
    });
  });

  const bounds = items.flatMap(({ name, depth, point }) => {
    const half = (name.length * FONT_SIZE_BY_DEPTH[depth] * GLYPH_WIDTH_RATIO) / 2;
    return [
      { x: point.x - half, y: point.y - FONT_SIZE_BY_DEPTH[depth] },
      { x: point.x + half, y: point.y + FONT_SIZE_BY_DEPTH[depth] },
    ];
  });
  return { viewBox, items, transform: fitTransform(viewBox, [...TRUNK_BOUNDS, ...bounds]) };
}

export function edgePath({ from, point }: PlacedFeeling): string {
  const middle = (from.y + point.y) / 2;
  return `M${from.x} ${from.y}C${from.x} ${middle} ${point.x} ${middle} ${point.x} ${point.y}`;
}
