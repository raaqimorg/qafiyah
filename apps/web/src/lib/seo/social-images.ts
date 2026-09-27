import {
  DEFAULT_SOCIAL_IMAGE,
  LOGO_SOCIAL_IMAGE,
  POET_AVATAR_MIME_TYPE,
  type SocialImage,
} from '@/lib/constants/site-meta';
import { poetAvatarUrl } from '@/lib/urls';

type AvatarOwner = {
  readonly slug: string;
  readonly name: string;
  readonly hasAvatar: boolean;
};

export function poetAvatarImage(poet: AvatarOwner): SocialImage | undefined {
  return poet.hasAvatar
    ? { url: poetAvatarUrl(poet.slug), alt: poet.name, mimeType: POET_AVATAR_MIME_TYPE }
    : undefined;
}

export function resolveSocialImages(pageImage: SocialImage | undefined): {
  readonly openGraph: SocialImage;
  readonly twitter: SocialImage;
} {
  return {
    openGraph: pageImage ?? DEFAULT_SOCIAL_IMAGE,
    twitter: pageImage ?? LOGO_SOCIAL_IMAGE,
  };
}
