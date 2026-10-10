export type Feeling = {
  readonly slug: string;
  readonly name: string;
  readonly children: readonly Feeling[];
};

const leaf = (slug: string, name: string): Feeling => ({ slug, name, children: [] });

const feeling = (slug: string, name: string, children: readonly Feeling[]): Feeling => ({
  slug,
  name,
  children,
});

export const FEELING_TREE = [
  feeling('alkhawf', 'الخوف', [
    feeling('faza', 'فزع', [leaf('khaif', 'خائف')]),
    feeling('qalaq', 'قلق', [leaf('murtabik', 'مرتبك'), leaf('mudtaribalbal', 'مضطرب البال')]),
    feeling('ghayramin', 'غير آمن', [leaf('ghayrkafi', 'غير كافي'), leaf('dhalil', 'ذليل')]),
    feeling('daf', 'ضعف', [leaf('muhammash', 'مهمَّش')]),
    feeling('rafd', 'رفض', [
      leaf('mustabad', 'مستبعد'),
      leaf('mudtahad', 'مضطهد'),
      leaf('inkimash', 'انكماش'),
    ]),
    feeling('tahdid', 'تهديد', [leaf('mutawattir', 'متوتر'), leaf('makshuf', 'مكشوف')]),
  ]),
  feeling('alghadab', 'الغضب', [
    feeling('khidhlan', 'خذلان', [leaf('khiyana', 'خيانة'), leaf('mustaa', 'مستاء')]),
    feeling('idhlal', 'إذلال', [leaf('idtihad', 'اضطهاد'), leaf('sukhriya', 'سخرية')]),
    feeling('hiqd', 'حقد', [leaf('naqim', 'ناقم'), leaf('intahak', 'انتهك')]),
    feeling('ghadib', 'غاضب', [leaf('ghadban', 'غضبان'), leaf('ghayur', 'غيور')]),
    feeling('udwaniya', 'عدوانية', [leaf('istifzaz', 'استفزاز'), leaf('sharasa', 'شراسة')]),
    feeling('ihbat', 'احباط', [leaf('munzaij', 'منزعج')]),
    feeling('mutabaid', 'متباعد', [leaf('munsahib', 'منسحب'), leaf('faqidalihsas', 'فاقد الإحساس')]),
    feeling('ihraj', 'إحراج', [leaf('mutashakkik', 'متشكك')]),
  ]),
  feeling('alishmizaz', 'الإشمئزاز', [
    feeling('rafid', 'رافض', [leaf('naqid', 'ناقد')]),
    feeling('khaibalamal', 'خائب الأمل', [leaf('murawwa', 'مروع'), leaf('thar', 'ثار')]),
    feeling('fazi', 'فظيع', [leaf('mushmaizz', 'مشمئز'), leaf('karih', 'كريه')]),
    feeling('nufur', 'نفور', [leaf('hala', 'هلع'), leaf('mutaraddid', 'متردد')]),
  ]),
  feeling('alhuzn', 'الحزن', [
    feeling('majruh', 'مجروح', [leaf('munharij', 'منحرج'), leaf('mukhayyab', 'مخيب')]),
    feeling('iktiab', 'اكتئاب', [
      leaf('munkasir', 'منكسر'),
      leaf('khaliminalmashair', 'خالي من المشاعر'),
    ]),
    feeling('mudhnib', 'مذنب', [leaf('khajul', 'خجول'), leaf('nadama', 'ندامة')]),
    feeling('yais', 'يائس', [leaf('ajiz', 'عاجز'), leaf('hazin', 'حزين')]),
    feeling('daif', 'ضعيف', [leaf('hash', 'هش'), leaf('dahiya', 'ضحية')]),
    feeling('wahid', 'وحيد', [leaf('takhallaanhu', 'تخلى عنه'), leaf('mazul', 'معزول')]),
  ]),
  feeling('alsaada', 'السعادة', [
    feeling('marah', 'مرح', [leaf('thair', 'ثائر'), leaf('mushakis', 'مشاكس')]),
    feeling('qanu', 'قنوع', [leaf('hurr', 'حر'), leaf('mubtahij', 'مبتهج')]),
    feeling('muhtamm', 'مهتم', [leaf('fuduli', 'فضولي'), leaf('mutasail', 'متسائل')]),
    feeling('fakhur', 'فخور', [leaf('najih', 'ناجح')]),
    feeling('maqbul', 'مقبول', [leaf('muhtaram', 'محترم'), leaf('mukarram', 'مكرَّم')]),
    feeling('qawi', 'قوي', [leaf('shuja', 'شجاع'), leaf('ibdai', 'إبداعي')]),
    feeling('musalim', 'مسالم', [leaf('muhibb', 'محب'), leaf('shakir', 'شاكر')]),
    feeling('wathiq', 'واثق', [leaf('hassas', 'حساس'), leaf('wadud', 'ودود')]),
    feeling('mutafail', 'متفائل', [leaf('mulham', 'ملهم')]),
  ]),
  feeling('almufajaa', 'المفاجأة', [
    feeling('madhhul', 'مذهول', [
      leaf('inbihar', 'انبهار'),
      leaf('ijab', 'إعجاب'),
      leaf('taajjub', 'تعجب'),
    ]),
    feeling('hair', 'حائر', [
      leaf('irtibak', 'ارتباك'),
      leaf('tashwish', 'تشويش'),
      leaf('iltibas', 'التباس'),
    ]),
    feeling('mundahish', 'مندهش', [leaf('rahba', 'رهبة')]),
    feeling('mutahammis', 'متحمس', [leaf('nashit', 'نشيط'), leaf('mutalahhif', 'متلهف')]),
  ]),
  feeling('alsu', 'السوء', [
    feeling('dhanb', 'ذنب', [leaf('nadam', 'ندم'), leaf('tanib', 'تأنيب'), leaf('lawm', 'لوم')]),
    feeling('ar', 'عار', [leaf('khizy', 'خزي'), leaf('fadiha', 'فضيحة')]),
    feeling('karahiya', 'كراهية', [leaf('bughd', 'بغض'), leaf('daghina', 'ضغينة')]),
  ]),
] as const satisfies readonly Feeling[];
