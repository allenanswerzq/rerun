from __future__ import annotations

from typing import TYPE_CHECKING

import pyarrow as pa
from attrs import asdict

if TYPE_CHECKING:
    from . import TableGroupArrayLike


class TableGroupExt:
    """Extension for [TableGroup][rerun.encodings.TableGroup]."""

    @staticmethod
    def native_to_pa_array_override(data: TableGroupArrayLike, data_type: pa.DataType) -> pa.Array:
        from . import TableGroup

        if isinstance(data, TableGroup):
            data = [data]
        return pa.array([asdict(value) for value in data], type=data_type)
