import { describe, expect, it } from 'bun:test';

import { crawlForSamples, extractInternalHrefs, isUsableSample } from './site-crawl';

import type { RouteShape } from './route-discovery';

describe('extractInternalHrefs', () => {
  it('extracts root-relative hrefs', () => {
    const html = '<a href="/poets/al-mutanabbi">شاعر</a><a href="/meters/tawil">بحر</a>';
    expect(extractInternalHrefs(html)).toEqual(['/poets/al-mutanabbi', '/meters/tawil']);
  });

  it('strips query strings and hash fragments', () => {
    const html = '<a href="/poets?cursor=eyJhZnRlciI6MX0">a</a><a href="/poems/x#h=term">b</a>';
    expect(extractInternalHrefs(html)).toEqual(['/poets', '/poems/x']);
  });

  it('ignores external, mailto, tel, and anchor-only links', () => {
    const html =
      '<a href="https://x.com/qafiyahx">x</a>' +
      '<a href="mailto:mail@qafiyah.com">mail</a>' +
      '<a href="tel:+1000">tel</a>' +
      '<a href="#top">top</a>';
    expect(extractInternalHrefs(html)).toEqual([]);
  });

  it('accepts single or double quoted attributes', () => {
    const html = `<a href='/rhymes/ba'>r</a>`;
    expect(extractInternalHrefs(html)).toEqual(['/rhymes/ba']);
  });
});

const staticShape = (shape: string): RouteShape => ({
  shape,
  isDynamic: false,
  staticPath: `/${shape === '/' ? '' : shape}`,
  pattern: null,
});

const dynamicShape = (shape: string, pattern: RegExp): RouteShape => ({
  shape,
  isDynamic: true,
  staticPath: null,
  pattern,
});

describe('isUsableSample', () => {
  it('rejects the unknown-entity placeholder, whose opaque slug looks like any other', () => {
    expect(isUsableSample('<h1 class="x">غير معروف</h1>', '/poets/JJHE')).toBe(false);
  });

  it('accepts a real record', () => {
    expect(isUsableSample('<h1>أبو تمام</h1>', '/poets/GuRy')).toBe(true);
  });

  it('rejects a page whose canonical points elsewhere, which is how a 404 rewrite looks', () => {
    const notFound = '<link rel="canonical" href="https://qafiyah.com/" /><h1>٤٠٤</h1>';
    expect(isUsableSample(notFound, '/poets/ZZZZ')).toBe(false);
  });

  it('accepts a page whose canonical matches the fetched path', () => {
    const page = '<link rel="canonical" href="https://qafiyah.com/poets/GuRy" /><h1>أبو تمام</h1>';
    expect(isUsableSample(page, '/poets/GuRy')).toBe(true);
  });
});

