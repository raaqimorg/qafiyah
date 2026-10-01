from dataclasses import dataclass

from prosody.meter import MeterMatch, match_meters
from prosody.qafiyah import QafiyahTerm, analyze_qafiyah
from prosody.result import Err, Ok, ProsodyError, Result
from prosody.scan import Scan, scan_hemistich


@dataclass(frozen=True)
class BaytAnalysis:
    sadr: Scan
    ajuz: Scan
    meters: tuple[MeterMatch, ...]
    qafiyah: dict[QafiyahTerm, str]


def analyze_poem(hemistichs: list[str]) -> Result[tuple[BaytAnalysis, ...], ProsodyError]:
    if len(hemistichs) == 0:
        return Err(ProsodyError.EMPTY_POEM)
    paired = list(hemistichs)
    if len(paired) == 1:
        paired.append(paired[0])
    elif len(paired[1]) == 0:
        paired[1] = paired[0]
    if len(paired) % 2 != 0:
        paired.append(paired[-1])

    scans: list[Scan] = []
    for hemistich in paired:
        scanned = scan_hemistich(hemistich)
        if isinstance(scanned, Err):
            return scanned
        scans.append(scanned.value)

    analyses: list[BaytAnalysis] = []
    for i in range(0, len(paired), 2):
        qafiyah = analyze_qafiyah(scans[i + 1], paired[i + 1])
        if isinstance(qafiyah, Err):
            return qafiyah
        meters = match_meters(scans[i].bits, scans[i + 1].bits)
        analyses.append(BaytAnalysis(sadr=scans[i], ajuz=scans[i + 1], meters=meters, qafiyah=qafiyah.value))
    return Ok(tuple(analyses))
