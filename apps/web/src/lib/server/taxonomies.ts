import { apiServer } from './client';
import { getOrNull, unwrap } from './unwrap';

import type { Ok } from './types';
import type { MeterSlug, RhymeSlug, ThemeSlug } from '@/lib/api/brands';

export type Meter = Ok<'/meters/{slug}'>['data'];
export type Rhyme = Ok<'/rhymes/{slug}'>['data'];
export type Theme = Ok<'/themes/{slug}'>['data'];

export const allEras = (): Promise<Ok<'/eras'>['data']> => unwrap(() => apiServer.GET('/eras'));
export const allMeters = (): Promise<Ok<'/meters'>['data']> =>
  unwrap(() => apiServer.GET('/meters'));
export const allRhymes = (): Promise<Ok<'/rhymes'>['data']> =>
  unwrap(() => apiServer.GET('/rhymes'));
export const allThemes = (): Promise<Ok<'/themes'>['data']> =>
  unwrap(() => apiServer.GET('/themes'));
export const allPoemTypes = (): Promise<Ok<'/poem-types'>['data']> =>
  unwrap(() => apiServer.GET('/poem-types'));

export const getMeter = (slug: MeterSlug): Promise<Meter | null> =>
  getOrNull(() => apiServer.GET('/meters/{slug}', { params: { path: { slug } } }));
export const getRhyme = (slug: RhymeSlug): Promise<Rhyme | null> =>
  getOrNull(() => apiServer.GET('/rhymes/{slug}', { params: { path: { slug } } }));
export const getTheme = (slug: ThemeSlug): Promise<Theme | null> =>
  getOrNull(() => apiServer.GET('/themes/{slug}', { params: { path: { slug } } }));
