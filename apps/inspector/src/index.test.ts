import { describe, expect, it } from 'bun:test';

import { buildReportForShapes } from './index';

import type { RouteShape } from './route-discovery';

describe('buildReportForShapes', () => {
  it('crawls, inspects, and renders an end-to-end report', async () => {
    const shapes: RouteShape[] = [
      { shape: 'poets', isDynamic: false, staticPath: '/poets', pattern: null },
      {
        shape: 'poets/[slug]',
        isDynamic: true,
        staticPath: null,
        pattern: /^\/poets\/[^/]+$/,
      },
    ];
    const pages: Record<string, string> = {
      '/poets': '<title>الشعراء</title><a href="/poets/al-mutanabbi">شاعر</a>',
      '/poets/al-mutanabbi': `<title>${'أ'.repeat(10)}</title>
<meta name="description" content="${'أ'.repeat(100)}" />
<link rel="canonical" href="https://qafiyah.com/poets/al-mutanabbi" />
<meta property="og:title" content="a" /><meta property="og:description" content="a" />
<meta property="og:url" content="https://qafiyah.com/poets/al-mutanabbi" />
<meta property="og:image" content="/open-graph.png" />
<meta property="og:type" content="profile" />
<meta name="twitter:card" content="summary" />
<meta name="twitter:title" content="a" /><meta name="twitter:description" content="a" />
<meta name="twitter:image" content="/organization-logo.png" />
<script type="application/ld+json">{"@type":"Person"}</script>
<h1>المتنبي</h1>`,
    };
    const fetchHtml = async (path: string): Promise<string> => pages[path] ?? '';
    const html = await buildReportForShapes(shapes, fetchHtml, 'http://localhost:4321');
    expect(html).toContain('poets/[slug]');
    expect(html).toContain('/poets/al-mutanabbi');
    expect(html).toContain('Page metadata');
    expect(html).toContain('src="http://localhost:4321/open-graph.png"');
    expect(html).toContain('field-ok');
  });

  it('surfaces unresolved shapes in the rendered report', async () => {
    const shapes: RouteShape[] = [
      { shape: 'poets', isDynamic: false, staticPath: '/poets', pattern: null },
      {
        shape: 'poems/[slug]',
        isDynamic: true,
        staticPath: null,
        pattern: /^\/poems\/[^/]+$/,
      },
    ];
    const fetchHtml = async (): Promise<string> => '<title>t</title>';
    const html = await buildReportForShapes(shapes, fetchHtml, 'http://localhost:4321');
    expect(html).toContain('no live example found');
  });

  it('orders sections to match the site nav, not crawl-discovery order', async () => {
    const shapes: RouteShape[] = [
      { shape: '404', isDynamic: false, staticPath: '/404', pattern: null },
      { shape: 'collections', isDynamic: false, staticPath: '/collections', pattern: null },
      { shape: '/', isDynamic: false, staticPath: '/', pattern: null },
      { shape: 'meters', isDynamic: false, staticPath: '/meters', pattern: null },
      { shape: 'poets', isDynamic: false, staticPath: '/poets', pattern: null },
      {
        shape: 'poets/[slug]',
        isDynamic: true,
        staticPath: null,
        pattern: /^\/poets\/[^/]+$/,
      },
      {
        shape: 'poems/[slug]',
        isDynamic: true,
        staticPath: null,
        pattern: /^\/poems\/[^/]+$/,
      },
    ];
    const pages: Record<string, string> = {
      '/poets': '<a href="/poets/al-mutanabbi">شاعر</a>',
      '/poets/al-mutanabbi': '<a href="/poems/qasida-1">قصيدة</a>',
      '/poems/qasida-1': '<h1>قصيدة</h1>',
    };
    const fetchHtml = async (path: string): Promise<string> => pages[path] ?? '';
    const html = await buildReportForShapes(shapes, fetchHtml, 'http://localhost:4321');
    const order = ['/', 'poets', 'poets/[slug]', 'poems/[slug]', 'meters', 'collections', '404'];
    const positions = order.map((shape) => html.indexOf(`<h2>${shape}`));
    expect(positions.every((position) => position !== -1)).toBe(true);
    expect(positions).toEqual([...positions].sort((a, b) => a - b));
  });

  it('places the about and developers pages after home and the login and account pages before 404', async () => {
    const shapes: RouteShape[] = [
      '404',
      'account',
      'login',
      'poets',
      'developers',
      'about',
      '/',
    ].map((shape) => ({
      shape,
      isDynamic: false,
      staticPath: shape === '/' ? '/' : `/${shape}`,
      pattern: null,
    }));
    const fetchHtml = async (): Promise<string> => '<title>t</title>';
    const html = await buildReportForShapes(shapes, fetchHtml, 'http://localhost:4321');
    const order = ['/', 'about', 'developers', 'poets', 'login', 'account', '404'];
    const positions = order.map((shape) => html.indexOf(`<h2>${shape}`));
    expect(positions.every((position) => position !== -1)).toBe(true);
    expect(positions).toEqual([...positions].sort((a, b) => a - b));
  });
});
