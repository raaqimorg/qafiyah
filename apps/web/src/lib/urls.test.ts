import { describe, expect, it } from 'vitest';

import { serializePoetSearch } from './poet-search-params';
import {
  feelingUrl,
  githubUrl,
  poemUrl,
  poetAvatarUrl,
  poetsUrl,
  isCanonical,
  poetUrl,
  retryUrl,
  shareIntent,
  taxonomyIndexUrl,
  taxonomyUrl,
  xProfileUrl,
} from './urls';

describe('taxonomyIndexUrl', () => {
  it('returns the bare section URL', () => {
    expect(taxonomyIndexUrl('meters')).toBe('/meters');
    expect(taxonomyIndexUrl('collections')).toBe('/collections');
  });
});

describe('taxonomyUrl', () => {
  it('returns the bare term URL for the first page', () => {
    expect(taxonomyUrl('meters', 'tawil')).toBe('/meters/tawil');
    expect(taxonomyUrl('meters', 'tawil', '')).toBe('/meters/tawil');
  });
  it('appends ?cursor= for any other page', () => {
    expect(taxonomyUrl('rhymes', 'meem', 'eyJhZnRlciI6MzB9')).toBe(
      '/rhymes/meem?cursor=eyJhZnRlciI6MzB9'
    );
  });
  it('percent-encodes a slug with non-ASCII letters, as the request URL does', () => {
    const url = new URL('https://qafiyah.com/meters/الطويل?cursor=eyJhZnRlciI6MzB9');
    expect(taxonomyUrl('meters', 'الطويل', 'eyJhZnRlciI6MzB9')).toBe(
      `${url.pathname}${url.search}`
    );
  });
});

describe('poetsUrl', () => {
  it('returns bare /poets with no options', () => {
    expect(poetsUrl()).toBe('/poets');
    expect(poetsUrl({ cursor: '' })).toBe('/poets');
  });
  it('appends ?cursor= for any other page', () => {
    expect(poetsUrl({ cursor: 'eyJhZnRlciI6MzB9' })).toBe('/poets?cursor=eyJhZnRlciI6MzB9');
  });
  it('adds era and cursor, omitting empties, in era→q→cursor order', () => {
    expect(poetsUrl({ era: 'jahili' })).toBe('/poets?era=jahili');
    expect(poetsUrl({ era: 'jahili', cursor: 'c' })).toBe('/poets?era=jahili&cursor=c');
  });
  it('encodes an Arabic q and round-trips it', () => {
    const url = poetsUrl({ q: 'متنبي' });
    expect(url.startsWith('/poets?')).toBe(true);
    expect(new URL(url, 'http://x').searchParams.get('q')).toBe('متنبي');
  });
});

describe('poetUrl', () => {
  it('builds the poet URL and omits an empty cursor', () => {
    expect(poetUrl('mutanabbi')).toBe('/poets/mutanabbi');
    expect(poetUrl('mutanabbi', { cursor: '' })).toBe('/poets/mutanabbi');
  });
  it('appends ?cursor= for any other page', () => {
    expect(poetUrl('mutanabbi', { cursor: 'c' })).toBe('/poets/mutanabbi?cursor=c');
  });
  it('repeats each poem filter per value, sorted, in meter, rhyme, theme, cursor order', () => {
    expect(
      poetUrl('mutanabbi', { theme: ['alnasib'], meter: ['altawil', 'alkamil'], cursor: 'c' })
    ).toBe('/poets/mutanabbi?meter=alkamil&meter=altawil&theme=alnasib&cursor=c');
    expect(poetUrl('mutanabbi', { meter: [], rhyme: [], theme: [] })).toBe('/poets/mutanabbi');
  });
  it('gives every selection one URL, whatever the order or repeats of its values', () => {
    expect(poetUrl('mutanabbi', { meter: ['altawil', 'alkamil', 'altawil'] })).toBe(
      poetUrl('mutanabbi', { meter: ['alkamil', 'altawil'] })
    );
  });
  it('writes the same query the poet page list writes to the address bar', () => {
    const states = [
      { meter: [], rhyme: [], theme: [], cursor: null },
      { meter: [], rhyme: [], theme: [], cursor: 'eyJhZnRlciI6MzB9' },
      { meter: ['altawil'], rhyme: [], theme: [], cursor: null },
      { meter: ['alkamil', 'altawil'], rhyme: ['alef-maqsura'], theme: ['alnasib'], cursor: 'c' },
      { meter: [], rhyme: ['meem', 'noon'], theme: ['alhikma'], cursor: 'eyJiZWZvcmUiOjMxfQ' },
    ];
    for (const state of states) {
      expect(poetUrl('mutanabbi', { ...state, cursor: state.cursor ?? undefined })).toBe(
        `/poets/mutanabbi${serializePoetSearch(state)}`
      );
    }
  });
  it('encodes a slug the way the request URL encodes its path, so a canonical slug never redirects', () => {
    for (const path of ['/poets/a,b', '/poets/a%2Cb', '/poets/a;b:c@d=e+f$g&h', '/poets/a%20b']) {
      const url = new URL(`https://qafiyah.com${path}`);
      const slug = decodeURI(url.pathname.slice('/poets/'.length));
      expect(poetUrl(slug)).toBe(url.pathname);
    }
  });
  it('percent-encodes a slug with non-ASCII letters, as the request URL does', () => {
    for (const slug of ['الفرزدق', 'imru-alqays؟']) {
      const url = new URL(`https://qafiyah.com/poets/${slug}`);
      expect(poetUrl(slug)).toBe(url.pathname);
      expect(isCanonical(url, poetUrl(slug))).toBe(true);
    }
  });
});

