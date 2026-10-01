from dataclasses import dataclass

from prosody.result import Err, Ok, ProsodyError, Result

FATHA = "َ"
DAMMA = "ُ"
KASRA = "ِ"
FATHATAN = "ً"
DAMMATAN = "ٌ"
KASRATAN = "ٍ"
SUKUN = "ْ"
SHADDA = "ّ"

ALEF = "ا"
ALEF_HAMZA = "أ"
ALEF_MADDAH = "آ"
ALEF_MAKSURA = "ى"
WAW = "و"
YEH = "ي"
LAM = "ل"
NOON = "ن"
TEH = "ت"
TEH_MARBUTA = "ة"
HEH = "ه"
FEH = "ف"

MOVING = "1"
STILL = "0"
MOVING_THEN_STILL = "10"
STILL_THEN_MOVING = "01"

DIACRITICS = frozenset({FATHA, KASRA, DAMMA, FATHATAN, KASRATAN, DAMMATAN, SUKUN, SHADDA})
HAMZAS = frozenset({ALEF_HAMZA, "ؤ", "ء", "ئ"})
SUN_LETTERS = frozenset("تثدذرزسشصضطظنل")
MOON_LETTERS = frozenset("أإؤابجحخعغفقكمهوي")
MAD_LETTERS = frozenset({ALEF, WAW, YEH, ALEF_MAKSURA})

WEIGHTS = {
    FATHA: MOVING,
    KASRA: MOVING,
    DAMMA: MOVING,
    FATHATAN: MOVING_THEN_STILL,
    KASRATAN: MOVING_THEN_STILL,
    DAMMATAN: MOVING_THEN_STILL,
    SUKUN: STILL,
    SHADDA: STILL_THEN_MOVING,
}
TANWEEN_VOWELS = {FATHATAN: FATHA, KASRATAN: KASRA, DAMMATAN: DAMMA}
MAD_FOR_VOWEL = {FATHA: ALEF + SUKUN, KASRA: YEH + SUKUN, DAMMA: WAW + SUKUN}
NO_MAD_LETTER = "-1"
PAD = 5

REWRITES = (
    ("ـ", ""),
    ("َّ", "َّ"),
    ("ُّ", "ُّ"),
    ("ِّ", "ِّ"),
    (",", ""),
    ("،", ""),
    ("ٰ", ""),
    ("الله", "اللاه"),
    ("اللّه", "اللاه"),
    ("اللَّه", "اللاه"),
    ("لِلَّه", "لللاه"),
    ("للَّه", "لللاه"),
    ("لِلّه", "لللاه"),
    ("للّه", "لللاه"),
    ("لَكِن", "لَاكِن"),
    ("لَكِنْ", "لَاكِن"),
    ("لَكن", "لَاكِن"),
    ("لكِن", "لَاكِن"),
    ("لكن", "لَاكِن"),
    ("هَذَا", "هَاذَا"),
    ("هذا", "هَاذَا"),
    ("هَذا", "هَاذَا"),
    ("هَذِه", "هَاذِه"),
    ("هذه", "هَاذِه"),
    ("هَذه", "هَاذِه"),
    ("هَذَان", "هَاذَان"),
    ("هَذَانْ", "هَاذَان"),
    ("هّذان", "هَاذَان"),
    ("هَؤُلَاء", "هَاؤُلَاء"),
    ("هَؤُلَاءْ", "هَاؤُلَاء"),
    ("هَؤلاء", "هَاؤُلَاء"),
    ("هؤلاء", "هَاؤُلَاء"),
    ("هَذِي", "هَاذِي"),
    ("هَذي", "هَاذِي"),
    ("هذي", "هَاذِي"),
    ("اَ", ""),
    ("اِ", ""),
    ("اُ", ""),
    ("اْ", ""),
    ("اّ", ""),
    (" َ", "َ"),
    (" ِ", "ِ"),
    (" ُ", "ُ"),
    ("\r", ""),
)


@dataclass(frozen=True)
class Scan:
    writing: str
    bits: str


def is_letter(char: str) -> bool:
    return char not in DIACRITICS


def is_definite_article(char: str, next_char: str) -> bool:
    return char == ALEF and next_char == LAM


def weight_of(char: str) -> str:
    return WEIGHTS.get(char, "")


