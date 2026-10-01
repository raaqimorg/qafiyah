from prosody.meter import BahrName, TaffelahVariant
from prosody.poem import analyze_poem
from prosody.qafiyah import QafiyahTerm
from prosody.result import Err, Ok, ProsodyError

SADR = "أَلَا لَا يَجْهَلَنْ أَحَدٌ عَلَيْنَا"
AJUZ = "فَنَجْهَلَ فَوْقَ جَهْلِ الْجَاهِلِينَا"


def test_a_wafer_bayt_matches_only_wafer_with_its_feet_per_hemistich() -> None:
    poem = analyze_poem([SADR, AJUZ])
    assert isinstance(poem, Ok)
    [meter] = poem.value[0].meters
    assert meter.bahr == BahrName.WAFER
    assert meter.sadr_feet == (TaffelahVariant.MAFAAEL_SAKIN_TON, TaffelahVariant.MAFAAELATON, TaffelahVariant.FAOOLON)
    assert meter.ajuz_feet == (TaffelahVariant.MAFAAELATON, TaffelahVariant.MAFAAEL_SAKIN_TON, TaffelahVariant.FAOOLON)


def test_the_qafiyah_of_a_yna_ending_is_mutawatir_with_a_noon_rawi_and_a_yaa_radf() -> None:
    poem = analyze_poem([SADR, AJUZ])
    assert isinstance(poem, Ok)
    qafiyah = poem.value[0].qafiyah
    assert qafiyah[QafiyahTerm.QAFIYAH] == "لِيْنَاْ"
    assert qafiyah[QafiyahTerm.WEIGHT] == "/0/0"
    assert qafiyah[QafiyahTerm.NICKNAME] == "متواتر"
    assert qafiyah[QafiyahTerm.RAWI] == "نون مفتوحة"
    assert qafiyah[QafiyahTerm.RADF] == "ياء"
    assert qafiyah[QafiyahTerm.RAWI_HARAKAH] == "مطلق"
    assert qafiyah[QafiyahTerm.MAJRA] == "مفتوح"


def test_a_single_hemistich_is_measured_against_itself() -> None:
    poem = analyze_poem([SADR])
    assert isinstance(poem, Ok)
    assert [meter.bahr for meter in poem.value[0].meters] == [BahrName.WAFER]


def test_a_bayt_that_fits_no_bahr_has_no_meter_matches() -> None:
    poem = analyze_poem(["بَبَبَبَبَبَبَبَبْ", "بَبَبَبَبَبَبَبَبْ"])
    assert isinstance(poem, Ok)
    assert poem.value[0].meters == ()


def test_an_empty_poem_is_an_empty_poem_error() -> None:
    assert analyze_poem([]) == Err(ProsodyError.EMPTY_POEM)


def test_an_ajuz_too_short_for_a_qafiyah_is_a_too_short_error() -> None:
    assert analyze_poem(["ب", "ب"]) == Err(ProsodyError.HEMISTICH_TOO_SHORT_FOR_QAFIYAH)
