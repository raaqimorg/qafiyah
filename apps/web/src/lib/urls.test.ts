import { describe, expect, it } from 'vitest';

import { serializePoetSearch } from './poet-search-params';
import {
  dbDumpsUrl,
  githubUrl,
  poemUrl,
  poetAvatarUrl,
  poetsUrl,
  isCanonical,
  poetUrl,
  retryUrl,
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
    expect(taxonomyUrl('meters', 'tawil', 1)).toBe('/meters/tawil');
  });
  it('appends ?page=N for pages beyond the first', () => {
    expect(taxonomyUrl('rhymes', 'meem', 3)).toBe('/rhymes/meem?page=3');
  });
});

describe('poetsUrl', () => {
  it('returns bare /poets with no options', () => {
    expect(poetsUrl()).toBe('/poets');
    expect(poetsUrl({ page: 1 })).toBe('/poets');
  });
  it('appends ?page=N beyond the first page', () => {
    expect(poetsUrl({ page: 2 })).toBe('/poets?page=2');
  });
  it('adds era and page, omitting empties, in era→q→page order', () => {
    expect(poetsUrl({ era: 'jahili' })).toBe('/poets?era=jahili');
    expect(poetsUrl({ era: 'jahili', page: 3 })).toBe('/poets?era=jahili&page=3');
  });
  it('encodes an Arabic q and round-trips it', () => {
    const url = poetsUrl({ q: 'متنبي' });
    expect(url.startsWith('/poets?')).toBe(true);
    expect(new URL(url, 'http://x').searchParams.get('q')).toBe('متنبي');
  });
});

describe('poetUrl', () => {
  it('builds the poet URL and omits page 1', () => {
    expect(poetUrl('mutanabbi')).toBe('/poets/mutanabbi');
    expect(poetUrl('mutanabbi', { page: 1 })).toBe('/poets/mutanabbi');
  });
  it('appends ?page=N beyond the first page', () => {
    expect(poetUrl('mutanabbi', { page: 4 })).toBe('/poets/mutanabbi?page=4');
  });
  it('repeats each poem filter per value, sorted, in meter, rhyme, theme, page order', () => {
    expect(
      poetUrl('mutanabbi', { theme: ['alnasib'], meter: ['altawil', 'alkamil'], page: 2 })
    ).toBe('/poets/mutanabbi?meter=alkamil&meter=altawil&theme=alnasib&page=2');
    expect(poetUrl('mutanabbi', { meter: [], rhyme: [], theme: [] })).toBe('/poets/mutanabbi');
  });
  it('gives every selection one URL, whatever the order or repeats of its values', () => {
    expect(poetUrl('mutanabbi', { meter: ['altawil', 'alkamil', 'altawil'] })).toBe(
      poetUrl('mutanabbi', { meter: ['alkamil', 'altawil'] })
    );
  });
  it('writes the same query the poet page list writes to the address bar', () => {
    const states = [
      { meter: [], rhyme: [], theme: [], page: 1 },
      { meter: [], rhyme: [], theme: [], page: 3 },
      { meter: ['altawil'], rhyme: [], theme: [], page: 1 },
      { meter: ['alkamil', 'altawil'], rhyme: ['alef-maqsura'], theme: ['alnasib'], page: 2 },
      { meter: [], rhyme: ['meem', 'noon'], theme: ['alhikma'], page: 5 },
    ];
    for (const state of states) {
      expect(poetUrl('mutanabbi', state)).toBe(`/poets/mutanabbi${serializePoetSearch(state)}`);
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
      isCanonical(new URL('https://qafiyah.com/meters/altawil?page=2'), '/meters/altawil?page=2')
    ).toBe(true);
    expect(isCanonical(new URL('https://qafiyah.com/meters/altawil'), '/meters/altawil')).toBe(
      true
    );
  });
  it('refuses an extra parameter, a padded number, or another parameter order', () => {
    const canonical = '/poets/oNbs?meter=altawil&page=2';
    for (const href of [
      'https://qafiyah.com/poets/oNbs?meter=altawil&page=2&utm_source=x',
      'https://qafiyah.com/poets/oNbs?meter=altawil&page=02',
      'https://qafiyah.com/poets/oNbs?page=2&meter=altawil',
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
    expect(retryUrl('/poets', new URL('https://qafiyah.com/poets?page=2'))).toBe('/poets?page=2');
  });
});

describe('go builders', () => {
  it('point at the versioned go endpoints on the API host', () => {
    expect(xProfileUrl()).toBe('https://api.qafiyah.com/v1/go/x');
    expect(githubUrl()).toBe('https://api.qafiyah.com/v1/go/github');
    expect(dbDumpsUrl()).toBe('https://api.qafiyah.com/v1/go/db');
  });
});

describe('poetAvatarUrl', () => {
  it('builds the CDN avatar url from the slug', () => {
    expect(poetAvatarUrl('CCMr')).toBe('https://cdn.qafiyah.com/poets/CCMr/avatar.webp');
  });
});
