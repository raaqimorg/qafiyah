import { describe, expect, test } from 'bun:test';

import { buildUpdateStatements, parseTransitions } from './apply-verdicts';

const CSV = [
  'id,slug,current,proposed,structure,hypothesis,ajuz_count,rhyme_score,cv,mad_outliers,parity_warning,reason',
  '7,AbCd,hurr,amudi,flat,flat-every,9,1,0.08,0,false,rhyme-verified',
  '9,EfGh,hurr,amudi,starred,starred,14,1,0.09,0,false,rhyme-verified',
].join('\n');

describe('parseTransitions', () => {
  test('reads id, current, and proposed and skips the header', () => {
    expect(parseTransitions(CSV)).toEqual([
      { id: '7', current: 'hurr', proposed: 'amudi' },
      { id: '9', current: 'hurr', proposed: 'amudi' },
    ]);
  });

  test('ignores a trailing blank line', () => {
    expect(parseTransitions(`${CSV}\n`)).toHaveLength(2);
  });
});

describe('buildUpdateStatements', () => {
  test('groups poems by target label into one statement each', () => {
    const statements = buildUpdateStatements(parseTransitions(CSV));
    expect(statements).toHaveLength(1);
    expect(statements[0]).toContain("slug = 'amudi'");
    expect(statements[0]).toContain('(7, 9)');
  });

  test('rejects a non-numeric id rather than interpolating it', () => {
    expect(() =>
      buildUpdateStatements([{ id: '7; DROP TABLE poems', current: 'hurr', proposed: 'amudi' }])
    ).toThrow();
  });

  test('rejects an unknown target label', () => {
    expect(() =>
      buildUpdateStatements([{ id: '7', current: 'hurr', proposed: 'qasida' }])
    ).toThrow();
  });

  test('rejects a removed poem type as a target', () => {
    expect(() =>
      buildUpdateStatements([{ id: '9', current: 'amudi', proposed: 'majhul' }])
    ).toThrow();
  });
});
