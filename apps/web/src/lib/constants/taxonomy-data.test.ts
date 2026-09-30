import { describe, expect, it } from 'vitest';

import { sortMeterOptions } from './taxonomy-data';

describe('sortMeterOptions', () => {
  it('orders the standard meters by their classical order first', () => {
    const options = [
      { value: 'alrajz', label: 'الرجز', poemsCount: 10 },
      { value: 'altawil', label: 'الطويل', poemsCount: 5 },
      { value: 'custom', label: 'مخصص', poemsCount: 7 },
    ];
    expect(sortMeterOptions(options).map((o) => o.value)).toEqual(['altawil', 'alrajz', 'custom']);
  });

  it('sorts unknown meters after the standard ones by poem count descending', () => {
    const options = [
      { value: 'a', label: 'أ', poemsCount: 2 },
      { value: 'b', label: 'ب', poemsCount: 9 },
    ];
    expect(sortMeterOptions(options).map((o) => o.value)).toEqual(['b', 'a']);
  });
});
