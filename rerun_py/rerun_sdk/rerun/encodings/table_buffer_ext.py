from __future__ import annotations

from collections.abc import Sequence
from typing import TYPE_CHECKING

import pyarrow as pa

from .._unions import build_dense_union

if TYPE_CHECKING:
    from . import TableBufferArrayLike, TableBufferLike, TensorData


class TableBufferExt:
    """Extension for [TableBuffer][rerun.encodings.TableBuffer]."""

    @staticmethod
    def inner__field_converter_override(value: TableBufferLike) -> TensorData | list[str]:
        from . import TableBuffer, TensorData

        if isinstance(value, TableBuffer):
            return value.inner
        if isinstance(value, TensorData):
            return value
        if isinstance(value, Sequence) and not isinstance(value, str):
            if all(isinstance(item, str) for item in value):
                return list(value)
        raise TypeError("A TableBuffer must contain TensorData or a sequence of strings")

    @staticmethod
    def native_to_pa_array_override(data: TableBufferArrayLike, data_type: pa.DataType) -> pa.Array:
        from . import TableBuffer, TensorData, TensorDataBatch

        if isinstance(data, (TableBuffer, TensorData)) or (
            isinstance(data, Sequence) and len(data) > 0 and all(isinstance(item, str) for item in data)
        ):
            data = [data]

        arrays = []
        for value in data:
            if value is None:
                arrays.append(build_dense_union(data_type, "_null_markers", pa.nulls(1)))
                continue
            if not isinstance(value, TableBuffer):
                value = TableBuffer(value)
            if isinstance(value.inner, TensorData):
                child = TensorDataBatch(value.inner).as_arrow_array()
                arrays.append(build_dense_union(data_type, "Numeric", child))
            else:
                child = pa.array([value.inner], type=data_type.field(2).type)
                arrays.append(build_dense_union(data_type, "Text", child))
        return pa.concat_arrays(arrays) if arrays else pa.nulls(0, type=data_type)
