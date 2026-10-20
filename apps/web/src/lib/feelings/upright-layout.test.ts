import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { buildFeelingTree } from './feeling-tree';
import {
  FRAME_PAD,
  labelBox,
  layoutUpright,
  type Point,
  type UprightLayout,
} from './upright-layout';

const SEED = readFileSync(
  fileURLToPath(new URL('../../../../../scripts/db/sql/feelings.sql', import.meta.url)),
  'utf8'
);
const SEED_ROWS = [...SEED.matchAll(/\((\d+), (NULL|\d+), '([^']+)', '([^']+)', '[^']*', \d+\)/gu)];
const SLUG_BY_ID = new Map(SEED_ROWS.map((m) => [m[1], m[3] ?? '']));
const SEED_TREE = buildFeelingTree(
  SEED_ROWS.map((m) => ({
    slug: m[3] ?? '',
    name: m[4] ?? '',
    verseCount: 1,
    ...(m[2] === 'NULL' ? {} : { parentSlug: SLUG_BY_ID.get(m[2] ?? '') ?? '' }),
  }))
);

const PATHS: readonly (readonly number[])[] = [
  [],
  ...SEED_TREE.flatMap((core, i) => [
    [i],
    ...core.children.flatMap((sub, j) => [[i, j], ...sub.children.map((_, k) => [i, j, k])]),
  ]),
];

const WIDTHS = [
  { name: 'wide', narrow: false },
  { name: 'narrow', narrow: true },
] as const;

const toView = ({ viewBox, scale, anchor }: UprightLayout, point: Point): Point => ({
  x: viewBox.x + viewBox.w / 2 + scale * (point.x - anchor.x),
  y: viewBox.y + FRAME_PAD + scale * (point.y - anchor.y),
});

const outside = (layout: UprightLayout): readonly string[] =>
  layout.items
    .filter((item) =>
      labelBox(item)
        .map((point) => toView(layout, point))
        .some(
          ({ x, y }) =>
            x < layout.viewBox.x ||
            x > layout.viewBox.x + layout.viewBox.w ||
            y < layout.viewBox.y ||
            y > layout.viewBox.y + layout.height
        )
    )
    .map((item) => item.name);

describe('layoutUpright', () => {
  it('reads all 129 feelings from the seed', () => {
    expect(SEED_ROWS).toHaveLength(129);
  });

  it.each(WIDTHS)('keeps every label inside the view box of the $name tree', ({ narrow }) => {
    const escaped = PATHS.flatMap((path) =>
      outside(layoutUpright(SEED_TREE, path, narrow)).map((name) => `${path.join('-')}: ${name}`)
    );
    expect(escaped).toEqual([]);
  });

  it.each(WIDTHS)(
    'ends the $name frame just under its lowest label, with no empty band',
    ({ narrow }) => {
      const bands = PATHS.map((path) => {
        const layout = layoutUpright(SEED_TREE, path, narrow);
        const lowest = Math.max(
          ...layout.items.flatMap((item) => labelBox(item).map((point) => toView(layout, point).y))
        );
        return layout.viewBox.y + layout.height - lowest;
      });
      expect(Math.max(...bands)).toBeLessThanOrEqual(FRAME_PAD + 1);
    }
  );

  it('places the same feelings in the wide and the narrow tree, so CSS can show either', () => {
    const differ = PATHS.filter((path) => {
      const keys = (narrow: boolean) =>
        layoutUpright(SEED_TREE, path, narrow).items.map((item) => item.key);
      return keys(false).join() !== keys(true).join();
    });
    expect(differ).toEqual([]);
  });

  it.each(WIDTHS)('places the first core feeling on the right of the $name tree', ({ narrow }) => {
    const xs = layoutUpright(SEED_TREE, [], narrow).items.map((item) => item.point.x);
    expect(xs[0]).toBe(Math.max(...xs));
  });
});
