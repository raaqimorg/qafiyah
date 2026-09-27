import { describe, expect, test } from 'bun:test';

import {
  checkTaxonomyOptionsFollowNewestDump,
  type History,
  TAXONOMY_OPTIONS_PATH,
} from './taxonomy-dump';

const TRACKED = [
  'data/db/0036_26_09_2026/CHANGES.md.enc',
  'data/db/0037_27_09_2026/CHANGES.md.enc',
  'data/db/0000_default/manifest.json',
];

function historyOf(commits: Readonly<Record<string, string>>, order: readonly string[]): History {
  return {
    lastCommitTouching: (path) => commits[path],
    isAncestor: (ancestor, descendant) => order.indexOf(ancestor) <= order.indexOf(descendant),
  };
}

describe('checkTaxonomyOptionsFollowNewestDump', () => {
  test('passes when the options were regenerated after the newest dump was committed', () => {
    const history = historyOf(
      { 'data/db/0037_27_09_2026': 'dump', [TAXONOMY_OPTIONS_PATH]: 'options' },
      ['dump', 'options']
    );
    expect(checkTaxonomyOptionsFollowNewestDump(TRACKED, history).isOk()).toBe(true);
  });

  test('passes when the options were regenerated in the same commit as the dump', () => {
    const history = historyOf(
      { 'data/db/0037_27_09_2026': 'snapshot', [TAXONOMY_OPTIONS_PATH]: 'snapshot' },
      ['snapshot']
    );
    expect(checkTaxonomyOptionsFollowNewestDump(TRACKED, history).isOk()).toBe(true);
  });

  test('refuses a newest dump committed after the last regeneration of the options', () => {
    const history = historyOf(
      { 'data/db/0037_27_09_2026': 'dump', [TAXONOMY_OPTIONS_PATH]: 'options' },
      ['options', 'dump']
    );
    const result = checkTaxonomyOptionsFollowNewestDump(TRACKED, history);
    expect(result.isErr()).toBe(true);
    expect(result._unsafeUnwrapErr()).toContain('0037_27_09_2026');
  });

  test('compares against the newest dump, not an older one', () => {
    const history = historyOf(
      {
        'data/db/0036_26_09_2026': 'old-dump',
        'data/db/0037_27_09_2026': 'new-dump',
        [TAXONOMY_OPTIONS_PATH]: 'options',
      },
      ['old-dump', 'options', 'new-dump']
    );
    expect(checkTaxonomyOptionsFollowNewestDump(TRACKED, history).isErr()).toBe(true);
  });

  test('passes when no dump is committed yet', () => {
    const history = historyOf({}, []);
    expect(
      checkTaxonomyOptionsFollowNewestDump(['data/db/0000_default/manifest.json'], history).isOk()
    ).toBe(true);
  });
});
