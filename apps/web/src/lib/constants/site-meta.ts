export const SITE_NAME_AR = 'قافية';

export const SITE_LANGUAGE = 'ar';

const OPEN_GRAPH_URL_PATH = '/open-graph.png';
export const SITE_LOGO_PATH = '/organization-logo.png';

export const SITE_TAGLINE_AR = 'مرجع الشعر العربي';
export const SITE_TITLE = `${SITE_NAME_AR} | ${SITE_TAGLINE_AR}`;

export type SocialImage = {
  readonly url: string;
  readonly alt: string;
  readonly mimeType: string;
  readonly width?: number | undefined;
  readonly height?: number | undefined;
};

export const DEFAULT_SOCIAL_IMAGE = {
  url: OPEN_GRAPH_URL_PATH,
  alt: SITE_TITLE,
  mimeType: 'image/png',
  width: 1200,
  height: 630,
} as const satisfies SocialImage;

export const LOGO_SOCIAL_IMAGE = {
  url: SITE_LOGO_PATH,
  alt: SITE_TITLE,
  mimeType: 'image/png',
  width: 512,
  height: 512,
} as const satisfies SocialImage;

export const POET_AVATAR_MIME_TYPE = 'image/webp';

export const SITE_DESCRIPTION =
  'نعنى بجمع شعر العرب، فحفظه من حفظ كتاب الله، كما قال ابن عباس: الشعر ديوان العرب، فإذا خفي علينا الحرف من القرآن، رجعنا إلى ديوانهم فالتمسناه فيه';
export const SITE_THEME_COLOR_HEX = '#fbfaf9';
export const SITE_THEME_COLOR_DARK_HEX = '#0e0c0c';
export const SCHEMA_ORG_CONTEXT = 'https://schema.org';

type NavLink = {
  readonly name: string;
  readonly href: string;
  readonly isExternal: boolean;
};

export const NAV_LINKS = [
  { name: 'الرئيسة', href: '/', isExternal: false },
  { name: 'الشعراء', href: '/poets', isExternal: false },
  { name: 'البحور', href: '/meters', isExternal: false },
  { name: 'القوافي', href: '/rhymes', isExternal: false },
  { name: 'الأغراض', href: '/themes', isExternal: false },
  { name: 'الدواوين', href: '/collections', isExternal: false },
] as const satisfies readonly NavLink[];

export const TWITTER_HANDLE = '@qafiyahx';
export const TWITTER_ID = '2063826482560974848';
export const X_HANDLE_URL = 'https://x.com/qafiyahx';
export const GITHUB_REPO_URL = 'https://github.com/raaqimorg/qafiyah';
export const TELEGRAM_HANDLE_URL = 'https://t.me/qafiyahx';

export const POEM_LANGUAGE = 'ar';
