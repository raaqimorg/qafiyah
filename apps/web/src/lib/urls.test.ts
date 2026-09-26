import { describe, expect, it } from 'vitest';

import {
  dbDumpsUrl,
  githubUrl,
  poemInSectionUrl,
  poemUrl,
  poetAvatarUrl,
  poetsUrl,
  poetUrl,
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
    expect(poetUrl('mutanabbi', 1)).toBe('/poets/mutanabbi');
  });
  it('appends ?page=N beyond the first page', () => {
    expect(poetUrl('mutanabbi', 4)).toBe('/poets/mutanabbi?page=4');
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

describe('poemInSectionUrl', () => {
  it('marks the listing the poem was opened from', () => {
    expect(poemInSectionUrl('TnKK', 'themes')).toBe('/poems/TnKK?from=themes');
    expect(poemInSectionUrl('TnKK', 'collections')).toBe('/poems/TnKK?from=collections');
  });
  it('returns the bare poem URL without a listing', () => {
    expect(poemInSectionUrl('TnKK', undefined)).toBe('/poems/TnKK');
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
