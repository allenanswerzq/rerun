from __future__ import annotations

from itertools import accumulate, chain
from typing import TYPE_CHECKING

import pyarrow as pa

if TYPE_CHECKING:
    from . import TableRowArrayLike


class TableRowExt:
    """Extension for [TableRow][rerun.components.TableRow]."""

    @staticmethod
    def native_to_pa_array_override(data: TableRowArrayLike, data_type: pa.DataType) -> pa.Array:
        from ..encodings import TableValueBatch
        from . import TableRow

        rows = [data] if isinstance(data, TableRow) else list(data)
        offsets = pa.array([0, *accumulate(len(row.values) for row in rows)], type=pa.int32())
        values = TableValueBatch(list(chain.from_iterable(row.values for row in rows))).as_arrow_array()
        return pa.ListArray.from_arrays(offsets, values, type=data_type)
