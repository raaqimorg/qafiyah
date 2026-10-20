import { describe, expect, it } from 'vitest';

import { isFeelingNavigation } from './morph-feeling-page';

const at = (path: string): URL => new URL(path, 'https://qafiyah.com');

describe('isFeelingNavigation', () => {
  it('morphs between the wheel and a feeling page', () => {
    expect(isFeelingNavigation(at('/feelings'), at('/feelings/alhuzn'))).toBe(true);
  });

  it('morphs between two pages of a feeling', () => {
    expect(isFeelingNavigation(at('/feelings/alhuzn'), at('/feelings/alhuzn?page=2'))).toBe(true);
  });

  it('leaves a navigation to another section to the default swap', () => {
    expect(isFeelingNavigation(at('/feelings/alhuzn'), at('/poems/qasida'))).toBe(false);
  });

  it('does not treat a path that only starts with the same letters as a feeling page', () => {
    expect(isFeelingNavigation(at('/feelings'), at('/feelingsx'))).toBe(false);
  });
});
