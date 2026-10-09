import { describe, expect, it } from 'vitest';

import { FEELING_TREE, type Feeling } from './feeling-tree';
import { FEELING_VERSES } from './feeling-verses';

const leafSlugs = (feelings: readonly Feeling[]): readonly string[] =>
  feelings.flatMap((feeling) =>
    feeling.children.length === 0 ? [feeling.slug] : leafSlugs(feeling.children)
  );

const LEAVES = leafSlugs(FEELING_TREE);

describe('FEELING_VERSES', () => {
  it('only lists precise feelings that exist in the tree', () => {
    expect(Object.keys(FEELING_VERSES).filter((slug) => !LEAVES.includes(slug))).toEqual([]);
  });

  it('gives every precise feeling at least one verse', () => {
    expect(LEAVES.filter((slug) => (FEELING_VERSES[slug] ?? []).length === 0)).toEqual([]);
  });

  it('marks the matched word in every verse, since the quiz matches by word', () => {
    const unmarked = Object.values(FEELING_VERSES)
      .flat()
      .filter((verse) => !verse.row.includes('<mark>'));
    expect(unmarked).toEqual([]);
  });
});