def scan_hemistich(hemistich: str) -> Result[Scan, ProsodyError]:
    for pattern, replacement in REWRITES:
        hemistich = hemistich.replace(pattern, replacement)
    line = " " * PAD + hemistich + " " * PAD
    last_real_index = len(line) - PAD - 2
    writing: list[str] = []
    bits: list[str] = []

    i = PAD
    while i < len(line):
        cur = line[i]
        if cur == " ":
            if PAD < i <= last_real_index:
                writing.append(" ")
            i += 1
            continue
        if not is_letter(cur):
            i += 1
            continue

        prev2, prev = line[i - 2], line[i - 1]
        nxt, nxt2, nxt3, nxt4 = line[i + 1], line[i + 2], line[i + 3], line[i + 4]
        shadda_vowel = nxt4 if weight_of(nxt4) == MOVING and weight_of(nxt3) == STILL_THEN_MOVING else FATHA

        if is_letter(nxt):
            if cur == ALEF_MADDAH:
                bits.append(MOVING_THEN_STILL)
                writing.append(cur)
            elif cur in HAMZAS:
                bits.append(MOVING)
                writing.append(cur + FATHA)
            elif cur == LAM and nxt == LAM and is_letter(nxt2):
                if nxt2 in SUN_LETTERS:
                    writing.append(cur + KASRA + nxt2 + SUKUN + nxt2 + shadda_vowel)
                    bits.append(MOVING + STILL + MOVING)
                    i += 2
                elif nxt2 in MOON_LETTERS:
                    writing.append(cur + KASRA + nxt + SUKUN + nxt2 + (FATHA if is_letter(nxt3) else nxt3))
                    bits.append(MOVING + STILL + MOVING)
                    i += 2
            elif cur == ALEF:
                if is_definite_article(cur, nxt) and is_letter(nxt2):
                    if nxt2 in SUN_LETTERS or nxt2 in MOON_LETTERS:
                        if i == PAD:
                            writing.append(ALEF_HAMZA)
                            bits.append(MOVING)
                        if nxt2 in SUN_LETTERS:
                            writing.append(nxt2 + SUKUN + nxt2 + shadda_vowel)
                        else:
                            writing.append(nxt + SUKUN + nxt2 + (FATHA if is_letter(nxt3) else nxt3))
                        bits.append(STILL_THEN_MOVING)
                        i += 2
                    else:
                        writing.append(cur + FATHA)
                        bits.append(STILL)
                elif (prev == WAW and nxt == " ") or i == last_real_index:
                    i += 1
                elif not is_definite_article(nxt2, nxt3):
                    if (prev in (FEH, WAW) and prev2 == " ") or prev == " ":
                        if prev == " " and prev2 == " ":
                            writing.append(cur + FATHA)
                            bits.append(MOVING)
                    elif not (nxt2 == ALEF and nxt == " "):
                        writing.append(cur + SUKUN)
                        bits.append(STILL)
            elif cur in MAD_LETTERS:
                if cur == WAW and nxt == ALEF and nxt2 == " " and is_definite_article(nxt3, nxt4):
                    i += 1
                elif not (is_definite_article(nxt2, nxt3) and nxt == " "):
                    if (nxt in MAD_LETTERS and i != last_real_index) or prev == " ":
                        writing.append(cur + FATHA)
                        bits.append(MOVING)
                    else:
                        writing.append(cur + SUKUN)
                        bits.append(STILL)
            elif nxt in (ALEF, ALEF_MAKSURA) and weight_of(nxt2) == MOVING_THEN_STILL:
                writing.append(cur + TANWEEN_VOWELS[nxt2] + NOON + SUKUN)
                bits.append(MOVING_THEN_STILL)
                i += 1
            elif (nxt in MAD_LETTERS and weight_of(nxt2) != MOVING) or prev == " ":
                writing.append(cur + FATHA)
                bits.append(MOVING)
            else:
                writing.append(cur + SUKUN)
                bits.append(STILL)
        elif (
            weight_of(prev) == MOVING
            and cur == HEH
            and weight_of(nxt) == MOVING
            and nxt2 == " "
            and not is_definite_article(nxt3, nxt4)
        ):
            writing.append(cur + nxt + MAD_FOR_VOWEL[nxt])
            bits.append(MOVING_THEN_STILL)
            i += 1
        elif weight_of(nxt) == MOVING_THEN_STILL:
            writing.append((TEH if cur == TEH_MARBUTA else cur) + TANWEEN_VOWELS[nxt] + NOON + SUKUN)
            bits.append(MOVING_THEN_STILL)
            if nxt2 in (ALEF, ALEF_MAKSURA) and nxt3 == " ":
                i += 1
            i += 1
        elif weight_of(nxt) == STILL_THEN_MOVING:
            writing.append(cur + SUKUN + cur)
            bits.append(STILL_THEN_MOVING)
            if weight_of(nxt2) == MOVING_THEN_STILL:
                bits.append(STILL)
                writing.append(TANWEEN_VOWELS[nxt2] + NOON + SUKUN)
                if nxt3 == ALEF and line[i + 5] == " ":
                    i += 1
                i += 1
            elif weight_of(nxt2) == MOVING:
                writing.append(nxt2)
                i += 1
            else:
                writing.append(FATHA)
            i += 1
        else:
            writing.append(cur + nxt)
            bits.append(weight_of(nxt))
            i += 1
        i += 1

    if len(bits) == 0:
        return Err(ProsodyError.EMPTY_HEMISTICH)
    scan = Scan(writing="".join(writing), bits="".join(bits))
    if scan.bits[-1] == MOVING:
        return Ok(
            Scan(writing=scan.writing + MAD_FOR_VOWEL.get(scan.writing[-1], NO_MAD_LETTER), bits=scan.bits + STILL)
        )
    return Ok(scan)
