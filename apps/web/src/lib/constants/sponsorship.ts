export type Sponsor = {
  readonly name: string;
  readonly image: string;
  readonly url: string;
};

export type SponsorshipPlan = {
  readonly days: number;
  readonly priceUsd: number;
};

export const BANNER_IMAGE_SIZE = { width: 992, height: 48 } as const;

export const EXAMPLE_SPONSORS: readonly Sponsor[] = [
  {
    name: 'دار المثال للنشر',
    image: '/sponsorship/examples/publisher.png',
    url: 'https://example.com',
  },
  {
    name: 'بودكاست المثال',
    image: '/sponsorship/examples/podcast.png',
    url: 'https://example.com',
  },
  { name: 'قناة المثال', image: '/sponsorship/examples/channel.png', url: 'https://example.com' },
  { name: 'موقع المثال', image: '/sponsorship/examples/website.png', url: 'https://example.com' },
  { name: 'تطبيق المثال', image: '/sponsorship/examples/app.png', url: 'https://example.com' },
  {
    name: 'مكتبة المثال',
    image: '/sponsorship/examples/bookstore.png',
    url: 'https://example.com',
  },
  { name: 'دورة المثال', image: '/sponsorship/examples/course.png', url: 'https://example.com' },
  { name: 'متجر المثال', image: '/sponsorship/examples/shop.png', url: 'https://example.com' },
  {
    name: 'نشرة المثال',
    image: '/sponsorship/examples/newsletter.png',
    url: 'https://example.com',
  },
  { name: 'عطور المثال', image: '/sponsorship/examples/perfume.png', url: 'https://example.com' },
];

export const EXAMPLE_POEM = {
  title: 'قفا نبك من ذكرى حبيب ومنزل',
  poet: 'امرؤ القيس',
  verses: [
    ['قِفا نَبكِ مِن ذكرَى حبيبٍ وَّمنزِلِ', 'بِسَقطِ اللِّوى بينَ الدَّخولِ فحَوْمَلِ'],
    ['فتُوضِحَ فالمِقراةِ لم يَعفُ رسمُها', 'لما نسجَتْها مِن جَنوبٍ وَّشَمْألِ'],
  ],
} as const;

export const SPONSORSHIP_PLANS: readonly SponsorshipPlan[] = [
  { days: 14, priceUsd: 3_499 },
  { days: 30, priceUsd: 4_999 },
  { days: 45, priceUsd: 7_999 },
];