describe('crawlForSamples', () => {
  it('prefers a pinned sample over whatever the crawl reaches first', async () => {
    const shapes = [staticShape('poets'), dynamicShape('poets/[slug]', /^\/poets\/[^/]+$/)];
    const pages: Record<string, string> = {
      '/poets': '<a href="/poets/JJHE">غير معروف</a>',
      '/poets/JJHE': '<h1>غير معروف</h1>',
      '/poets/GuRy': '<h1>أبو تمام</h1>',
    };
    const fetchHtml = async (path: string): Promise<string> => pages[path] ?? '';
    const result = await crawlForSamples(shapes, fetchHtml, { 'poets/[slug]': '/poets/GuRy' });
    const poet = result.resolved.find((page) => page.shape === 'poets/[slug]');
    expect(poet?.path).toBe('/poets/GuRy');
  });

  it('skips a placeholder record and keeps crawling for a real one', async () => {
    const shapes = [staticShape('poets'), dynamicShape('poets/[slug]', /^\/poets\/[^/]+$/)];
    const pages: Record<string, string> = {
      '/poets': '<a href="/poets/JJHE">غير معروف</a><a href="/poets/GuRy">أبو تمام</a>',
      '/poets/JJHE': '<h1>غير معروف</h1>',
      '/poets/GuRy': '<h1>أبو تمام</h1>',
    };
    const fetchHtml = async (path: string): Promise<string> => pages[path] ?? '';
    const result = await crawlForSamples(shapes, fetchHtml);
    const poet = result.resolved.find((page) => page.shape === 'poets/[slug]');
    expect(poet?.path).toBe('/poets/GuRy');
  });

  it('fetches static shapes directly', async () => {
    const shapes = [staticShape('/')];
    const fetchHtml = async (path: string): Promise<string> => `html for ${path}`;
    const result = await crawlForSamples(shapes, fetchHtml);
    expect(result.resolved).toEqual([{ shape: '/', path: '/', html: 'html for /' }]);
    expect(result.unresolved).toEqual([]);
  });

  it('resolves a dynamic shape by following a link found on a static page', async () => {
    const shapes = [staticShape('poets'), dynamicShape('poets/[slug]', /^\/poets\/[^/]+$/)];
    const pages: Record<string, string> = {
      '/poets': '<a href="/poets/al-mutanabbi">شاعر</a>',
      '/poets/al-mutanabbi': '<h1>المتنبي</h1>',
    };
    const fetchHtml = async (path: string): Promise<string> => pages[path] ?? '';
    const result = await crawlForSamples(shapes, fetchHtml);
    expect(result.resolved).toEqual([
      { shape: 'poets', path: '/poets', html: pages['/poets'] ?? '' },
      {
        shape: 'poets/[slug]',
        path: '/poets/al-mutanabbi',
        html: pages['/poets/al-mutanabbi'] ?? '',
      },
    ]);
    expect(result.unresolved).toEqual([]);
  });

  it('resolves a shape discovered transitively through another dynamic page', async () => {
    const shapes = [
      staticShape('poets'),
      dynamicShape('poets/[slug]', /^\/poets\/[^/]+$/),
      dynamicShape('poems/[slug]', /^\/poems\/[^/]+$/),
    ];
    const pages: Record<string, string> = {
      '/poets': '<a href="/poets/al-mutanabbi">شاعر</a>',
      '/poets/al-mutanabbi': '<a href="/poems/qasida-1">قصيدة</a>',
      '/poems/qasida-1': '<h1>قصيدة</h1>',
    };
    const fetchHtml = async (path: string): Promise<string> => pages[path] ?? '';
    const result = await crawlForSamples(shapes, fetchHtml);
    expect(result.resolved.map((page) => page.shape)).toEqual([
      'poets',
      'poets/[slug]',
      'poems/[slug]',
    ]);
    expect(result.unresolved).toEqual([]);
  });

  it('flags a dynamic shape as unresolved when no live link is found', async () => {
    const shapes = [staticShape('poets'), dynamicShape('poems/[slug]', /^\/poems\/[^/]+$/)];
    const fetchHtml = async (): Promise<string> => '<p>no links here</p>';
    const result = await crawlForSamples(shapes, fetchHtml);
    expect(result.unresolved).toEqual(['poems/[slug]']);
  });

  it('does not requeue an href once its shape is already resolved', async () => {
    const shapes = [staticShape('poets'), dynamicShape('poets/[slug]', /^\/poets\/[^/]+$/)];
    let fetchCount = 0;
    const pages: Record<string, string> = {
      '/poets': '<a href="/poets/a">a</a><a href="/poets/b">b</a><a href="/poets/c">c</a>',
      '/poets/a': '<h1>a</h1>',
    };
    const fetchHtml = async (path: string): Promise<string> => {
      fetchCount += 1;
      return pages[path] ?? '';
    };
    const result = await crawlForSamples(shapes, fetchHtml);
    expect(fetchCount).toBe(2);
    expect(result.resolved.map((page) => page.path)).toEqual(['/poets', '/poets/a']);
  });

  it('stops fetching redundant candidates for a shape once another shape is still pending', async () => {
    const shapes = [
      staticShape('meters'),
      dynamicShape('meters/[slug]', /^\/meters\/[^/]+$/),
      dynamicShape('poems/[slug]', /^\/poems\/[^/]+$/),
    ];
    let fetchCount = 0;
    const pages: Record<string, string> = {
      '/meters': '<a href="/meters/a">a</a><a href="/meters/b">b</a><a href="/meters/c">c</a>',
      '/meters/a': '<h1>a</h1>',
      '/meters/b': '<h1>b</h1>',
      '/meters/c': '<h1>c</h1>',
    };
    const fetchHtml = async (path: string): Promise<string> => {
      fetchCount += 1;
      return pages[path] ?? '';
    };
    const result = await crawlForSamples(shapes, fetchHtml);
    expect(fetchCount).toBe(2);
    expect(result.resolved.map((page) => page.shape)).toEqual(['meters', 'meters/[slug]']);
    expect(result.unresolved).toEqual(['poems/[slug]']);
  });

  it('skips the "ghayrmaruf" unknown-entity sentinel in favor of a real example (regression: it often has the most poems, sorting first)', async () => {
    const shapes = [staticShape('themes'), dynamicShape('themes/[slug]', /^\/themes\/[^/]+$/)];
    const pages: Record<string, string> = {
      '/themes': '<a href="/themes/ghayrmaruf">غير معروف</a><a href="/themes/alhikma">الحكمة</a>',
      '/themes/alhikma': '<h1>الحكمة</h1>',
    };
    const fetchHtml = async (path: string): Promise<string> => pages[path] ?? '';
    const result = await crawlForSamples(shapes, fetchHtml);
    expect(result.resolved.map((page) => page.path)).toEqual(['/themes', '/themes/alhikma']);
    expect(result.unresolved).toEqual([]);
  });

  it('leaves a shape unresolved rather than resolving it to the sentinel alone', async () => {
    const shapes = [staticShape('themes'), dynamicShape('themes/[slug]', /^\/themes\/[^/]+$/)];
    const pages: Record<string, string> = {
      '/themes': '<a href="/themes/ghayrmaruf">غير معروف</a>',
    };
    const fetchHtml = async (path: string): Promise<string> => pages[path] ?? '';
    const result = await crawlForSamples(shapes, fetchHtml);
    expect(result.unresolved).toEqual(['themes/[slug]']);
  });

  it('does not let a shape with many redundant candidates exhaust the fetch budget for another shape', async () => {
    const manyMeterLinks = Array.from(
      { length: 40 },
      (_, i) => `<a href="/meters/${i}">m</a>`
    ).join('');
    const shapes = [
      staticShape('meters'),
      staticShape('poets'),
      dynamicShape('meters/[slug]', /^\/meters\/[^/]+$/),
      dynamicShape('poets/[slug]', /^\/poets\/[^/]+$/),
    ];
    const pages: Record<string, string> = {
      '/meters': manyMeterLinks,
      '/poets': '<a href="/poets/al-mutanabbi">شاعر</a>',
      '/poets/al-mutanabbi': '<h1>المتنبي</h1>',
    };
    for (let i = 0; i < 40; i += 1) {
      pages[`/meters/${i}`] = `<h1>${i}</h1>`;
    }
    let fetchCount = 0;
    const fetchHtml = async (path: string): Promise<string> => {
      fetchCount += 1;
      return pages[path] ?? '';
    };
    const result = await crawlForSamples(shapes, fetchHtml);
    expect(fetchCount).toBeLessThan(10);
    expect(result.resolved.map((page) => page.shape)).toContain('poets/[slug]');
    expect(result.unresolved).toEqual([]);
  });
});

