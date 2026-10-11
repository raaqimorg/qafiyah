import { describe, expect, test } from 'bun:test';

import { DEFAULT_SOUND_CLASSES } from './arabic-text';
import { assessPoem, parseCorpusLine, proposedLabel, type CorpusRow } from './verify-classical';

const row = (overrides: Partial<CorpusRow> = {}): CorpusRow => ({
  id: '1',
  slug: 'AbCd',
  poemType: 'amudi',
  meter: 'altawil',
  rhyme: 'lam',
  body: 'قفا نبك*بسقط اللوى',
  ...overrides,
});

describe('parseCorpusLine', () => {
  test('reads the six exported fields', () => {
    const parsed = parseCorpusLine('7\tAbCd\tamudi\taltawil\tlam\tقفا نبك*بسقط اللوى');
    expect(parsed).toEqual({
      id: '7',
      slug: 'AbCd',
      poemType: 'amudi',
      meter: 'altawil',
      rhyme: 'lam',
      body: 'قفا نبك*بسقط اللوى',
    });
  });

  test('rejects a line with a missing body', () => {
    expect(parseCorpusLine('7\tAbCd\tamudi\taltawil\tlam')).toBeNull();
  });

  test('rejects a blank line', () => {
    expect(parseCorpusLine('   ')).toBeNull();
  });
});

describe('assessPoem', () => {
  test('picks the flat hypothesis that scores highest and reports it', () => {
    const flat = row({
      poemType: 'hurr',
      body: 'المعاني|الأماني|التهاني|الغواني|الدواني',
    });
    const result = assessPoem(flat, DEFAULT_SOUND_CLASSES);
    expect(result.hypothesis).toBe('flat-every');
    expect(result.assessment.verdict).toBe('verified');
  });

  test('marks an irregular poem untestable', () => {
    const result = assessPoem(row({ body: 'جزء*جزء*جزء' }), DEFAULT_SOUND_CLASSES);
    expect(result.assessment).toEqual({ verdict: 'untestable', reason: 'irregular-structure' });
  });
});

describe('proposedLabel', () => {
  test('promotes a verified non-classical poem', () => {
    expect(proposedLabel(row({ poemType: 'hurr' }), 'verified')).toBe('amudi');
  });

  test('leaves a verified classical or nabati poem alone', () => {
    expect(proposedLabel(row({ poemType: 'amudi' }), 'verified')).toBeNull();
    expect(proposedLabel(row({ poemType: 'nabati' }), 'verified')).toBeNull();
  });

  test('never proposes a type for a contradicted poem', () => {
    expect(proposedLabel(row({ poemType: 'amudi' }), 'contradicted')).toBeNull();
    expect(proposedLabel(row({ poemType: 'hurr' }), 'contradicted')).toBeNull();
  });

  test('proposes nothing for an untestable poem', () => {
    expect(proposedLabel(row({ poemType: 'amudi' }), 'untestable')).toBeNull();
    expect(proposedLabel(row({ poemType: 'hurr' }), 'untestable')).toBeNull();
  });
});
