use re_sdk_types::archetypes::DataTable;
use re_sdk_types::components::{TableColumn, TableRow};
use re_sdk_types::encodings::{
    TableBuffer, TableColumnOption, TableColumnSchema, TableGroup, TableSchema, TableSortScope,
    TableValue, TensorData,
};
use re_sdk_types::{Archetype as _, AsComponents as _, FromArrow as _, ToArrow as _};

fn schema() -> TableSchema {
    TableSchema {
        groups: vec![TableGroup {
            id: "stocks".into(),
            primary_group: "Market".into(),
            sub_group: Some("Stocks".into()),
            columns: vec![
                TableColumnSchema {
                    id: "ticker".into(),
                    label: "Ticker".into(),
                    ..Default::default()
                },
                TableColumnSchema {
                    id: "checked".into(),
                    checkbox: Some(true),
                    ..Default::default()
                },
                TableColumnSchema {
                    id: "change".into(),
                    label: "Change".into(),
                    column_dropdown: Some(vec![
                        TableColumnOption {
                            id: "day".into(),
                            label: "1d".into(),
                        },
                        TableColumnOption {
                            id: "week".into(),
                            label: "1w".into(),
                        },
                    ]),
                    ..Default::default()
                },
            ],
        }],
        ..Default::default()
    }
}

#[test]
fn roundtrip_both_layouts() {
    let rows = [
        TableRow(vec![
            TableValue::Text("AAA".into()),
            TableValue::Number(1.0),
            TableValue::Number(-2.5),
        ]),
        TableRow(vec![
            TableValue::Text("BBB".into()),
            TableValue::Number(0.0),
            TableValue::Number(1.25),
        ]),
    ];
    let columns = [
        TableColumn(TableBuffer::Text(vec!["AAA".into(), "BBB".into()])),
        TableColumn(TableBuffer::Numeric(TensorData::from(vec![1_u8, 0]))),
        TableColumn(TableBuffer::Numeric(TensorData::from(vec![-2.5_f64, 1.25]))),
    ];

    let row_major = DataTable::new(schema()).with_rows(rows);
    let column_major = DataTable::new(schema()).with_cols(columns.clone());
    let switched = DataTable::clear_fields()
        .with_schema(schema())
        .with_cols(columns);
    assert!(row_major.cols.is_none());
    assert!(column_major.rows.is_none());
    assert!(switched.rows.as_ref().unwrap().array.is_empty());

    for expected in [row_major, column_major, switched] {
        let serialized = expected.to_arrow().unwrap();
        assert_eq!(DataTable::from_arrow(serialized).unwrap(), expected);
    }
}

#[test]
fn schema_preserves_dropdown_order_and_defaults() {
    let expected = schema();
    let serialized = TableSchema::to_arrow([&expected]).unwrap();
    let decoded = TableSchema::from_arrow(&serialized).unwrap();
    assert_eq!(decoded, [expected]);

    assert_eq!(decoded[0].sort_scope, None);
    assert_eq!(decoded[0].sticky_columns, None);
    let dropdown = decoded[0].groups[0].columns[2]
        .column_dropdown
        .as_ref()
        .unwrap();
    assert_eq!(dropdown[0].id.as_str(), "day");
    assert_eq!(dropdown[1].id.as_str(), "week");

    let columns = [
        TableColumnSchema::default(),
        TableColumnSchema {
            column_dropdown: Some(Vec::new()),
            ..Default::default()
        },
    ];
    let serialized = TableColumnSchema::to_arrow(&columns).unwrap();
    assert_eq!(TableColumnSchema::from_arrow(&serialized).unwrap(), columns);
}

#[test]
fn schema_preserves_display_settings() {
    for sort_scope in [TableSortScope::Table, TableSortScope::Subgroup] {
        for sticky_columns in [0, 2, u32::MAX] {
            let expected = DataTable::new(TableSchema {
                sort_scope: Some(sort_scope),
                sticky_columns: Some(sticky_columns),
                ..schema()
            });
            let serialized = expected.to_arrow().unwrap();
            assert_eq!(DataTable::from_arrow(serialized).unwrap(), expected);
        }
    }
}
