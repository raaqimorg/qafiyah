from dataclasses import dataclass
from enum import Enum


class TaffelahVariant(Enum):
    FAAELATON = ("فاعلاتن", "1011010")
    FAELATON = ("فعلاتن", "111010")
    FAAELA = ("فاعلا", "10110")
    FAELA = ("فعلا", "1110")
    FAAELAT = ("فاعلات", "101100")
    FAELAT_SAKIN = ("فعلاتْ", "11100")
    FAAELATONNON = ("فاعلاتنْنْ", "10110100")
    FAELATONNON = ("فعلاتنْنْ", "1110100")
    FAAEL = ("فاعل", "1010")
    FAAEL_SAKIN_TON = ("فاعلْتنْ", "101010")
    FAAELON = ("فاعلن", "10110")
    FAELON = ("فعلن", "1110")
    FAOOLON = ("فعولن", "11010")
    FAOOLO_DAMMA = ("فعولُ", "1101")
    FAOO = ("فعو", "110")
    FAE = ("فع", "10")
    FAOOL_SAKIN = ("فعولْ", "1100")
    MAFAAEELON = ("مفاعيلن", "1101010")
    MAFAAEE = ("مفاعي", "11010")
    MAFAAEEL = ("مفاعيل", "110100")
    MAFAAEELO_DAMMA = ("مفاعيلُ", "110101")
    MAFAAELON = ("مفاعلن", "110110")
    FAAELAAN = ("فاعلان", "1010101")
    MAFEOOLAAT = ("مفعولات", "101101")
    MAFAAELATON = ("مفاعلتن", "1101110")
    MAFAAEL_SAKIN_TON = ("مفاعلْتنْ", "1101010")
    MAFAAEL_SAKIN = ("مفاعلْ", "11010")
    MOSTAFEELON = ("مستفعلن", "1010110")
    MOTAFEELON = ("متفعلن", "110110")
    MOSTAELON = ("مستعلن", "101110")
    MOTAELON = ("متعلن", "11110")
    MOSTAFEEL_SAKIN = ("مستفعلْ", "101010")
    MOTAFAAELON = ("متفاعلن", "1110110")
    MOTAFAAELO_DAMMA = ("متفاعلُ", "1010110")
    MOTAFAAEL_SAKIN = ("متفاعلْ", "111010")
    MOT_SAKIN_FAAELON = ("متْفاعلنْ", "101010")
    MOTAFA = ("متفا", "1110")
    MOT_SAKIN_FA = ("متْفا", "1010")
    MOTAFAAELAATON = ("متفاعلاتن", "111011010")
    MOT_SAKIN_FAAELAATON = ("متْفاعلاتن", "101011010")
    MOTAFAAELAAN = ("متفاعلان", "11101100")
    MOT_SAKIN_FAAELAAN = ("متْفاعلان", "10101100")

    @property
    def label(self) -> str:
        return self.value[0]

    @property
    def bits(self) -> str:
        return self.value[1]


class BahrName(Enum):
    TAWEEL = "الطويل"
    BASSET = "البسيط"
    BASSET_PART = "مجزوءالبسيط"
    KAMEL = "الكامل"
    KAMEL_PART = "مجزوءالكامل"
    WAFER = "الوافر"
    WAFER_PART = "مجزوءالوافر"
    KHAFEEF = "الخفيف"
    KHAFEEF_PART = "مجزوءالخفيف"
    RAMAL = "الرمل"
    RAMAL_PART = "مجزوءالرمل"
    MOTOKAREB = "المتقارب"
    MONSAREH = "المنسرح"
    MADEED = "المديد"
    SAREEH = "السريع"
    RAJAZ = "الرجز"
    RAJAZ_PART = "مجزوء الرجز"
    HAZAJ = "الهزج"
    MOHDATH = "المحدث"
    MODAREEH = "المضارع"
    MOJTATH = "المجتث"
    MOQTATEB = "المقتضب"

    @property
    def label(self) -> str:
        return self.value


@dataclass(frozen=True)
class Bahr:
    name: BahrName
    max_length: int
    sadr: tuple[tuple[TaffelahVariant, ...], ...]
    ajuz: tuple[tuple[TaffelahVariant, ...], ...]


@dataclass(frozen=True)
class MeterMatch:
    bahr: BahrName
    sadr_feet: tuple[TaffelahVariant, ...]
    ajuz_feet: tuple[TaffelahVariant, ...]


