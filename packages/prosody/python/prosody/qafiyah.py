from enum import Enum

from prosody.result import Err, Ok, ProsodyError, Result
from prosody.scan import ALEF, DAMMA, FATHA, HEH, KASRA, SUKUN, TEH_MARBUTA, WAW, YEH, Scan


class QafiyahTerm(Enum):
    QAFIYAH = "القافية"
    NICKNAME = "لقب القافية"
    WEIGHT = "وزن القافية"
    RAWI = "الروي"
    RAWI_HARAKAH = "حركة الروي"
    WASL = "الوصل"
    KHOROJ = "الخروج"
    TAASEES = "التأسيس"
    DAKHEEL = "الدخيل"
    RADF = "الردف"
    MAJRA = "المجرى"
    TAWJEEH = "التوجيه"
    NAFATH = "النفاذ"
    HATHW = "الحذو"
    ESHBAA = "الإشباع"
    RASS = "الرس"

    @property
    def label(self) -> str:
        return self.value


MISSING = "لا يوجد"
WASL_MISSING = "لايوجد"
UNKNOWN_SHAPE = "هناك خطأ"
UNKNOWN_LETTER = "خطأ"
SAKINAH = "ساكنة"

SHAPE_SEARCH_ORDER = ("100", "1010", "10110", "101110", "101110")
NICKNAMES = {"100": "مترادف", "1010": "متواتر", "10110": "متدارك", "101110": "متراكب", "1011110": "متكاوس"}
ADDED_MAD_TAILS = frozenset({KASRA + YEH + SUKUN, DAMMA + WAW + SUKUN, FATHA + ALEF + SUKUN})
MADD_LETTERS = frozenset({ALEF, YEH, WAW})

LETTER_NAMES = {
    "ا": "ألف",
    "أ": "ألف",
    "ؤ": "همزة",
    "ئ": "همزة",
    "ء": "همزة",
    "ب": "باء",
    "ت": "تاء",
    "ث": "ثاء",
    "ج": "جيم",
    "ح": "حاء",
    "خ": "خاء",
    "د": "دال",
    "ذ": "ذال",
    "ر": "راء",
    "ز": "زاي",
    "س": "سين",
    "ش": "شين",
    "ص": "صاد",
    "ض": "ضاد",
    "ط": "طاء",
    "ظ": "ظاء",
    "ع": "عين",
    "غ": "غين",
    "ف": "فاء",
    "ق": "قاف",
    "ك": "كاف",
    "ل": "لام",
    "م": "ميم",
    "ن": "نون",
    "ه": "هاء",
    "و": "واو",
    "ي": "ياء",
    "ى": "مقصورة",
    "ة": "تاء مربوطة",
    SUKUN: SAKINAH,
    FATHA: "مفتوحة",
    KASRA: "مكسورة",
    DAMMA: "مضمومة",
}


def analyze_qafiyah(ajuz: Scan, ajuz_text: str) -> Result[dict[QafiyahTerm, str], ProsodyError]:
    writing, bits = ajuz.writing, ajuz.bits
    end = len(writing)

    def at(index: int) -> str:
        if index < 0:
            raise IndexError(index)
        return writing[index]

    def tail(text: str, start: int, stop: int) -> str:
        if start < 0:
            raise IndexError(start)
        return text[start:stop]

    def name_of(char: str) -> str:
        return LETTER_NAMES.get(char, UNKNOWN_LETTER)

    def vowel_name(index: int) -> str:
        return name_of(at(index)).replace(TEH_MARBUTA, "")

    def pair(index: int) -> str:
        return f"{name_of(at(index))} {name_of(at(index + 1))}"

    try:
        shape = next(
            (
                shape
                for length, shape in enumerate(SHAPE_SEARCH_ORDER, start=3)
                if tail(bits, len(bits) - length, len(bits)) == shape
            ),
            UNKNOWN_SHAPE,
        )
        result = {
            QafiyahTerm.QAFIYAH: tail(writing, end - len(shape) * 2, end),
            QafiyahTerm.NICKNAME: NICKNAMES.get(shape, UNKNOWN_SHAPE),
            QafiyahTerm.WEIGHT: shape.replace("1", "/"),
            QafiyahTerm.WASL: WASL_MISSING,
            QafiyahTerm.MAJRA: MISSING,
            QafiyahTerm.TAWJEEH: MISSING,
            QafiyahTerm.NAFATH: MISSING,
            QafiyahTerm.HATHW: MISSING,
            QafiyahTerm.ESHBAA: MISSING,
            QafiyahTerm.RASS: MISSING,
        }

        if (at(end - 3) != KASRA and at(end - 2) != YEH and at(end - 1) != SUKUN) or (
            at(end - 3) != FATHA and at(end - 2) != WAW and at(end - 1) != SUKUN
        ):
            minus = 1
        elif (at(end - 3) != SUKUN and at(end - 2) == HEH and at(end - 1) == SUKUN) or (
            at(end - 2) != ALEF and at(end - 1) != SUKUN
        ):
            minus = 2
        else:
            minus = 2 if at(end - 2) in MADD_LETTERS else 0

        result[QafiyahTerm.KHOROJ] = pair(end - minus) if at(end - 2 - minus) == HEH else MISSING

        if result[QafiyahTerm.NICKNAME] == NICKNAMES["100"]:
            rawi = pair(end - 2) + " "
        elif tail(writing, end - 3, end - 1) not in ADDED_MAD_TAILS:
            rawi = pair(end - 4)
        else:
            rawi = pair(end - 2 - minus)
        result[QafiyahTerm.RAWI] = rawi

        last = ajuz_text[-1]
        if last in MADD_LETTERS or (last == SUKUN and ajuz_text[-2] in MADD_LETTERS):
            wasl = pair(end - minus) if at(end - 2 - minus) != HEH else MISSING
            result[QafiyahTerm.WASL] = WASL_MISSING if wasl == rawi else wasl

        has_taasees = at(end - 6 - minus) == ALEF and at(end - 5 - minus) == SUKUN
        result[QafiyahTerm.TAASEES] = ALEF if has_taasees else MISSING
        result[QafiyahTerm.DAKHEEL] = pair(end - 4 - minus) if has_taasees else MISSING

        has_radf = at(end - 3 - minus) == SUKUN and at(end - 4 - minus) in MADD_LETTERS
        result[QafiyahTerm.RADF] = name_of(at(end - 4 - minus)) if has_radf else MISSING

        is_moqayyad = SAKINAH in rawi
        result[QafiyahTerm.RAWI_HARAKAH] = "مقيد" if is_moqayyad else "مطلق"
        result[QafiyahTerm.TAWJEEH if is_moqayyad else QafiyahTerm.MAJRA] = vowel_name(end - minus - 1)

        if LETTER_NAMES[HEH] in rawi:
            result[QafiyahTerm.NAFATH] = vowel_name(end - minus - 1)
        if has_radf:
            result[QafiyahTerm.HATHW] = vowel_name(end - 5 - minus)
        if has_taasees:
            result[QafiyahTerm.ESHBAA] = vowel_name(end - minus - 3)
            result[QafiyahTerm.RASS] = vowel_name(end - 7 - minus)
    except IndexError:
        return Err(ProsodyError.HEMISTICH_TOO_SHORT_FOR_QAFIYAH)
    return Ok(result)
