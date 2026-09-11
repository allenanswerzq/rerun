use egui_kittest::kittest::Queryable as _;
use re_chunk::{RowId, TimePoint};
use re_sdk_types::archetypes::DataTable;
use re_sdk_types::components::{TableColumn, TableRow};
use re_sdk_types::encodings::{
    TableBuffer, TableColumnOption, TableColumnSchema, TableGroup, TableSchema, TableSortScope,
    TableValue, TensorData,
};
use re_test_context::TestContext;
use re_test_viewport::TestContextExt as _;
use re_view_datatable::DataTableView;
use re_viewer_context::{ViewClass as _, ViewEventKind};
use re_viewport_blueprint::ViewBlueprint;

fn schema() -> TableSchema {
    TableSchema {
        groups: vec![
            TableGroup {
                id: "stocks".into(),
                primary_group: "Consumer durables".into(),
                sub_group: Some("Motor vehicles".into()),
                columns: vec![
                    TableColumnSchema {
                        id: "checked".into(),
                        checkbox: Some(true),
                        ..Default::default()
                    },
                    TableColumnSchema {
                        id: "ticker".into(),
                        label: "Ticker".into(),
                        ..Default::default()
                    },
                    TableColumnSchema {
                        id: "return".into(),
                        label: "Return".into(),
                        heatmap: Some(true),
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
            },
            TableGroup {
                id: "commodities".into(),
                primary_group: "Commodities".into(),
                columns: vec![
                    TableColumnSchema {
                        id: "name".into(),
                        label: "Name".into(),
                        ..Default::default()
                    },
                    TableColumnSchema {
                        id: "price".into(),
                        label: "Price".into(),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        ],
        ..Default::default()
    }
}

fn columns() -> [TableColumn; 5] {
    [
        TableColumn(TableBuffer::Numeric(TensorData::from(vec![0_u8, 0, 1]))),
        TableColumn(TableBuffer::Text(
            ["AAA", "BBB", "CCC"].map(Into::into).to_vec(),
        )),
        TableColumn(TableBuffer::Numeric(TensorData::from(vec![
            2.25_f64, -4.5, 1.0,
        ]))),
        TableColumn(TableBuffer::Text(
            ["Oil with a deliberately long description", "Gold", "Copper"]
                .map(Into::into)
                .to_vec(),
        )),
        TableColumn(TableBuffer::Numeric(TensorData::from(vec![
            72.5_f64, 1940.0, 4.1,
        ]))),
    ]
}

fn column_table() -> DataTable {
    DataTable::new(schema()).with_cols(columns())
}

fn row_table() -> DataTable {
    let rows = [
        (
            0.0,
            "AAA",
            2.25,
            "Oil with a deliberately long description",
            72.5,
        ),
        (0.0, "BBB", -4.5, "Gold", 1940.0),
        (1.0, "CCC", 1.0, "Copper", 4.1),
    ]
    .map(|(checked, ticker, returns, name, price)| {
        TableRow(vec![
            TableValue::Number(checked),
            TableValue::Text(ticker.into()),
            TableValue::Number(returns),
            TableValue::Text(name.into()),
            TableValue::Number(price),
        ])
    });
    DataTable::new(schema()).with_rows(rows)
}

fn setup(data: &DataTable) -> (TestContext, re_viewer_context::ViewId) {
    let mut context = TestContext::new_with_view_class::<DataTableView>();
    context.log_entity("table", |builder| {
        builder.with_archetype(RowId::new(), TimePoint::STATIC, data)
    });
    let view_id = context.setup_viewport_blueprint(|_, blueprint| {
        blueprint.add_view_at_root(ViewBlueprint::new_with_root_wildcard(
            DataTableView::identifier(),
        ))
    });
    (context, view_id)
}

#[test]
fn datatable_snapshot() {
    let (context, view_id) = setup(&column_table());
    context
        .run_view_ui_and_save_snapshot(view_id, "datatable", egui::vec2(400.0, 320.0), None)
        .unwrap();
}

#[test]
fn row_major_and_ragged_columns_render() {
    let (context, view_id) = setup(&row_table());
    let mut harness = context
        .setup_kittest_for_rendering_ui(egui::vec2(600.0, 300.0))
        .build_ui(|ui| context.run_with_single_view(ui, view_id));
    harness.run();
    harness.get_by_label("CCC");
    harness.get_by_label(&format!("{}.00", re_format::format_f64(1940.0)));

    let ragged = column_table().with_cols([
        TableColumn(TableBuffer::Numeric(TensorData::from(vec![1_u8]))),
        columns()[1].clone(),
        TableColumn(TableBuffer::Numeric(TensorData::from(vec![-2.5_f64, 0.0]))),
        TableColumn(TableBuffer::Text(Vec::new())),
        TableColumn(TableBuffer::Numeric(TensorData::from(vec![42.0_f64]))),
    ]);
    let (context, view_id) = setup(&ragged);
    let mut harness = context
        .setup_kittest_for_rendering_ui(egui::vec2(600.0, 300.0))
        .build_ui(|ui| context.run_with_single_view(ui, view_id));
    harness.run();
    harness.get_by_label("CCC");
    harness.get_by_label("checked row 0");
    assert!(harness.query_by_label("checked row 1").is_none());
}

#[test]
fn sorting_and_controls_emit_typed_events() {
    let (context, view_id) = setup(&column_table());
    let mut harness = context
        .setup_kittest_for_rendering_ui(egui::vec2(600.0, 300.0))
        .build_ui(|ui| context.run_with_single_view(ui, view_id));
    harness.run();

    harness.get_by_label("Sort Return").click();
    harness.run();
    assert!(harness.get_by_label("BBB").rect().top() < harness.get_by_label("AAA").rect().top());

    harness.get_by_label("checked row 1").click();
    harness.run();
    assert_eq!(
        context.take_view_events()[0].kind,
        ViewEventKind::DataTableCheckboxChanged {
            entity_path: "table".into(),
            group_id: "stocks".into(),
            column_id: "checked".into(),
            row_index: 1,
            checked: true,
        }
    );

    harness.get_by_role(egui::accesskit::Role::ComboBox).click();
    harness.run();
    harness.get_by_label("1w").click();
    harness.run();
    assert_eq!(
        context.take_view_events()[0].kind,
        ViewEventKind::DataTableColumnChanged {
            entity_path: "table".into(),
            group_id: "stocks".into(),
            column_id: "return".into(),
            option_id: "week".into(),
        }
    );
}

#[test]
fn schema_controls_sort_scope_and_sticky_columns() {
    let data = column_table().with_schema(TableSchema {
        sort_scope: Some(TableSortScope::Subgroup),
        sticky_columns: Some(2),
        row_groups: Some(vec![1, 3].into()),
        ..schema()
    });
    let (context, view_id) = setup(&data);
    let mut harness = context
        .setup_kittest_for_rendering_ui(egui::vec2(240.0, 300.0))
        .build_ui(|ui| context.run_with_single_view(ui, view_id));
    harness.run();
    harness.get_by_label("Row 1");
    harness.get_by_label("Row 3");
    assert!(harness.query_by_label("Row 2").is_none());
    assert!(harness.query_by_label("BBB").is_none());

    harness.get_by_label("Sort Return").click();
    harness.run();
    assert!(
        harness.get_by_label("Row 3").rect().top() < harness.get_by_label("Row 1").rect().top()
    );

    harness.get_by_label("Expand row group 1").click();
    harness.run();
    harness.get_by_label("Collapse row group 1");
    harness.get_by_label("Row 2");
    harness.get_by_label("BBB");

    assert_eq!(
        harness.get_by_label("Gold").rect().top(),
        harness.get_by_label("BBB").rect().top()
    );

    harness.get_by_label("Sort Return").click();
    harness.run();
    assert!(
        harness.get_by_label("Row 1").rect().top() < harness.get_by_label("Row 3").rect().top()
    );
    harness.get_by_label("Sort Return").click();
    harness.run();
    assert!(
        harness.get_by_label("Row 1").rect().top() < harness.get_by_label("Row 3").rect().top()
    );
    harness.get_by_label("Sort Return").click();
    harness.run();
    assert!(
        harness.get_by_label("Row 3").rect().top() < harness.get_by_label("Row 1").rect().top()
    );

    let row_left = harness.get_by_label("Row 1").rect().left();
    let ticker_left = harness.get_by_label("AAA").rect().left();
    let commodity_left = harness.get_by_label("Gold").rect().left();
    harness.hover_at(egui::pos2(210.0, 180.0));
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(-40.0, 0.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
    assert_eq!(harness.get_by_label("Row 1").rect().left(), row_left);
    assert_eq!(harness.get_by_label("AAA").rect().left(), ticker_left);
    assert!(harness.get_by_label("Gold").rect().left() < commodity_left);

    harness.get_by_label("Collapse row group 1").click();
    harness.run();
    assert!(harness.query_by_label("BBB").is_none());
}

#[test]
fn heatmap_is_opt_in_and_numbers_are_not_clipped() {
    let data = DataTable::new(TableSchema {
        groups: vec![TableGroup {
            id: "numbers".into(),
            primary_group: "Numbers".into(),
            columns: vec![
                TableColumnSchema {
                    id: "colored".into(),
                    label: "Colored".into(),
                    heatmap: Some(true),
                    ..Default::default()
                },
                TableColumnSchema {
                    id: "plain".into(),
                    label: "Plain".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }],
        ..Default::default()
    })
    .with_cols([
        TableColumn(TableBuffer::Numeric(TensorData::from(vec![
            -987_654_321.25,
            4.5,
        ]))),
        TableColumn(TableBuffer::Numeric(TensorData::from(vec![72.5, 1940.0]))),
    ]);
    let (context, view_id) = setup(&data);
    let mut harness = context
        .setup_kittest_for_rendering_ui(egui::vec2(600.0, 240.0))
        .build_ui(|ui| {
            ui.ctx().set_theme(egui::Theme::Light);
            context.run_with_single_view(ui, view_id);
        });
    harness.run();

    let large = re_format::FloatFormatOptions::DEFAULT_f64
        .with_decimals(2)
        .with_strip_trailing_zeros(false)
        .format(-987_654_321.25);
    let large_rect = harness.get_by_label(&large).rect();
    let shape = harness
        .output()
        .shapes
        .iter()
        .find(|shape| {
            matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == large)
        })
        .expect("The full number must be painted");
    let egui::Shape::Text(text) = &shape.shape else {
        unreachable!()
    };
    assert!(!text.galley.elided);

    let background_at = |center| {
        harness
            .output()
            .shapes
            .iter()
            .rev()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect)
                    if rect.rect.contains(center) && (rect.rect.height() - 19.0).abs() < 1.0 =>
                {
                    Some(rect.fill)
                }
                _ => None,
            })
    };
    let colored = background_at(large_rect.center()).expect("Heatmap column must be colored");
    assert!(colored.r() > colored.g());
    assert!(background_at(harness.get_by_label("72.50").rect().center()).is_none());
}
