import { describe, expect, it } from 'vitest';

import { FEELING_TREE, type Feeling } from './feeling-tree';

type Entry = { readonly feeling: Feeling; readonly depth: number };

const flatten = (feelings: readonly Feeling[], depth = 1): readonly Entry[] =>
  feelings.flatMap((feeling) => [{ feeling, depth }, ...flatten(feeling.children, depth + 1)]);

const ENTRIES = flatten(FEELING_TREE);

const duplicatesOf = (values: readonly string[]): readonly string[] =>
  values.filter((value, index) => values.indexOf(value) !== index);

describe('FEELING_TREE', () => {
  it('gives every feeling a unique slug, since labels point at slugs', () => {
    expect(duplicatesOf(ENTRIES.map((entry) => entry.feeling.slug))).toEqual([]);
  });

  it('names every feeling once, so no word sits under two branches', () => {
    expect(duplicatesOf(ENTRIES.map((entry) => entry.feeling.name))).toEqual([]);
  });

  it('uses lowercase ASCII slugs like the other taxonomies', () => {
    const invalid = ENTRIES.map((entry) => entry.feeling.slug).filter(
      (slug) => !/^[a-z]+$/u.test(slug)
    );
    expect(invalid).toEqual([]);
  });

  it('ends every branch at the third level, where the quiz expects a precise feeling', () => {
    const misplaced = ENTRIES.filter(
      (entry) => (entry.feeling.children.length === 0) !== (entry.depth === 3)
    ).map((entry) => entry.feeling.name);
    expect(misplaced).toEqual([]);
  });
});
