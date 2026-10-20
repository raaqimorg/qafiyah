import { describe, expect, it } from 'vitest';

import { buildFeelingTree, pathOf } from './feeling-tree';

const row = (slug: string, parentSlug?: string, verseCount = 1) => ({
  slug,
  name: `ال${slug}`,
  verseCount,
  ...(parentSlug ? { parentSlug } : {}),
});

const ROWS = [
  row('sad', undefined, 5),
  row('lonely', 'sad', 3),
  row('isolated', 'lonely'),
  row('happy'),
];

describe('buildFeelingTree', () => {
  it('nests each feeling under its parent in list order', () => {
    const tree = buildFeelingTree(ROWS);
    expect(tree.map((core) => core.slug)).toEqual(['sad', 'happy']);
    expect(tree[0]?.children[0]?.children[0]?.slug).toBe('isolated');
    expect(tree[0]?.verseCount).toBe(5);
  });
});

describe('pathOf', () => {
  it('gives the index path of a feeling, and null for an unknown slug', () => {
    const tree = buildFeelingTree(ROWS);
    expect(pathOf(tree, 'isolated')).toEqual([0, 0, 0]);
    expect(pathOf(tree, 'happy')).toEqual([1]);
    expect(pathOf(tree, 'nothing')).toBeNull();
  });
});