describe('poemUrl', () => {
  it('returns the poem URL', () => {
    expect(poemUrl('TnKK')).toBe('/poems/TnKK');
    expect(poemUrl('TnKK', [])).toBe('/poems/TnKK');
    expect(poemUrl('TnKK', ['  '])).toBe('/poems/TnKK');
  });
  it('appends comma-joined encoded highlight terms after #h=', () => {
    const url = poemUrl('TnKK', ['يا ليت', 'هذا']);
    expect(url.startsWith('/poems/TnKK#h=')).toBe(true);
    const encoded = url.slice('/poems/TnKK#h='.length);
    expect(encoded.split(',').map((part) => decodeURIComponent(part))).toEqual(['يا ليت', 'هذا']);
  });
});

describe('isCanonical', () => {
  it('accepts the canonical URL exactly', () => {
    expect(
      isCanonical(
        new URL('https://qafiyah.com/meters/altawil?cursor=c'),
        '/meters/altawil?cursor=c'
      )
    ).toBe(true);
    expect(isCanonical(new URL('https://qafiyah.com/meters/altawil'), '/meters/altawil')).toBe(
      true
    );
  });
  it('refuses an extra parameter, an old page number, or another parameter order', () => {
    const canonical = '/poets/oNbs?meter=altawil&cursor=c';
    for (const href of [
      'https://qafiyah.com/poets/oNbs?meter=altawil&cursor=c&utm_source=x',
      'https://qafiyah.com/poets/oNbs?meter=altawil&cursor=c&page=2',
      'https://qafiyah.com/poets/oNbs?cursor=c&meter=altawil',
    ]) {
      expect(isCanonical(new URL(href), canonical)).toBe(false);
    }
  });
});

describe('retryUrl', () => {
  it('points at the original path when the error page is a rewrite of another route', () => {
    expect(retryUrl('/poems/random', new URL('https://qafiyah.com/500'))).toBe('/poems/random');
  });
  it('keeps the query string of the failed request', () => {
    expect(retryUrl('/poets', new URL('https://qafiyah.com/poets?cursor=c'))).toBe(
      '/poets?cursor=c'
    );
  });
});

describe('go builders', () => {
  it('point at the versioned go endpoints on the API host', () => {
    expect(xProfileUrl()).toBe('https://api.qafiyah.com/v1/go/x');
    expect(githubUrl()).toBe('https://api.qafiyah.com/v1/go/github');
  });
});

describe('poetAvatarUrl', () => {
  it('builds the CDN avatar url from the slug', () => {
    expect(poetAvatarUrl('CCMr')).toBe('https://cdn.qafiyah.com/poets/CCMr/avatar.webp');
  });
});

describe('shareIntent', () => {
  const payload = {
    body: 'عنوان القصيدة\nبيت & آخر؟',
    url: 'https://qafiyah.com/poems/TnKK',
  };

  it.each([
    ['x', 'https://x.com/intent/tweet', { text: `${payload.body}\n${payload.url}` }],
    ['whatsapp', 'https://wa.me/', { text: `${payload.body}\n${payload.url}` }],
    ['telegram', 'https://t.me/share/url', { url: payload.url, text: payload.body }],
    ['facebook', 'https://www.facebook.com/sharer/sharer.php', { u: payload.url }],
  ] as const)('encodes the supported content for %s', (network, endpoint, parameters) => {
    const intent = new URL(shareIntent(network, payload));
    expect(`${intent.origin}${intent.pathname}`).toBe(endpoint);
    expect(Object.fromEntries(intent.searchParams)).toEqual(parameters);
  });
});

describe('feelingUrl', () => {
  it('builds the wheel, a feeling, and a later page', () => {
    expect(feelingUrl()).toBe('/feelings');
    expect(feelingUrl('alhuzn')).toBe('/feelings/alhuzn');
    expect(feelingUrl('alhuzn', 'c')).toBe('/feelings/alhuzn?cursor=c');
  });
});
