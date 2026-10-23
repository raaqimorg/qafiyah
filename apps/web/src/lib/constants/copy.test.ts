import { describe, expect, it } from 'vitest';

import { formatPoemQuote, formatPoemText } from './copy';

describe('formatPoemQuote', () => {
  it('puts the plain poem link under the selected text after a blank line', () => {
    expect(formatPoemQuote('صدر البيت\nعجز البيت', 'https://qafiyah.com/poems/rHUD')).toBe(
      'صدر البيت\nعجز البيت\n\nhttps://qafiyah.com/poems/rHUD'
    );
  });

  it('keeps the blank line between the selected verses', () => {
    expect(
      formatPoemQuote('صدر الأول\nعجز الأول\n\nصدر الثاني', 'https://qafiyah.com/poems/rHUD')
    ).toBe('صدر الأول\nعجز الأول\n\nصدر الثاني\n\nhttps://qafiyah.com/poems/rHUD');
  });

  it('puts the poet under the selected text for sharing', () => {
    expect(formatPoemQuote('صدر البيت\nعجز البيت', 'شاعر')).toBe('صدر البيت\nعجز البيت\n\nشاعر');
  });

  it('trims the blank space a selection carries at its ends', () => {
    expect(formatPoemQuote('\n صدر البيت \n', 'https://qafiyah.com/poems/rHUD')).toBe(
      'صدر البيت\n\nhttps://qafiyah.com/poems/rHUD'
    );
  });
});

describe('formatPoemText', () => {
  it('puts each half on its own line, a blank line between verses, then the poet', () => {
    expect(
      formatPoemText(
        [
          ['أَيَا رَاكِبًا إِمَّا عَرَضْتَ فَبَلِّغَنْ', 'مُغَلْغَلَةً قَوْلَ امْرِئٍ نَاخِلِ الصَّدْرِ'],
          ['فَوَ اللهِ مَا فَارَقْتُكُمْ عَنْ كُشَاحَةٍ', 'وَلَا طِيْبِ نَفْسٍ عَنْكُمُ آخِرَ الدَّهْرِ'],
        ],
        'طرفة الجذيمي العبسي'
      )
    ).toBe(
      'أَيَا رَاكِبًا إِمَّا عَرَضْتَ فَبَلِّغَنْ\nمُغَلْغَلَةً قَوْلَ امْرِئٍ نَاخِلِ الصَّدْرِ\n\nفَوَ اللهِ مَا فَارَقْتُكُمْ عَنْ كُشَاحَةٍ\nوَلَا طِيْبِ نَفْسٍ عَنْكُمُ آخِرَ الدَّهْرِ\n\nطرفة الجذيمي العبسي'
    );
  });

  it('keeps a one-line verse on one line', () => {
    expect(formatPoemText([['سطر أول'], ['سطر ثان']], 'شاعر')).toBe('سطر أول\n\nسطر ثان\n\nشاعر');
  });

  it('gives the poet alone when there is no verse', () => {
    expect(formatPoemText([], 'طرفة الجذيمي العبسي')).toBe('طرفة الجذيمي العبسي');
  });
});
