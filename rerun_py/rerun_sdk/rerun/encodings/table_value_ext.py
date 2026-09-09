from __future__ import annotations

from numbers import Real


class TableValueExt:
    """Extension for [TableValue][rerun.encodings.TableValue]."""

    @staticmethod
    def inner__field_converter_override(value: float | str) -> float | str:
        if isinstance(value, str):
            return value
        if isinstance(value, Real):
            return float(value)
        raise TypeError("A TableValue must be a number or text")