_V = TaffelahVariant

BAHRS = (
    Bahr(
        name=BahrName.TAWEEL,
        max_length=24,
        sadr=(
            (_V.FAOOLO_DAMMA, _V.FAOOLON),
            (_V.MAFAAELON, _V.MAFAAEELON),
            (_V.FAOOLO_DAMMA, _V.FAOOLON),
            (_V.MAFAAELON, _V.MAFAAEELON),
        ),
        ajuz=(
            (_V.FAOOLO_DAMMA, _V.FAOOLON),
            (_V.MAFAAELON, _V.MAFAAEELON),
            (_V.FAOOLO_DAMMA, _V.FAOOLON),
            (_V.MAFAAELON, _V.MAFAAEELON),
        ),
    ),
    Bahr(
        name=BahrName.BASSET,
        max_length=24,
        sadr=(
            (_V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.FAELON, _V.FAAELON),
            (_V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.FAELON, _V.FAAEL),
        ),
        ajuz=(
            (_V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.FAELON, _V.FAAELON),
            (_V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.FAELON, _V.FAAEL),
        ),
    ),
    Bahr(
        name=BahrName.BASSET_PART,
        max_length=17,
        sadr=(
            (_V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.FAAELON,),
            (_V.FAOOLON,),
        ),
        ajuz=(
            (_V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.FAAELON,),
            (_V.FAOOLON,),
        ),
    ),
    Bahr(
        name=BahrName.KAMEL,
        max_length=21,
        sadr=(
            (_V.MOTAFAAELO_DAMMA, _V.MOTAFAAELON),
            (_V.MOTAFAAELO_DAMMA, _V.MOTAFAAELON),
            (_V.MOT_SAKIN_FA, _V.MOT_SAKIN_FAAELON, _V.MOTAFAAEL_SAKIN, _V.MOTAFAAELO_DAMMA, _V.MOTAFAAELON),
        ),
        ajuz=(
            (_V.MOTAFAAELO_DAMMA, _V.MOTAFAAELON),
            (_V.MOTAFAAELO_DAMMA, _V.MOTAFAAELON),
            (_V.MOT_SAKIN_FA, _V.MOTAFAAELO_DAMMA, _V.MOTAFAAELON),
        ),
    ),
    Bahr(
        name=BahrName.KAMEL_PART,
        max_length=14,
        sadr=(
            (_V.MOTAFAAELO_DAMMA, _V.MOTAFAAELON),
            (
                _V.MOT_SAKIN_FAAELAAN,
                _V.MOTAFAAELAAN,
                _V.MOT_SAKIN_FAAELAATON,
                _V.MOTAFAAELAATON,
                _V.MOT_SAKIN_FAAELON,
                _V.MOTAFAAEL_SAKIN,
                _V.MOTAFAAELO_DAMMA,
                _V.MOTAFAAELON,
            ),
        ),
        ajuz=(
            (_V.MOTAFAAELO_DAMMA, _V.MOTAFAAELON),
            (
                _V.MOT_SAKIN_FAAELAAN,
                _V.MOTAFAAELAAN,
                _V.MOT_SAKIN_FAAELAATON,
                _V.MOTAFAAELAATON,
                _V.MOT_SAKIN_FAAELON,
                _V.MOTAFAAEL_SAKIN,
                _V.MOTAFAAELO_DAMMA,
                _V.MOTAFAAELON,
            ),
        ),
    ),
    Bahr(
        name=BahrName.WAFER,
        max_length=19,
        sadr=(
            (_V.MAFAAEL_SAKIN_TON, _V.MAFAAELATON),
            (_V.MAFAAEL_SAKIN_TON, _V.MAFAAELATON),
            (_V.FAOOLON,),
        ),
        ajuz=(
            (_V.MAFAAEL_SAKIN_TON, _V.MAFAAELATON),
            (_V.MAFAAEL_SAKIN_TON, _V.MAFAAELATON),
            (_V.FAOOLON,),
        ),
    ),
    Bahr(
        name=BahrName.WAFER_PART,
        max_length=14,
        sadr=(
            (_V.MAFAAEL_SAKIN_TON, _V.MAFAAELATON),
            (_V.MAFAAELATON,),
        ),
        ajuz=(
            (_V.MAFAAEL_SAKIN_TON, _V.MAFAAELATON),
            (_V.MAFAAEL_SAKIN_TON, _V.MAFAAELATON),
        ),
    ),
    Bahr(
        name=BahrName.KHAFEEF,
        max_length=21,
        sadr=(
            (_V.FAELATON, _V.FAAELATON),
            (_V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.FAAEL_SAKIN_TON, _V.FAELA, _V.FAELATON, _V.FAAELATON),
        ),
        ajuz=(
            (_V.FAELATON, _V.FAAELATON),
            (_V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.FAAEL_SAKIN_TON, _V.FAELA, _V.FAELATON, _V.FAAELATON),
        ),
    ),
    Bahr(
        name=BahrName.KHAFEEF_PART,
        max_length=14,
        sadr=(
            (_V.FAELATON, _V.FAAELATON),
            (_V.MOTAFEELON, _V.MOSTAFEELON),
        ),
        ajuz=(
            (_V.FAELATON, _V.FAAELATON),
            (_V.MOTAFEELON, _V.MOSTAFEELON),
        ),
    ),
    Bahr(
        name=BahrName.RAMAL,
        max_length=21,
        sadr=(
            (_V.FAELATON, _V.FAAELATON),
            (_V.FAELATON, _V.FAAELATON),
            (_V.FAELAT_SAKIN, _V.FAAELAT, _V.FAELA, _V.FAAELA, _V.FAELATON, _V.FAAELATON),
        ),
        ajuz=(
            (_V.FAELATON, _V.FAAELATON),
            (_V.FAELATON, _V.FAAELATON),
            (_V.FAELAT_SAKIN, _V.FAAELAT, _V.FAELA, _V.FAAELA, _V.FAELATON, _V.FAAELATON),
        ),
    ),
    Bahr(
        name=BahrName.RAMAL_PART,
        max_length=14,
        sadr=(
            (_V.FAELATON, _V.FAAELATON),
            (_V.FAELATON, _V.FAAELATON, _V.FAELA, _V.FAAELA, _V.FAELATON, _V.FAAELATON),
        ),
        ajuz=(
            (_V.FAELATON, _V.FAAELATON),
            (_V.FAELATON, _V.FAAELATON, _V.FAELA, _V.FAAELA, _V.FAELATON, _V.FAAELATON),
        ),
    ),
    Bahr(
        name=BahrName.MOTOKAREB,
        max_length=20,
        sadr=(
            (_V.FAOOLO_DAMMA, _V.FAOOLON),
            (_V.FAOOLO_DAMMA, _V.FAOOLON),
            (_V.FAOOLO_DAMMA, _V.FAOOLON),
            (_V.FAOOL_SAKIN, _V.FAE, _V.FAOO, _V.FAOOLO_DAMMA, _V.FAOOLON),
        ),
        ajuz=(
            (_V.FAOOLO_DAMMA, _V.FAOOLON),
            (_V.FAOOLO_DAMMA, _V.FAOOLON),
            (_V.FAOOLO_DAMMA, _V.FAOOLON),
            (_V.FAOOL_SAKIN, _V.FAE, _V.FAOO, _V.FAOOLO_DAMMA, _V.FAOOLON),
        ),
    ),
    Bahr(
        name=BahrName.MONSAREH,
        max_length=21,
        sadr=(
            (_V.MOTAELON, _V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.FAAELAAN, _V.MAFEOOLAAT),
            (_V.MOSTAFEEL_SAKIN, _V.MOTAFEELON, _V.MOSTAFEELON),
        ),
        ajuz=(
            (_V.MOTAELON, _V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.FAAELAAN, _V.MAFEOOLAAT),
            (_V.MOSTAFEEL_SAKIN, _V.MOTAFEELON, _V.MOSTAFEELON),
        ),
    ),
    Bahr(
        name=BahrName.MADEED,
        max_length=19,
        sadr=(
            (_V.FAELATON, _V.FAAELATON),
            (_V.FAELON, _V.FAAELON),
            (_V.FAAEL, _V.FAELAT_SAKIN, _V.FAAELAT, _V.FAELA, _V.FAAELA, _V.FAELATON, _V.FAAELATON),
        ),
        ajuz=(
            (_V.FAELATON, _V.FAAELATON),
            (_V.FAELON, _V.FAAELON),
            (_V.FAAEL, _V.FAELAT_SAKIN, _V.FAAELAT, _V.FAELA, _V.FAAELA, _V.FAELATON, _V.FAAELATON),
        ),
    ),
    Bahr(
        name=BahrName.SAREEH,
        max_length=19,
        sadr=(
            (_V.MOTAELON, _V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.MOTAELON, _V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.FAAEL, _V.FAELON, _V.FAAELON),
        ),
        ajuz=(
            (_V.MOTAELON, _V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.MOTAELON, _V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.FAAEL, _V.FAELON, _V.FAAELON),
        ),
    ),
    Bahr(
        name=BahrName.RAJAZ,
        max_length=21,
        sadr=(
            (_V.MOTAELON, _V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.MOTAELON, _V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.MOSTAFEEL_SAKIN, _V.MOTAELON, _V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
        ),
        ajuz=(
            (_V.MOTAELON, _V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.MOTAELON, _V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.MOSTAFEEL_SAKIN, _V.MOTAELON, _V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
        ),
    ),
    Bahr(
        name=BahrName.RAJAZ_PART,
        max_length=14,
        sadr=(
            (_V.MOTAELON, _V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.MOTAELON, _V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
        ),
        ajuz=(
            (_V.MOTAELON, _V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.MOTAELON, _V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
        ),
    ),
    Bahr(
        name=BahrName.HAZAJ,
        max_length=14,
        sadr=(
            (_V.MAFAAEELO_DAMMA, _V.MAFAAEELON),
            (_V.MAFAAEELO_DAMMA, _V.MAFAAEELON),
        ),
        ajuz=(
            (_V.MAFAAEELO_DAMMA, _V.MAFAAEELON),
            (_V.MAFAAEELO_DAMMA, _V.MAFAAEELON),
        ),
    ),
    Bahr(
        name=BahrName.MOHDATH,
        max_length=20,
        sadr=(
            (_V.FAAEL, _V.FAELON, _V.FAAELON),
            (_V.FAAEL, _V.FAELON, _V.FAAELON),
            (_V.FAAEL, _V.FAELON, _V.FAAELON),
            (_V.FAAEL, _V.FAELON, _V.FAAELON),
        ),
        ajuz=(
            (_V.FAAEL, _V.FAELON, _V.FAAELON),
            (_V.FAAEL, _V.FAELON, _V.FAAELON),
            (_V.FAAEL, _V.FAELON, _V.FAAELON),
            (_V.FAAEL, _V.FAELON, _V.FAAELON),
        ),
    ),
    Bahr(
        name=BahrName.MODAREEH,
        max_length=13,
        sadr=(
            (_V.MAFAAEELO_DAMMA,),
            (_V.FAAELATON,),
        ),
        ajuz=(
            (_V.MAFAAEELO_DAMMA,),
            (_V.FAAELATON,),
        ),
    ),
    Bahr(
        name=BahrName.MOJTATH,
        max_length=14,
        sadr=(
            (_V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.FAAEL_SAKIN_TON, _V.FAELATON, _V.FAAELATON),
        ),
        ajuz=(
            (_V.MOSTAELON, _V.MOTAFEELON, _V.MOSTAFEELON),
            (_V.FAAEL_SAKIN_TON, _V.FAELATON, _V.FAAELATON),
        ),
    ),
    Bahr(
        name=BahrName.MOQTATEB,
        max_length=12,
        sadr=(
            (_V.FAAELON,),
            (_V.MAFAAELATON,),
        ),
        ajuz=(
            (_V.FAAELON,),
            (_V.MAFAAELATON,),
        ),
    ),
)


def match_meters(sadr_bits: str, ajuz_bits: str) -> tuple[MeterMatch, ...]:
    matches: list[MeterMatch] = []
    for bahr in BAHRS:
        halves: list[tuple[TaffelahVariant, ...]] = []
        for slots, bits in ((bahr.sadr, sadr_bits), (bahr.ajuz, ajuz_bits)):
            feet: list[TaffelahVariant] = []
            start = 0
            for allowed in slots:
                fitting = [variant for variant in allowed if bits[start : start + len(variant.bits)] == variant.bits]
                if len(fitting) == 0:
                    break
                feet.append(fitting[-1])
                start += len(fitting[-1].bits)
            else:
                halves.append(tuple(feet))
        if len(halves) == 2 and len(sadr_bits) <= bahr.max_length:
            matches.append(MeterMatch(bahr=bahr.name, sadr_feet=halves[0], ajuz_feet=halves[1]))
    return tuple(matches)
