import { describe, expect, it } from 'bun:test';

import { keptTags, parseClassifyReply, parseVerifyReply } from './feeling-replies';

const slugs = new Set(['alhuzn', 'alshawq', 'alfakhr', 'alghadab']);

describe('parseClassifyReply', () => {
  it("reads each verse's rated feelings in order", () => {
    expect(parseClassifyReply('{"2": [], "1": [["alhuzn", 3], ["alshawq", 2]]}', 2, slugs)).toEqual(
      [
        [
          ['alhuzn', 3],
          ['alshawq', 2],
        ],
        [],
      ]
    );
  });

  it('drops unknown feelings, ratings outside 1 to 3, and malformed pairs', () => {
    const reply = '{"1": [["alfarah", 3], ["alhuzn", 4], ["alhuzn"], "alhuzn", ["alshawq", 1]]}';
    expect(parseClassifyReply(reply, 1, slugs)).toEqual([[['alshawq', 1]]]);
  });

  it('reads a rating sent as a string', () => {
    expect(parseClassifyReply('{"1": [["alhuzn", "2"]]}', 1, slugs)).toEqual([[['alhuzn', 2]]]);
  });

  it('keeps at most three feelings a verse', () => {
    const reply = '{"1": [["alhuzn", 3], ["alshawq", 2], ["alfakhr", 2], ["alghadab", 1]]}';
    expect(parseClassifyReply(reply, 1, slugs)[0]).toHaveLength(3);
  });

  it('refuses a reply that leaves out a verse', () => {
    expect(() => parseClassifyReply('{"1": []}', 2, slugs)).toThrow('verse 2');
  });

  it('refuses a verse whose feelings are not a list', () => {
    expect(() => parseClassifyReply('{"1": "alhuzn"}', 1, slugs)).toThrow('verse 1');
  });
});

describe('parseVerifyReply', () => {
  it('reads yes and no in order', () => {
    expect(parseVerifyReply('{"2": "no", "1": "yes"}', 2)).toEqual([true, false]);
  });

  it('refuses an answer that is not yes or no', () => {
    expect(() => parseVerifyReply('{"1": "yes", "2": "maybe"}', 2)).toThrow('item 2');
  });
});

describe('keptTags', () => {
  it('keeps the strong feelings and the confirmed clear ones, and drops the faint ones', () => {
    const tags = [
      ['alhuzn', 3],
      ['alshawq', 2],
      ['alfakhr', 2],
      ['alghadab', 1],
    ] as const;
    expect(keptTags(tags, (slug) => slug === 'alshawq')).toEqual([
      ['alhuzn', 3],
      ['alshawq', 2],
    ]);
  });
});
