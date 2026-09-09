from __future__ import annotations

import numpy as np
import pytest
import rerun as rr
from rerun.archetypes import DataTable
from rerun.components import TableColumn, TableRow, TableSchema, TableSchemaBatch
from rerun.encodings import (
    TableBuffer,
    TableBufferBatch,
    TableColumnOption,
    TableColumnOptionBatch,
    TableColumnSchema,
    TableColumnSchemaBatch,
    TableGroup,
    TableGroupBatch,
    TableSchema as TableSchemaEncoding,
    TableSortScope,
    TableValue,
    TableValueBatch,
    TensorData,
)


def schema() -> TableSchema:
    return TableSchema([
        TableGroup(
            id="stocks",
            primary_group="Market",
            sub_group="Stocks",
            columns=[
                TableColumnSchema("name", "Name"),
                TableColumnSchema("checked", "", checkbox=True),
                TableColumnSchema(
                    "change",
                    "Change",
                    column_dropdown=[TableColumnOption("day", "1d"), TableColumnOption("week", "1w")],
                ),
            ],
        ),
    ])


def test_table_values_and_buffers() -> None:
    rr.set_strict_mode(True)
    assert TableValueBatch([1, -2.5, np.int64(7), "AAA", None, ""]).as_arrow_array().to_pylist() == [
        1.0,
        -2.5,
        7.0,
        "AAA",
        None,
        "",
    ]
    tensor = TensorData(array=np.array([2**64 - 1, 0], dtype=np.uint64))
    array = TableBufferBatch([TableBuffer(tensor), TableBuffer(["AAA", ""]), None, TableBuffer([])]).as_arrow_array()
    array.validate(full=True)
    assert array.to_pylist() == [
        {"shape": [2], "names": None, "buffer": [2**64 - 1, 0]},
        ["AAA", ""],
        None,
        [],
    ]
    with pytest.raises(TypeError):
        TableValue(object())
    with pytest.raises(TypeError):
        TableBuffer(["AAA", 1])


def test_table_schema_serialization() -> None:
    rr.set_strict_mode(True)
    value = schema()
    array = TableSchemaBatch(value).as_arrow_array()
    array.validate(full=True)
    decoded = array.to_pylist()[0]
    assert decoded["sort_scope"] is None
    assert decoded["sticky_columns"] is None
    group = decoded["groups"][0]
    assert group["id"] == "stocks"
    assert group["sub_group"] == "Stocks"
    assert group["columns"][1]["checkbox"] is True
    assert group["columns"][2]["column_dropdown"] == [{"id": "day", "label": "1d"}, {"id": "week", "label": "1w"}]
    for batch_type in [
        TableSchemaBatch,
        TableGroupBatch,
        TableColumnSchemaBatch,
        TableColumnOptionBatch,
        TableBufferBatch,
    ]:
        empty = batch_type([]).as_arrow_array()
        empty.validate(full=True)
        assert len(empty) == 0
    columns = (
        TableColumnSchemaBatch([
            TableColumnSchema("plain", "Plain"),
            TableColumnSchema("empty", "Empty", column_dropdown=[]),
        ])
        .as_arrow_array()
        .to_pylist()
    )
    assert columns[0]["column_dropdown"] is None
    assert columns[1]["column_dropdown"] == []


def test_table_schema_display_settings() -> None:
    rr.set_strict_mode(True)
    for schema_type in [TableSchema, TableSchemaEncoding]:
        for sort_scope in [TableSortScope.Table, TableSortScope.Subgroup, "subgroup"]:
            for sticky_columns in [0, 2, 2**32 - 1]:
                value = schema_type(schema().groups, sort_scope=sort_scope, sticky_columns=sticky_columns)
                assert value.sort_scope is TableSortScope.auto(sort_scope)
                array = TableSchemaBatch(value).as_arrow_array()
                array.validate(full=True)
                decoded = array.to_pylist()[0]
                assert decoded["sort_scope"] == TableSortScope.auto(sort_scope).value
                assert decoded["sticky_columns"] == sticky_columns
                table = DataTable(value)
                assert table.schema is not None
                assert table.schema.as_arrow_array().equals(array)
    with pytest.raises(ValueError):
        TableSchema([], sort_scope=0)


def test_data_table_both_layouts_and_clearing() -> None:
    rr.set_strict_mode(True)
    assert rr.DataTable is DataTable
    rows = [TableRow(["AAA", 1, -2.5]), TableRow(["BBB", 0, 1.25])]
    cols = [
        TableColumn(TableBuffer(["AAA", "BBB"])),
        TableColumn(TableBuffer(TensorData(array=np.array([1, 0], dtype=np.uint8)))),
        TableColumn(TableBuffer(TensorData(array=np.array([-2.5, 1.25], dtype=np.float64)))),
    ]
    row_major = DataTable(schema(), rows=rows)
    column_major = DataTable(schema(), cols=cols)
    assert row_major.cols is None and column_major.rows is None
    assert row_major.rows is not None and column_major.cols is not None
    assert row_major.schema is not None and column_major.schema is not None
    assert row_major.schema.as_arrow_array().equals(column_major.schema.as_arrow_array())
    values = column_major.cols.as_arrow_array().to_pylist()
    assert row_major.rows.as_arrow_array().to_pylist() == list(
        map(list, zip(values[0], values[1]["buffer"], values[2]["buffer"], strict=True))
    )

    switched = DataTable.from_fields(clear_unset=True, schema=schema(), cols=cols)
    assert switched.rows is not None and len(switched.rows.as_arrow_array()) == 0
    assert callable(DataTable.columns)
    assert len(DataTable.columns(schema=[schema()])) == 1
