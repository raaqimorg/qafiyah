export type Sponsor = {
  readonly name: string;
  readonly image: string;
  readonly url: string;
};

export type TextSponsor = {
  readonly name: string;
  readonly line: string;
  readonly action: string;
  readonly logo: string;
  readonly url: string;
};

export type SponsorshipPlan = {
  readonly days: number;
  readonly priceUsd: number;
};

export const BANNER_IMAGE_SIZE = { width: 992, height: 40 } as const;

const WIRDY_LOGO = `<g fill="currentColor"><circle cx="32.00" cy="10.50" r="6.4"/><circle cx="42.75" cy="13.38" r="6.4"/><circle cx="50.62" cy="21.25" r="6.4"/><circle cx="53.50" cy="32.00" r="6.4"/><circle cx="50.62" cy="42.75" r="6.4"/><circle cx="42.75" cy="50.62" r="6.4"/><circle cx="32.00" cy="53.50" r="6.4"/><circle cx="21.25" cy="50.62" r="6.4"/><circle cx="13.38" cy="42.75" r="6.4"/><circle cx="10.50" cy="32.00" r="6.4"/><circle cx="13.38" cy="21.25" r="6.4"/><circle cx="21.25" cy="13.38" r="6.4"/></g><circle cx="32" cy="32" r="12.5" fill="currentColor" fill-opacity="0.4"/>`;

export const SPONSOR: Sponsor | TextSponsor | undefined = {
  name: 'وِردي',
  line: 'أذكار صحيحة بتخريجها',
  action: 'زوروا الموقع',
  logo: WIRDY_LOGO,
  url: 'https://wirdy.site',
};

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