describe('crawlForSamples fetch coverage', () => {
  it('fetches every static shape even after the last dynamic shape resolves', async () => {
    const shapes = [staticShape('/'), staticShape('poets'), staticShape('meters')];
    const fetched: string[] = [];
    const result = await crawlForSamples(shapes, async (path) => {
      fetched.push(path);
      return '<html></html>';
    });
    expect(fetched).toEqual(['/', '/poets', '/meters']);
    expect(result.resolved.map((page) => page.shape)).toEqual(['/', 'poets', 'meters']);
  });

  it('keeps fetching queued static shapes after a preferred sample resolves the only dynamic shape', async () => {
    const shapes = [dynamicShape('poets/[slug]', /^\/poets\/[^/]+$/), staticShape('poets')];
    const fetched: string[] = [];
    const result = await crawlForSamples(
      shapes,
      async (path) => {
        fetched.push(path);
        return '<html></html>';
      },
      { 'poets/[slug]': '/poets/GuRy' }
    );
    expect(fetched).toEqual(['/poets/GuRy', '/poets']);
    expect(result.unresolved).toEqual([]);
  });

  it('stops at the fetch budget exactly', async () => {
    const shapes = Array.from({ length: 31 }, (_, i) => staticShape(`s${i}`));
    let fetched = 0;
    await crawlForSamples(shapes, async () => {
      fetched += 1;
      return '<html></html>';
    });
    expect(fetched).toBe(30);
  });
});
