import json
from pathlib import Path
from typing import Any

import pytest

from prosody.poem import BaytAnalysis, analyze_poem
from prosody.result import Err

FIXTURES = Path(__file__).parent / "fixtures"
NO_RESULT_LABEL = "لا نتيجة"


def _read(name: str) -> Any:
    return json.loads((FIXTURES / name).read_text(encoding="utf8"))


def _as_dart_output(verse_id: int, analysis: BaytAnalysis) -> dict[str, Any]:
    bahrs = [
        {
            "name": meter.bahr.label,
            "first": [foot.label for foot in meter.sadr_feet],
            "second": [foot.label for foot in meter.ajuz_feet],
        }
        for meter in analysis.meters
    ]
    return {
        "verseId": verse_id,
        "binary": [analysis.sadr.bits, analysis.ajuz.bits],
        "arowdi": [analysis.sadr.writing, analysis.ajuz.writing],
        "bahrs": bahrs if len(bahrs) > 0 else [{"name": NO_RESULT_LABEL, "first": [], "second": []}],
        "rhyme": {term.label: value for term, value in analysis.qafiyah.items()},
    }


FIXTURE = _read("sample-100-bayt.json")
GOLDEN = {entry["verseId"]: entry for entry in _read("dart-golden-100-bayt.json")}


def test_the_golden_file_covers_every_bayt_of_the_fixture() -> None:
    assert sorted(GOLDEN) == sorted(item["verseId"] for item in FIXTURE)


@pytest.mark.parametrize("item", FIXTURE, ids=[str(item["verseId"]) for item in FIXTURE])
def test_each_bayt_produces_exactly_what_the_dart_engine_produced(item: dict[str, Any]) -> None:
    poem = analyze_poem(item["content"].split("*"))
    assert not isinstance(poem, Err)
    assert _as_dart_output(item["verseId"], poem.value[0]) == GOLDEN[item["verseId"]]
