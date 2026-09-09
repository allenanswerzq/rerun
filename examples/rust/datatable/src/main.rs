//! A large column-major `DataTable` with host-side checkbox and dropdown event handling.

use std::{cell::RefCell, rc::Rc};

use rand::{Rng as _, SeedableRng as _, rngs::SmallRng};
use rerun::blueprint::{
    Blueprint, BlueprintPanel, SelectionPanel, TimePanel, components::PanelState,
};
use rerun::components::TableColumn;
use rerun::encodings::{
    TableBuffer, TableColumnOption, TableColumnSchema, TableGroup, TableSchema, TableSortScope,
    TensorData,
};
use rerun::external::{eframe, re_log, re_log_channel, re_memory, re_viewer, tokio};

#[global_allocator]
static GLOBAL: re_memory::AccountingAllocator<mimalloc::MiMalloc> =
    re_memory::AccountingAllocator::new(mimalloc::MiMalloc);

const APPLICATION_ID: &str = "rerun_example_datatable";
const TABLE_ENTITY: &str = "instruments";
const NUM_ROWS: u32 = 100_000;
const NUM_METRICS: u32 = 24;
const SEED: u64 = 42;
const METRICS_PER_GROUP: u32 = 8;
const HORIZONS: [(&str, &str, f64); 3] = [
    ("day", "1d", 1.0),
    ("three_days", "3d", 3.0), // NOLINT
    ("week", "1w", 7.0),
];
const SECTORS: [&str; 8] = [
    "Technology",
    "Energy",
    "Healthcare",
    "Finance",
    "Industrials",
    "Utilities",
    "Materials",
    "Consumer",
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    re_log::setup_logging();
    let main_thread_token = rerun::MainThreadToken::i_promise_i_am_on_the_main_thread();
    re_viewer::run_native_app(
        main_thread_token,
        Box::new(move |cc| Ok(Box::new(create_app(main_thread_token, cc)?))),
        None,
    )?;
    Ok(())
}

fn blueprint() -> Blueprint {
    Blueprint::auto()
        .with_blueprint_panel(BlueprintPanel::from_state(PanelState::Collapsed))
        .with_selection_panel(SelectionPanel::from_state(PanelState::Collapsed))
        .with_time_panel(TimePanel::new().with_state(PanelState::Collapsed))
}

fn create_app(
    main_thread_token: rerun::MainThreadToken,
    cc: &eframe::CreationContext<'_>,
) -> Result<re_viewer::App, Box<dyn std::error::Error + Send + Sync>> {
    let (sender, receiver) = re_log_channel::log_channel(re_log_channel::LogSource::Sdk);
    let recording = rerun::RecordingStreamBuilder::new(APPLICATION_ID).buffered()?;
    let store_id = recording
        .store_info()
        .ok_or("The example recording is disabled")?
        .store_id
        .clone();
    recording.set_sink(Box::new(rerun::sink::CallbackSink::new(move |messages| {
        for message in messages {
            if sender.send(message.clone().into()).is_err() {
                return;
            }
        }
    })));
    let table = fake_table();
    recording.log_static(
        TABLE_ENTITY,
        &rerun::DataTable::new(table.schema.clone()).with_cols(table.cols.iter().cloned()),
    )?;
    recording.log(
        "events",
        &rerun::TextLog::new(format!(
            "Loaded {NUM_ROWS} instruments and {NUM_METRICS} signals; seed {SEED}",
        )),
    )?;
    blueprint().send(&recording, Default::default())?;

    let table = RefCell::new(table);
    let startup_options = re_viewer::StartupOptions {
        persist_state: false,
        on_view_event: Some(Rc::new(move |event| {
            if event.store_id != store_id {
                return;
            }
            if let Err(err) = table.borrow_mut().handle_event(&recording, &event.kind) {
                re_log::error!("Failed to handle the table event: {err}");
            }
        })),
        ..Default::default()
    };
    let mut app = re_viewer::App::new(
        main_thread_token,
        re_viewer::build_info(),
        re_viewer::AppEnvironment::Custom("DataTable events".into()),
        startup_options,
        cc,
        None,
        re_viewer::AsyncRuntimeHandle::from_current_tokio_runtime_or_wasmbindgen()?,
    );
    app.add_log_receiver(receiver);
    Ok(app)
}

struct DemoTable {
    schema: TableSchema,
    cols: Vec<TableColumn>,
    selected: Vec<u8>,
}

