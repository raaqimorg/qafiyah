from dataclasses import dataclass
from enum import Enum
from typing import Generic, TypeVar

T = TypeVar("T")
E = TypeVar("E")


@dataclass(frozen=True)
class Ok(Generic[T]):
    value: T


@dataclass(frozen=True)
class Err(Generic[E]):
    error: E


Result = Ok[T] | Err[E]


class ProsodyError(Enum):
    EMPTY_POEM = "empty_poem"
    EMPTY_HEMISTICH = "empty_hemistich"
    HEMISTICH_TOO_SHORT_FOR_QAFIYAH = "hemistich_too_short_for_qafiyah"
