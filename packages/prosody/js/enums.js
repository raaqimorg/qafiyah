const defineEnum = (entries) =>
  Object.freeze(
    Object.fromEntries(
      Object.entries(entries).map(([key, fields]) => [key, Object.freeze({ key, ...fields })])
    )
  );

export const BahrName = defineEnum({
  taweel: { label: 'الطويل' },
  basset: { label: 'البسيط' },
  bassetPart: { label: 'مجزوءالبسيط' },
  kamel: { label: 'الكامل' },
  kamelPart: { label: 'مجزوءالكامل' },
  wafer: { label: 'الوافر' },
  waferPart: { label: 'مجزوءالوافر' },
  khafeef: { label: 'الخفيف' },
  khafeefPart: { label: 'مجزوءالخفيف' },
  ramal: { label: 'الرمل' },
  ramalPart: { label: 'مجزوءالرمل' },
  motokareb: { label: 'المتقارب' },
  monsareh: { label: 'المنسرح' },
  madeed: { label: 'المديد' },
  sareeh: { label: 'السريع' },
  rajaz: { label: 'الرجز' },
  rajazPart: { label: 'مجزوء الرجز' },
  hazaj: { label: 'الهزج' },
  mohdath: { label: 'المحدث' },
  modareeh: { label: 'المضارع' },
  mojtath: { label: 'المجتث' },
  moqtateb: { label: 'المقتضب' },
  noResult: { label: 'لا نتيجة' },
});

export const BaseTaffelah = defineEnum({
  faaelaton: { label: 'فاعلاتن' },
  faaelon: { label: 'فاعلن' },
  faoolon: { label: 'فعولن' },
  mafaaeelon: { label: 'مفاعيلن' },
  mafeoolaat: { label: 'مفعولات' },
  mafaaelaton: { label: 'مفاعلتن' },
  mostafeelon: { label: 'مستفعلن' },
  motafaaelon: { label: 'متفاعلن' },
});

export const TaffelahVariant = defineEnum({
  faaelaton: { label: 'فاعلاتن', bits: '1011010' },
  faelaton: { label: 'فعلاتن', bits: '111010' },
  faaela: { label: 'فاعلا', bits: '10110' },
  faela: { label: 'فعلا', bits: '1110' },
  faaelat: { label: 'فاعلات', bits: '101100' },
  faelatSakin: { label: 'فعلاتْ', bits: '11100' },
  faaelatonnon: { label: 'فاعلاتنْنْ', bits: '10110100' },
  faelatonnon: { label: 'فعلاتنْنْ', bits: '1110100' },
  faael: { label: 'فاعل', bits: '1010' },
  faaelSakinTon: { label: 'فاعلْتنْ', bits: '101010' },
  faaelon: { label: 'فاعلن', bits: '10110' },
  faelon: { label: 'فعلن', bits: '1110' },
  faoolon: { label: 'فعولن', bits: '11010' },
  faooloDamma: { label: 'فعولُ', bits: '1101' },
  faoo: { label: 'فعو', bits: '110' },
  fae: { label: 'فع', bits: '10' },
  faoolSakin: { label: 'فعولْ', bits: '1100' },
  mafaaeelon: { label: 'مفاعيلن', bits: '1101010' },
  mafaaee: { label: 'مفاعي', bits: '11010' },
  mafaaeel: { label: 'مفاعيل', bits: '110100' },
  mafaaeeloDamma: { label: 'مفاعيلُ', bits: '110101' },
  mafaaelon: { label: 'مفاعلن', bits: '110110' },
  faaelaan: { label: 'فاعلان', bits: '1010101' },
  mafeoolaat: { label: 'مفعولات', bits: '101101' },
  mafaaelaton: { label: 'مفاعلتن', bits: '1101110' },
  mafaaelSakinTon: { label: 'مفاعلْتنْ', bits: '1101010' },
  mafaaelSakin: { label: 'مفاعلْ', bits: '11010' },
  mostafeelon: { label: 'مستفعلن', bits: '1010110' },
  motafeelon: { label: 'متفعلن', bits: '110110' },
  mostaelon: { label: 'مستعلن', bits: '101110' },
  motaelon: { label: 'متعلن', bits: '11110' },
  mostafeelSakin: { label: 'مستفعلْ', bits: '101010' },
  motafaaelon: { label: 'متفاعلن', bits: '1110110' },
  motafaaeloDamma: { label: 'متفاعلُ', bits: '1010110' },
  motafaaelSakin: { label: 'متفاعلْ', bits: '111010' },
  motSakinFaaelon: { label: 'متْفاعلنْ', bits: '101010' },
  motafa: { label: 'متفا', bits: '1110' },
  motSakinFa: { label: 'متْفا', bits: '1010' },
  motafaaelaaton: { label: 'متفاعلاتن', bits: '111011010' },
  motSakinFaaelaaton: { label: 'متْفاعلاتن', bits: '101011010' },
  motafaaelaan: { label: 'متفاعلان', bits: '11101100' },
  motSakinFaaelaan: { label: 'متْفاعلان', bits: '10101100' },
});

export const ShatrHalf = defineEnum({
  first: { label: 'الصدر' },
  second: { label: 'العجز' },
});

export const HarakahWeight = defineEnum({
  moving: { bits: '1' },
  still: { bits: '0' },
  movingThenStill: { bits: '10' },
  stillThenMoving: { bits: '01' },
  hamza: { bits: '1m' },
  none: { bits: '-1' },
});

export const HarakahName = defineEnum({
  sakinah: { label: 'ساكنة' },
  maftoohah: { label: 'مفتوحة' },
  maksoorah: { label: 'مكسورة' },
  madmoomah: { label: 'مضمومة' },
});

export const RawiKind = defineEnum({
  motlaq: { label: 'مطلق' },
  moqayyad: { label: 'مقيد' },
});

export const QafiyahNickname = defineEnum({
  motaradef: { label: 'مترادف', bits: '100' },
  motawater: { label: 'متواتر', bits: '1010' },
  motadarek: { label: 'متدارك', bits: '10110' },
  motarakeb: { label: 'متراكب', bits: '101110' },
  motakawes: { label: 'متكاوس', bits: '1011110' },
});

export const qafiyahNicknameForBits = (bits) =>
  Object.values(QafiyahNickname).find((nickname) => nickname.bits === bits) ?? null;

export const QafiyahTerm = defineEnum({
  qafiyah: { label: 'القافية' },
  nickname: { label: 'لقب القافية' },
  weight: { label: 'وزن القافية' },
  rawi: { label: 'الروي' },
  rawiHarakah: { label: 'حركة الروي' },
  wasl: { label: 'الوصل' },
  khoroj: { label: 'الخروج' },
  taasees: { label: 'التأسيس' },
  dakheel: { label: 'الدخيل' },
  radf: { label: 'الردف' },
  majra: { label: 'المجرى' },
  tawjeeh: { label: 'التوجيه' },
  nafath: { label: 'النفاذ' },
  hathw: { label: 'الحذو' },
  eshbaa: { label: 'الإشباع' },
  rass: { label: 'الرس' },
});