impl DemoTable {
    fn handle_event(
        &mut self,
        recording: &rerun::RecordingStream,
        kind: &re_viewer::ViewEventKind,
    ) -> anyhow::Result<()> {
        let message = match kind {
            re_viewer::ViewEventKind::DataTableCheckboxChanged {
                entity_path,
                group_id,
                column_id,
                row_index,
                checked,
            } if entity_path == &TABLE_ENTITY.into()
                && group_id == "instruments"
                && column_id == "selected"
                && *row_index < u64::from(NUM_ROWS) =>
            {
                self.selected[*row_index as usize] = u8::from(*checked);
                format!("SIM{row_index:06}: selected={checked}")
            }
            re_viewer::ViewEventKind::DataTableColumnChanged {
                entity_path,
                group_id,
                column_id,
                option_id,
            } if entity_path == &TABLE_ENTITY.into() => {
                let Some(&(_, horizon, days)) = HORIZONS.iter().find(|(id, _, _)| *id == option_id)
                else {
                    return Ok(());
                };
                let Some(model) = self.schema.groups.iter().skip(1).position(|group| {
                    group.id.as_str() == group_id && group.columns[0].id.as_str() == column_id
                }) else {
                    return Ok(());
                };
                let start = model as u32 * METRICS_PER_GROUP;
                for metric in start..NUM_METRICS.min(start + METRICS_PER_GROUP) {
                    self.cols[3 + metric as usize] = signal_column(metric, days);
                }
                self.cols[0] = TableColumn(TableBuffer::Numeric(TensorData::from(
                    self.selected.clone(),
                )));
                recording.log_static(
                    TABLE_ENTITY,
                    &rerun::DataTable::update_fields().with_cols(self.cols.iter().cloned()),
                )?;
                format!("{group_id}: regenerated {NUM_ROWS} rows for {horizon}")
            }
            _ => return Ok(()),
        };
        re_log::info!("{message}");
        recording.log("events", &rerun::TextLog::new(message))?;
        Ok(())
    }
}

fn fake_table() -> DemoTable {
    let mut groups = vec![TableGroup {
        id: "instruments".into(),
        primary_group: "Instruments".into(),
        columns: vec![
            TableColumnSchema {
                id: "selected".into(),
                label: "".into(),
                checkbox: Some(true),
                ..Default::default()
            },
            TableColumnSchema {
                id: "symbol".into(),
                label: "Symbol".into(),
                ..Default::default()
            },
            TableColumnSchema {
                id: "sector".into(),
                label: "Sector".into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    }];
    let selected: Vec<u8> = (0..NUM_ROWS).map(|row| u8::from(row % 11 == 0)).collect();
    let mut cols = vec![
        TableColumn(TableBuffer::Numeric(TensorData::from(selected.clone()))),
        TableColumn(TableBuffer::Text(
            (0..NUM_ROWS)
                .map(|row| format!("SIM{row:06}").into())
                .collect(),
        )),
        TableColumn(TableBuffer::Text(
            (0..NUM_ROWS)
                .map(|row| SECTORS[row as usize % SECTORS.len()].into())
                .collect(),
        )),
    ];

    for start in (0..NUM_METRICS).step_by(METRICS_PER_GROUP as usize) {
        let model = start / METRICS_PER_GROUP + 1;
        let mut group = TableGroup {
            id: format!("model_{model}").into(),
            primary_group: "Signals".into(),
            sub_group: Some(format!("Model {model}").into()),
            ..Default::default()
        };
        for metric in start..NUM_METRICS.min(start + METRICS_PER_GROUP) {
            let number = metric + 1;
            group.columns.push(TableColumnSchema {
                id: format!("signal_{number}").into(),
                label: format!("Signal {number:02}").into(),
                column_dropdown: (metric == start).then(|| {
                    HORIZONS
                        .iter()
                        .map(|&(id, label, _)| TableColumnOption {
                            id: id.into(),
                            label: label.into(),
                        })
                        .collect()
                }),
                ..Default::default()
            });
            cols.push(signal_column(metric, HORIZONS[0].2));
        }
        groups.push(group);
    }

    DemoTable {
        schema: TableSchema {
            groups,
            sort_scope: Some(TableSortScope::Table),
            sticky_columns: Some(2),
        },
        cols,
        selected,
    }
}

fn signal_column(metric: u32, days: f64) -> TableColumn {
    TableColumn(TableBuffer::Numeric(TensorData::from(signal_values(
        metric, days, NUM_ROWS,
    ))))
}

fn signal_values(metric: u32, days: f64, rows: u32) -> Vec<f64> {
    let mut rng = SmallRng::seed_from_u64(SEED + u64::from(metric));
    let values: Vec<_> = (0..rows)
        .map(|row| {
            let phase = f64::from(row) * 0.17 / days + f64::from(metric);
            let spread = f64::from((row * 37 + metric * 53) % 101) / 100.0;
            let magnitude = 2.0
                + 30.0 * phase.sin().abs()
                + 180.0 * spread.powi(2)
                + rng.random_range(0.0..10.0);
            let sign = if (row / 3 + metric).is_multiple_of(2) {
                1.0
            } else {
                -1.0
            };
            sign * days.sqrt() * magnitude
        })
        .collect();
    re_log::debug_assert!(
        rows < 12 || {
            let sample = &values[..12];
            let minimum = sample.iter().copied().fold(f64::INFINITY, f64::min);
            let maximum = sample.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            minimum < -50.0 && maximum > 50.0 && maximum - minimum > 150.0
        }
    );
    values
}
