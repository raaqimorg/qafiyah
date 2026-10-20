export type Feeling = {
  readonly slug: string;
  readonly name: string;
  readonly verseCount: number;
  readonly children: readonly Feeling[];
};

type FeelingInput = {
  readonly slug: string;
  readonly name: string;
  readonly verseCount: number;
  readonly parentSlug?: string | null | undefined;
};

export function buildFeelingTree(rows: readonly FeelingInput[]): readonly Feeling[] {
  const childrenOf = (parent: string | null): readonly Feeling[] =>
    rows
      .filter((row) => (row.parentSlug ?? null) === parent)
      .map((row) => ({
        slug: row.slug,
        name: row.name,
        verseCount: row.verseCount,
        children: childrenOf(row.slug),
      }));
  return childrenOf(null);
}

export function pathOf(tree: readonly Feeling[], slug: string): readonly number[] | null {
  for (const [index, feeling] of tree.entries()) {
    if (feeling.slug === slug) return [index];
    const below = pathOf(feeling.children, slug);
    if (below) return [index, ...below];
  }
  return null;
}
