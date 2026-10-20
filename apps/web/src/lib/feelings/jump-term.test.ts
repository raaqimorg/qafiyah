import { describe, expect, it } from 'vitest';

import { MAX_HIGHLIGHT_TERM_LENGTH } from '@/lib/highlight';

import { jumpTerm } from './jump-term';

describe('jumpTerm', () => {
  it('is the first part, trimmed', () => {
    expect(jumpTerm(['  قفا نبك  ', 'بسقط اللوى'])).toBe('قفا نبك');
  });

  it('cuts a long first part at a word boundary within the highlight limit', () => {
    const long = Array.from({ length: 30 }, () => 'كلمة').join(' ');
    const term = jumpTerm([long, 'x']);
    expect(term.length).toBeLessThanOrEqual(MAX_HIGHLIGHT_TERM_LENGTH);
    expect(long.startsWith(term)).toBe(true);
    expect(term.endsWith(' ')).toBe(false);
  });
});
