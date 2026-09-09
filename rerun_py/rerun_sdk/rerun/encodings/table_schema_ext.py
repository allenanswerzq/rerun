from __future__ import annotations

from typing import TYPE_CHECKING

import pyarrow as pa
from attrs import asdict

if TYPE_CHECKING:
    from . import TableSchemaArrayLike, TableSortScope, TableSortScopeLike


class TableSchemaExt:
    """Extension for [TableSchema][rerun.encodings.TableSchema]."""

    @staticmethod
    def sort_scope__field_converter_override(data: TableSortScopeLike | None) -> TableSortScope | None:
        from . import TableSortScope

        return None if data is None else TableSortScope.auto(data)

    @staticmethod
    def native_to_pa_array_override(data: TableSchemaArrayLike, data_type: pa.DataType) -> pa.Array:
        from . import TableSchema

        if isinstance(data, TableSchema):
            data = [data]
        values = []
        for value in data:
            fields = asdict(value)
            fields["sort_scope"] = None if value.sort_scope is None else value.sort_scope.value
            values.append(fields)
        return pa.array(values, type=data_type)
