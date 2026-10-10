import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import { POEM_SCALE } from '@/lib/poem-scale';

import { PoemToolbar } from './poem-toolbar';

type Props = Parameters<typeof PoemToolbar>[0];

function render(overrides: Partial<Props>) {
  const props: Props = {
    fontScale: POEM_SCALE.font.initial,
    spacingScale: POEM_SCALE.spacing.initial,
    spacingMin: POEM_SCALE.spacing.min,
    showTashkeel: true,
    onFontScaleChange: () => {},
    onSpacingScaleChange: () => {},
    onToggleTashkeel: () => {},
    ...overrides,
  };
  return renderToStaticMarkup(<PoemToolbar {...props} />);
}

function button(html: string, label: string): string {
  return new RegExp(`<button[^>]*aria-label="${label}"[^>]*>`).exec(html)?.[0] ?? '';
}

describe('PoemToolbar', () => {
  it('keeps every button enabled at the default size and spacing', () => {
    const html = render({});
    expect(html).not.toContain('aria-disabled="true"');
    expect(html).not.toContain('disabled=""');
  });

  it('marks a font button unavailable at its limit but keeps it focusable', () => {
    const html = render({ fontScale: POEM_SCALE.font.max });
    const plus = button(html, 'تكبير خط القصيدة');
    expect(plus).toContain('aria-disabled="true"');
    expect(plus).not.toContain('disabled=""');
  });

  it('stops the tighter spacing button at the floor it is given', () => {
    const html = render({
      spacingScale: POEM_SCALE.spacing.halvesMin,
      spacingMin: POEM_SCALE.spacing.halvesMin,
    });
    expect(button(html, 'تضييق المسافة بين الأبيات')).toContain('aria-disabled="true"');
  });

  it('allows the last tighter step when the poem has a verse gap to shrink', () => {
    const html = render({ spacingScale: POEM_SCALE.spacing.halvesMin });
    expect(button(html, 'تضييق المسافة بين الأبيات')).toContain('aria-disabled="false"');
  });
});
