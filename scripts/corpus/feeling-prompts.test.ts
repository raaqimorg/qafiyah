import { describe, expect, it } from 'bun:test';

import { classifyMessages, verifyMessages } from './feeling-prompts';

const feelings = [
  { slug: 'alhuzn', name: 'الحزن', wheelTerm: 'Sad', description: 'ضيق النفس وانكسارها' },
  { slug: 'alshawq', name: 'الشوق', wheelTerm: 'Longing', description: 'نزوع النفس إلى غائب' },
];

describe('classifyMessages', () => {
  const { system, user } = classifyMessages(feelings, ['قفا نبك من ذكرى', 'ألا هبي بصحنك']);

  it('lists every feeling with its name, wheel term, and description', () => {
    expect(system).toContain('alhuzn: الحزن (Sad): ضيق النفس وانكسارها\nalshawq: الشوق (Longing)');
  });

  it('asks for up to three rated feelings a verse, as JSON', () => {
    expect(system).toContain('up to three, strongest first');
    expect(system).toContain('{"1": [["alhuzn", 3]], "2": []}');
  });

  it('numbers the verses from 1, one a line', () => {
    expect(user).toBe('1. قفا نبك من ذكرى\n2. ألا هبي بصحنك');
  });
});

describe('verifyMessages', () => {
  it('pairs each verse with the feeling to check', () => {
    const { user } = verifyMessages(feelings, [
      ['قفا نبك من ذكرى', 'alhuzn'],
      ['ألا هبي بصحنك', 'alshawq'],
    ]);
    expect(user).toBe(
      '1. Feeling: الحزن (Sad): ضيق النفس وانكسارها\n   Verse: قفا نبك من ذكرى\n' +
        '2. Feeling: الشوق (Longing): نزوع النفس إلى غائب\n   Verse: ألا هبي بصحنك'
    );
  });

  it('asks for yes or no, as JSON', () => {
    expect(verifyMessages(feelings, []).system).toContain('{"1": "yes", "2": "no"}');
  });

  it('refuses a feeling that the list does not have', () => {
    expect(() => verifyMessages(feelings, [['قفا نبك', 'alfarah']])).toThrow('alfarah');
  });
});
