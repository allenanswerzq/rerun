from __future__ import annotations

from typing import TYPE_CHECKING

import pyarrow as pa
from attrs import asdict

if TYPE_CHECKING:
    from . import TableColumnSchemaArrayLike


class TableColumnSchemaExt:
    """Extension for [TableColumnSchema][rerun.encodings.TableColumnSchema]."""

    @staticmethod
    def native_to_pa_array_override(data: TableColumnSchemaArrayLike, data_type: pa.DataType) -> pa.Array:
        from . import TableColumnSchema

        if isinstance(data, TableColumnSchema):
            data = [data]
        return pa.array([asdict(value) for value in data], type=data_type)
