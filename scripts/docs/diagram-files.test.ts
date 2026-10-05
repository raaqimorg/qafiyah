import { describe, expect, it } from 'bun:test';

import { GENERATED_HEADER, generatedName, stale, withHeader } from './diagram-files';

describe('generatedName', () => {
  it('names an exported view by its key with the generated suffix', () => {
    expect(generatedName('structurizr-context.svg')).toBe('context.gen.svg');
    expect(generatedName('structurizr-api-components.svg')).toBe('api-components.gen.svg');
  });

  it('ignores files that are not an exported view', () => {
    expect(generatedName('structurizr-context.puml')).toBeUndefined();
    expect(generatedName('legend.svg')).toBeUndefined();
    expect(generatedName('structurizr-.svg')).toBeUndefined();
  });
});

describe('withHeader', () => {
  it('puts the header right after the XML declaration', () => {
    const svg = '<?xml version="1.0" encoding="us-ascii" standalone="no"?><svg></svg>';
    expect(withHeader(svg)).toBe(
      `<?xml version="1.0" encoding="us-ascii" standalone="no"?>\n${GENERATED_HEADER}\n<svg></svg>\n`
    );
  });

  it('puts the header first when there is no declaration', () => {
    expect(withHeader('<svg></svg>')).toBe(`${GENERATED_HEADER}\n<svg></svg>\n`);
  });

  it('ends with exactly one newline', () => {
    expect(withHeader('<svg></svg>\n')).toBe(`${GENERATED_HEADER}\n<svg></svg>\n`);
  });
});

describe('stale', () => {
  it('lists the files no view produces any more', () => {
    expect(stale(['context.gen.svg', 'old.gen.svg'], ['context.gen.svg'])).toEqual(['old.gen.svg']);
  });

  it('lists nothing when every file is produced', () => {
    expect(stale(['context.gen.svg'], ['context.gen.svg', 'search.gen.svg'])).toEqual([]);
  });
});
