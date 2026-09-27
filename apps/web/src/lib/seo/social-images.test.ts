import { describe, expect, it } from 'vitest';

import { DEFAULT_SOCIAL_IMAGE, LOGO_SOCIAL_IMAGE } from '@/lib/constants/site-meta';

import { poetAvatarImage, resolveSocialImages } from './social-images';

const poet = { name: 'مارية الرفاعي', slug: 'ZUqh' };

describe('poetAvatarImage', () => {
  it('points at the CDN avatar, named after the poet, when the poet has one', () => {
    expect(poetAvatarImage({ ...poet, hasAvatar: true })).toEqual({
      url: 'https://cdn.qafiyah.com/poets/ZUqh/avatar.webp',
      alt: 'مارية الرفاعي',
      mimeType: 'image/webp',
    });
  });

  it('omits dimensions so scrapers measure the varying avatar sizes', () => {
    const image = poetAvatarImage({ ...poet, hasAvatar: true });
    expect(image?.width).toBeUndefined();
    expect(image?.height).toBeUndefined();
  });

  it('gives no image when the poet has no avatar', () => {
    expect(poetAvatarImage({ ...poet, hasAvatar: false })).toBeUndefined();
  });
});

describe('resolveSocialImages', () => {
  it('shows the open graph banner outside twitter and the square logo on twitter by default', () => {
    expect(resolveSocialImages(undefined)).toEqual({
      openGraph: DEFAULT_SOCIAL_IMAGE,
      twitter: LOGO_SOCIAL_IMAGE,
    });
  });

  it('uses a page image on both when the page has one', () => {
    const avatar = poetAvatarImage({ ...poet, hasAvatar: true });
    expect(resolveSocialImages(avatar)).toEqual({ openGraph: avatar, twitter: avatar });
  });
});
