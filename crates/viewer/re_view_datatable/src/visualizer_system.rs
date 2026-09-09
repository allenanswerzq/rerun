use arrow::array::ArrayRef;
use re_chunk::{ChunkId, RowId};
use re_chunk_store::LatestAtQuery;
use re_log_types::EntityPath;
use re_log_types::hash::Hash64;
use re_sdk_types::archetypes::DataTable;
use re_sdk_types::components::{TableColumn, TableRow, TableSchema};
use re_sdk_types::{Archetype as _, FromArrow as _};
use re_view::{ComponentMappingError, DataResultQuery as _};
use re_viewer_context::{
    IdentifiedViewSystem, ViewContext, ViewContextCollection, ViewQuery, ViewSystemExecutionError,
    VisualizerExecutionOutput, VisualizerQueryInfo, VisualizerSystem,
};

use crate::data::TableData;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SourceKey {
    pub entity_path: EntityPath,
    components: [Option<(ChunkId, Option<RowId>)>; 3],
    mappings: Hash64,
    error: Option<String>,
}

#[derive(Clone)]
pub(crate) struct TableSource {
    pub key: SourceKey,
    schema: Option<ArrayRef>,
    rows: Option<ArrayRef>,
    cols: Option<ArrayRef>,
}

impl TableSource {
    pub fn decode(&self) -> Result<TableData, String> {
        if let Some(err) = &self.key.error {
            return Err(err.clone());
        }
        let schema = self
            .schema
            .as_ref()
            .ok_or("No DataTable schema at the selected time.")?;
        let mut schemas =
            TableSchema::from_arrow(schema.as_ref()).map_err(|err| err.to_string())?;
        if schemas.len() != 1 {
            return Err("A DataTable needs exactly one schema.".into());
        }
        let rows = self
            .rows
            .as_ref()
            .map(|array| TableRow::from_arrow(array.as_ref()))
            .transpose()
            .map_err(|err| err.to_string())?;
        let cols = self
            .cols
            .as_ref()
            .map(|array| TableColumn::from_arrow(array.as_ref()))
            .transpose()
            .map_err(|err| err.to_string())?;
        TableData::new(schemas.remove(0), rows, cols)
    }
}

#[derive(Default)]
pub(crate) struct DataTableVisualizer;

impl IdentifiedViewSystem for DataTableVisualizer {
    fn identifier() -> re_viewer_context::ViewSystemIdentifier {
        "DataTable".into()
    }
}

impl VisualizerSystem for DataTableVisualizer {
    fn visualizer_query_info(
        &self,
        _options: &re_viewer_context::AppOptions,
    ) -> VisualizerQueryInfo {
        VisualizerQueryInfo::single_required_component::<TableSchema>(
            &DataTable::descriptor_schema(),
            &DataTable::all_components(),
        )
    }

    fn execute(
        &self,
        ctx: &ViewContext<'_>,
        view_query: &ViewQuery<'_>,
        _systems: &ViewContextCollection,
    ) -> Result<VisualizerExecutionOutput, ViewSystemExecutionError> {
        let query = LatestAtQuery::new(view_query.timeline, view_query.latest_at);
        let mut sources = Vec::new();
        for (result, instruction) in view_query.iter_visualizer_instruction_for(Self::identifier())
        {
            let results = result.latest_at_with_blueprint_resolved_data::<DataTable>(
                ctx,
                &query,
                Some(instruction),
            );
            let mut error = None;
            let mut read = |component| {
                let unit = match optional_component(results.get_unit_chunk(component, true)) {
                    Ok(unit) => unit,
                    Err(err) => {
                        error.get_or_insert(err);
                        None
                    }
                };
                let key = unit.as_ref().map(|unit| (unit.id(), unit.row_id()));
                let array = unit.and_then(|unit| unit.component_batch_raw(component));
                (key, array)
            };
            let (schema_key, schema) = read(DataTable::descriptor_schema().component);
            let (rows_key, rows) = read(DataTable::descriptor_rows().component);
            let (cols_key, cols) = read(DataTable::descriptor_cols().component);
            sources.push(TableSource {
                key: SourceKey {
                    entity_path: result.entity_path.clone(),
                    components: [schema_key, rows_key, cols_key],
                    mappings: Hash64::hash(&instruction.component_mappings),
                    error,
                },
                schema,
                rows,
                cols,
            });
        }
        Ok(VisualizerExecutionOutput::default().with_visualizer_data(sources))
    }
}

fn optional_component<T>(
    result: Result<Option<T>, ComponentMappingError>,
) -> Result<Option<T>, String> {
    match result {
        Ok(value) => Ok(value),
        Err(
            ComponentMappingError::ComponentNotPresentOnEntity { .. }
            | ComponentMappingError::NoComponentDataForQuery(_),
        ) => Ok(None),
        Err(err) => Err(err.to_string()),
    }
}
