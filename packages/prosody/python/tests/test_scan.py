from prosody.result import Err, Ok, ProsodyError
from prosody.scan import Scan, scan_hemistich


def test_a_fully_voweled_hemistich_is_respelled_and_encoded_with_its_final_vowel_stretched() -> None:
    assert scan_hemistich("قِفَا نَبْكِ مِنْ ذِكْرَى حَبِيبٍ وَمَنْزِلِ") == Ok(
        Scan(writing="قِفَاْ نَبْكِ مِنْ ذِكْرَىْ حَبِيْبِنْ وَمَنْزِلِيْ", bits="11010110101011010110110")
    )


def test_tanween_is_encoded_as_a_moving_letter_followed_by_a_still_noon() -> None:
    assert scan_hemistich("أَلَا لَا يَجْهَلَنْ أَحَدٌ عَلَيْنَا") == Ok(
        Scan(writing="أَلَاْ لَاْ يَجْهَلَنْ أَحَدُنْ عَلَيْنَاْ", bits="1101010110111011010")
    )


def test_an_empty_or_blank_hemistich_is_an_empty_hemistich_error() -> None:
    assert scan_hemistich("") == Err(ProsodyError.EMPTY_HEMISTICH)
    assert scan_hemistich("   ") == Err(ProsodyError.EMPTY_HEMISTICH)
