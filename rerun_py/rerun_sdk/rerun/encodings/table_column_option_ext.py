from __future__ import annotations

from typing import TYPE_CHECKING

import pyarrow as pa
from attrs import asdict

if TYPE_CHECKING:
    from . import TableColumnOptionArrayLike


class TableColumnOptionExt:
    """Extension for [TableColumnOption][rerun.encodings.TableColumnOption]."""

    @staticmethod
    def native_to_pa_array_override(data: TableColumnOptionArrayLike, data_type: pa.DataType) -> pa.Array:
        from . import TableColumnOption

        if isinstance(data, TableColumnOption):
            data = [data]
        return pa.array([asdict(value) for value in data], type=data_type)
