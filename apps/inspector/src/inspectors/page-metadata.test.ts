import { describe, expect, it } from 'bun:test';

import { inspectPageMetadata, pageMetadataInspector } from './page-metadata';

const POEM_SHAPE = 'poems/[slug]';
const POET_SHAPE = 'poets/[slug]';
const ERROR_SHAPE = '404';

const GOOD_HTML = `<html><head>
<title>قصيدة قصيرة - شاعر</title>
<meta name="description" content="${'x'.repeat(120)}" />
<link rel="canonical" href="https://qafiyah.com/poems/x" />
<link rel="alternate" hreflang="ar" href="https://qafiyah.com/poems/x" />
<meta property="og:title" content="قصيدة قصيرة" />
<meta property="og:description" content="وصف" />
<meta property="og:url" content="https://qafiyah.com/poems/x" />
<meta property="og:image" content="https://qafiyah.com/open-graph.png" />
<meta property="og:type" content="article" />
<meta name="twitter:card" content="summary" />
<meta name="twitter:title" content="قصيدة قصيرة" />
<meta name="twitter:description" content="وصف" />
<meta name="twitter:image" content="https://qafiyah.com/organization-logo.png" />
<script type="application/ld+json">{"@context":"https://schema.org","@type":"CreativeWork"}</script>
</head><body><h1>قصيدة قصيرة</h1></body></html>`;

function fieldFor(report: ReturnType<typeof inspectPageMetadata>, label: string) {
  return report.fields.find((field) => field.label === label);
}

describe('inspectPageMetadata', () => {
  it('passes every field on well-formed metadata', () => {
    const report = inspectPageMetadata(GOOD_HTML, POEM_SHAPE);
    expect(report.fields.every((field) => field.ok)).toBe(true);
    expect(fieldFor(report, 'og:image')?.value).toBe('https://qafiyah.com/open-graph.png');
  });

  it('fails title when missing or too long', () => {
    expect(fieldFor(inspectPageMetadata('<html></html>', POEM_SHAPE), 'title')?.ok).toBe(false);
    const longTitle = `<title>${'a'.repeat(81)}</title>`;
    expect(fieldFor(inspectPageMetadata(longTitle, POEM_SHAPE), 'title')?.ok).toBe(false);
  });

  it('fails description outside the 60-320 char window', () => {
    const shortDesc = '<meta name="description" content="too short" />';
    expect(fieldFor(inspectPageMetadata(shortDesc, POEM_SHAPE), 'description')?.ok).toBe(false);
  });

  it('fails twitter:card on any page still serving the large card', () => {
    const html = '<meta name="twitter:card" content="summary_large_image" />';
    expect(fieldFor(inspectPageMetadata(html, POEM_SHAPE), 'twitter:card')?.ok).toBe(false);
    expect(fieldFor(inspectPageMetadata(html, POET_SHAPE), 'twitter:card')?.ok).toBe(false);
  });

  it('fails hreflang when it does not point at the canonical', () => {
    const html =
      '<link rel="canonical" href="https://qafiyah.com/a" />' +
      '<link rel="alternate" hreflang="ar" href="https://qafiyah.com/b" />';
    expect(fieldFor(inspectPageMetadata(html, POEM_SHAPE), 'hreflang')?.ok).toBe(false);
  });

  it('fails hreflang when it is absent', () => {
    const html = '<link rel="canonical" href="https://qafiyah.com/a" />';
    expect(fieldFor(inspectPageMetadata(html, POEM_SHAPE), 'hreflang')?.value).toBe('(missing)');
  });

  it('fails h1 count when not exactly one', () => {
    expect(fieldFor(inspectPageMetadata('<body></body>', POEM_SHAPE), 'h1 count')?.ok).toBe(false);
    const two = '<h1>a</h1><h1>b</h1>';
    const report = inspectPageMetadata(two, POEM_SHAPE);
    expect(fieldFor(report, 'h1 count')?.value).toBe('2');
    expect(fieldFor(report, 'h1 count')?.ok).toBe(false);
  });

  it('fails json-ld when a block does not parse', () => {
    const html = '<script type="application/ld+json">{not json}</script>';
    const report = inspectPageMetadata(html, POEM_SHAPE);
    expect(fieldFor(report, 'json-ld[0]')?.ok).toBe(false);
  });

  it('reports a missing json-ld block as a single failing field', () => {
    const report = inspectPageMetadata('<html></html>', POEM_SHAPE);
    expect(fieldFor(report, 'json-ld')?.ok).toBe(false);
  });

  it('on an error page, only checks the title and skips every other field', () => {
    const report = inspectPageMetadata('<title>لم يتم العثور على الصفحة</title>', ERROR_SHAPE);
    expect(report.fields).toHaveLength(1);
    expect(report.fields[0]?.label).toBe('title');
    expect(report.fields[0]?.ok).toBe(true);
  });

  it('exposes an Inspector-conformant wrapper for the registry', () => {
    const fields = pageMetadataInspector.inspect({
      shape: POEM_SHAPE,
      path: '/poems/x',
      html: GOOD_HTML,
    });
    expect(fields).toEqual(inspectPageMetadata(GOOD_HTML, POEM_SHAPE).fields);
    expect(pageMetadataInspector.id).toBe('page-metadata');
  });
});

describe('inspectPageMetadata edge values', () => {
  const ok = (html: string, label: string): boolean | undefined =>
    inspectPageMetadata(html, '/').fields.find((f) => f.label === label)?.ok;
  const page = (title: string, desc: string): string =>
    `<html><head><title>${title}</title><meta name="description" content="${desc}"></head></html>`;

  it('rejects an empty title', () => {
    expect(ok('<html><head><title></title></head></html>', 'title')).toBe(false);
  });

  it('rejects a missing hreflang even when the canonical is also missing', () => {
    expect(ok('<html><head><title>t</title></head></html>', 'hreflang')).toBe(false);
  });

  it('accepts the exact thresholds and rejects one past them', () => {
    expect(ok(page('t'.repeat(80), 'd'.repeat(60)), 'title')).toBe(true);
    expect(ok(page('t'.repeat(80), 'd'.repeat(60)), 'description')).toBe(true);
    expect(ok(page('t'.repeat(81), 'd'.repeat(321)), 'title')).toBe(false);
    expect(ok(page('t'.repeat(81), 'd'.repeat(321)), 'description')).toBe(false);
    expect(ok(page('t', 'd'.repeat(59)), 'description')).toBe(false);
    expect(ok(page('t', 'd'.repeat(320)), 'description')).toBe(true);
  });
});
