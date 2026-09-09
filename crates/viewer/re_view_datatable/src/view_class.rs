use rayon::prelude::*;
use re_log_types::EntityPath;
use re_sdk_types::{View as _, ViewClassIdentifier};
use re_ui::{Help, UiExt as _};
use re_viewer_context::{
    IdentifiedViewSystem as _, MissingChunkReporter, SystemExecutionOutput, ViewClass,
    ViewClassLayoutPriority, ViewClassRegistryError, ViewClassUiOutput, ViewEvent, ViewQuery,
    ViewSpawnHeuristics, ViewState, ViewStateExt as _, ViewSystemExecutionError,
    ViewSystemRegistrator, ViewerContext, suggest_view_for_each_entity,
};

use crate::data::TableData;
use crate::state::TableUiState;
use crate::table_ui::table_ui;
use crate::visualizer_system::{DataTableVisualizer, SourceKey, TableSource};

#[derive(Default)]
pub struct DataTableView;

#[derive(Default)]
struct DataTableViewState {
    source: Option<SourceKey>,
    data: Option<TableData>,
    error: Option<String>,
    magnitudes: Vec<f64>,
    ui: TableUiState,
}

impl ViewState for DataTableViewState {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn heap_size_bytes(&self) -> u64 {
        use re_byte_size::SizeBytes as _;
        self.data.heap_size_bytes()
            + self.ui.heap_size_bytes()
            + self.magnitudes.heap_size_bytes()
            + self.error.heap_size_bytes()
    }
}

impl DataTableViewState {
    fn synchronize(&mut self, source: &TableSource) {
        if self.source.as_ref() == Some(&source.key) {
            return;
        }
        match source.decode() {
            Ok(data) => {
                let schema_changed = self
                    .source
                    .as_ref()
                    .is_none_or(|old| old.entity_path != source.key.entity_path)
                    || self
                        .data
                        .as_ref()
                        .is_none_or(|old| old.schema != data.schema);
                self.ui.synchronize(&data, schema_changed);
                self.magnitudes = vec![0.0; data.num_columns()];
                let heatmap_columns: Vec<_> = (0..data.num_columns())
                    .filter(|&column| data.is_heatmap_column(column))
                    .collect();
                match heatmap_columns.as_slice() {
                    [] => {}
                    [column] => {
                        self.magnitudes[*column] = data.column_max_abs(*column);
                    }
                    _ => {
                        let maxima: Vec<_> = heatmap_columns
                            .par_iter()
                            .map(|&column| data.column_max_abs(column))
                            .collect();
                        for (column, maximum) in std::iter::zip(heatmap_columns, maxima) {
                            self.magnitudes[column] = maximum;
                        }
                    }
                }
                self.data = Some(data);
                self.error = None;
            }
            Err(err) => {
                self.data = None;
                self.magnitudes.clear();
                self.ui = TableUiState::default();
                self.error = Some(err);
            }
        }
        self.source = Some(source.key.clone());
    }
}

impl ViewClass for DataTableView {
    fn identifier() -> ViewClassIdentifier {
        re_sdk_types::blueprint::views::DataTableView::identifier()
    }

    fn display_name(&self) -> &'static str {
        "Data table"
    }

    fn icon(&self) -> &'static re_ui::Icon {
        &re_ui::icons::VIEW_DATAFRAME
    }

    fn help(&self, _os: egui::os::OperatingSystem) -> Help {
        Help::new("Data table")
    }

    fn on_register(
        &self,
        registry: &mut ViewSystemRegistrator<'_>,
    ) -> Result<(), ViewClassRegistryError> {
        registry.register_visualizer::<DataTableVisualizer>()
    }

    fn new_state(&self) -> Box<dyn ViewState> {
        Box::<DataTableViewState>::default()
    }

    fn layout_priority(&self) -> ViewClassLayoutPriority {
        ViewClassLayoutPriority::Low
    }

    fn spawn_heuristics(
        &self,
        ctx: &ViewerContext<'_>,
        include: &dyn Fn(&EntityPath) -> bool,
    ) -> ViewSpawnHeuristics {
        suggest_view_for_each_entity::<DataTableVisualizer>(ctx, include)
    }

    fn ui(
        &self,
        ctx: &ViewerContext<'_>,
        _missing: &MissingChunkReporter,
        ui: &mut egui::Ui,
        state: &mut dyn ViewState,
        query: &ViewQuery<'_>,
        output: SystemExecutionOutput,
    ) -> Result<ViewClassUiOutput, ViewSystemExecutionError> {
        let state = state.downcast_mut::<DataTableViewState>()?;
        let sources = output
            .visualizer_data_or_default::<Vec<TableSource>>(DataTableVisualizer::identifier())?;
        let [source] = sources.as_slice() else {
            *state = DataTableViewState::default();
            if sources.is_empty() {
                ui.weak("No DataTable at the selected time.");
            } else {
                ui.error_label("Select one DataTable entity per view.");
            }
            return Ok(ViewClassUiOutput::default());
        };
        state.synchronize(source);
        if let Some(err) = &state.error {
            ui.error_label(err);
            return Ok(ViewClassUiOutput::default());
        }
        if let Some(data) = &state.data {
            let id = egui::Id::new(("data_table", query.view_id, &source.key.entity_path));
            for kind in table_ui(
                ui,
                id,
                data,
                &state.magnitudes,
                &mut state.ui,
                &source.key.entity_path,
            ) {
                ctx.command_sender().send_view_event(ViewEvent::new(
                    ctx.recording().store_id().clone(),
                    query.view_id,
                    kind,
                ));
            }
        }
        Ok(ViewClassUiOutput::default())
    }
}
