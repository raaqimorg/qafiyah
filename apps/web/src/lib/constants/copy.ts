import { toArabicDigits } from '@/lib/arabic';
import { SITE_NAME_AR } from '@/lib/constants/site-meta';

export const ERROR_TEXTS = {
  unexpected: 'عذرًا، وقع خلل غير متوقّع.',
  loadFailed: 'تعذّر تحميل البيانات.',
  retry: 'إعادة المحاولة',
  goHome: 'العودة إلى الرئيسة',
  supportPrefix: 'إن استمرّت المشكلة، راسلنا على',
} as const;

export const SEARCH_TEXTS = {
  currentHeaderTitle: 'مرجع الشعر العربي',
  search: 'ابحث',
  filtersTitle: 'الفلاتر',
  erasLabel: 'العصور',
  metersLabel: 'البحور',
  poemTypesLabel: 'الأنواع',
  themesLabel: 'الأغراض',
  collectionsLabel: 'الدواوين',
  rhymesLabel: 'القوافي',
  allPlaceholder: 'الكل',
  filterOnlyResultLabel: 'بهذه الفلاتر',
  noFilterResultsText: 'لم يُعثر على نتائج بهذه الفلاتر',
  arabicOnlyError: 'البحث بالعربية فقط',
  poemsSearchPlaceholder: 'ابحث في أكثر من ٦ ملايين بيت',
  poetsSearchPlaceholder: 'ابحث عن ديوان شاعر',
  poemSingular: 'بيت',
  poetSingular: 'شاعر',
  maxLengthErrorTemplate: 'يجب ألا يتجاوز النص {n} حرفا',
  poemsSectionTitle: 'القصائد',
  poetsSectionTitle: 'الشعراء',
  loadMorePoems: 'تحميل المزيد',
  exactLabel: 'تطابق حرفي',
  exactToggleEnableAria: 'تفعيل التطابق الحرفي في نتائج البحث',
  exactToggleDisableAria: 'إلغاء التطابق الحرفي في نتائج البحث',
  feelingsLabel: 'البحث بالمشاعر',
} as const;

export const FEELING_TEXTS = {
  copyVerse: 'نسخ البيت',
  verseCopied: 'تم نسخ البيت',
} as const;

export const POEM_TOOLBAR_TEXTS = {
  label: 'أدوات القراءة',
  fontSizeDecrease: 'تصغير خط القصيدة',
  fontSizeIncrease: 'تكبير خط القصيدة',
  spacingDecrease: 'تضييق المسافة بين الأبيات',
  spacingIncrease: 'توسيع المسافة بين الأبيات',
  toggleTheme: 'تبديل المظهر',
  cycleFontFamily: 'تبديل الخط',
  tashkeel: 'التشكيل',
  hideTashkeel: 'إخفاء التشكيل',
} as const;

export const SHARE_BAR_TEXTS = {
  label: 'مشاركة',
  x: 'مشاركة على إكس',
  facebook: 'مشاركة على فيسبوك',
  whatsapp: 'مشاركة على واتساب',
  telegram: 'مشاركة على تيليجرام',
  reddit: 'مشاركة على ريديت',
  linkedin: 'مشاركة على لينكدإن',
  copyLink: 'نسخ الرابط',
  copied: 'تم نسخ الرابط',
  copyFailed: 'تعذر النسخ. حاول مرة أخرى أو انسخ النص يدويًا.',
} as const;

export const SHARE_BUTTON_TEXTS = {
  label: 'مشاركة القصيدة',
  open: 'فتح نافذة المشاركة',
  close: 'إغلاق نافذة المشاركة',
  verses: 'الأبيات',
  addVerse: 'إضافة بيت',
  removeVerse: 'حذف بيت',
  copyText: 'نسخ النص',
  copiedText: 'تم نسخ النص',
  copyLink: 'نسخ الرابط',
  copiedLink: 'تم نسخ الرابط',
} as const;

export const NOT_FOUND_MESSAGE_AR = 'الصفحة غير موجودة';
export const SERVER_ERROR_MESSAGE_AR = 'حدث خطأ في الخادم';

export const NOT_FOUND_CODE = toArabicDigits(404);
export const NOT_FOUND_TITLE = `${NOT_FOUND_CODE} | ${NOT_FOUND_MESSAGE_AR}`;

export const SERVER_ERROR_CODE = toArabicDigits(500);
export const SERVER_ERROR_TITLE = `${SERVER_ERROR_CODE} | ${SERVER_ERROR_MESSAGE_AR}`;

export function formatPoemCopySignature(poetName: string, canonicalUrl: string): string {
  return `${poetName} | ${SITE_NAME_AR}: ${canonicalUrl}`;
}

export const POEM_SELECTION_SHARE_TEXTS = {
  share: 'مشاركة المحدد',
  title: 'مشاركة الأبيات المحددة',
  shareFailed: 'تعذرت المشاركة. اختر وسيلة أخرى أو انسخ الأبيات.',
} as const;
