use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::ops::Range;

use re_sdk_types::components::{TableColumn, TableRow, TableSchema};
use re_sdk_types::encodings::{
    TableBuffer, TableColumnOption, TableColumnSchema, TableGroup, TableSortScope, TableValue,
    TensorBuffer,
};

#[derive(re_byte_size::SizeBytes)]
pub(crate) enum TableValues {
    Rows(Vec<TableRow>),
    Columns(Vec<TableColumn>),
}

#[derive(re_byte_size::SizeBytes)]
pub(crate) struct TableData {
    /// Group and column definitions, including dropdown and checkbox settings.
    pub schema: TableSchema,

    /// Maps each flat column to (group index, column index within that group).
    pub columns: Vec<(usize, usize)>,

    /// Each group's column range `[start, end)` in the flat column list.
    pub group_ranges: Vec<(usize, usize)>,

    /// Flat indices of checkbox columns, used by row rendering.
    checkbox_columns: Vec<usize>,

    /// Number of values in each flat column.
    column_lengths: Vec<usize>,

    /// Cell data kept in its original row-major or column-major layout.
    values: TableValues,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum CellValue<'a> {
    Float(f64),
    Text(&'a str),
}

impl CellValue<'_> {
    pub fn number(self) -> Option<f64> {
        match self {
            Self::Float(value) => Some(value),
            Self::Text(_) => None,
        }
    }

    pub fn compare(self, other: Self) -> Ordering {
        match (self, other) {
            (Self::Float(left), Self::Float(right)) => left.total_cmp(&right),
            (Self::Text(left), Self::Text(right)) => left.cmp(right),
            (Self::Text(_), Self::Float(_)) => Ordering::Greater,
            (Self::Float(_), Self::Text(_)) => Ordering::Less,
        }
    }
}

impl std::fmt::Display for CellValue<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Float(value) => value.fmt(formatter),
            Self::Text(value) => value.fmt(formatter),
        }
    }
}

impl TableData {
    pub fn new(
        schema: TableSchema,
        rows: Option<Vec<TableRow>>,
        cols: Option<Vec<TableColumn>>,
    ) -> Result<Self, String> {
        let mut columns = Vec::new();
        let mut group_ranges = Vec::new();
        let mut group_ids = BTreeSet::new();
        for (group_index, group) in schema.groups.iter().enumerate() {
            if group.id.is_empty() || !group_ids.insert(group.id.clone()) {
                return Err("Table group IDs must be nonempty and unique.".into());
            }
            let start = columns.len();
            let mut column_ids = BTreeSet::new();
            for (column_index, column) in group.columns.iter().enumerate() {
                if column.id.is_empty() || !column_ids.insert(column.id.clone()) {
                    return Err(format!(
                        "Column IDs in group '{}' must be nonempty and unique.",
                        group.id
                    ));
                }
                let mut option_ids = BTreeSet::new();
                for option in column.column_dropdown.iter().flatten() {
                    if option.id.is_empty() || !option_ids.insert(option.id.clone()) {
                        return Err(format!(
                            "Dropdown IDs in column '{}' must be nonempty and unique.",
                            column.id
                        ));
                    }
                }
                columns.push((group_index, column_index));
            }
            group_ranges.push((start, columns.len()));
        }

        let rows = rows.filter(|rows| !rows.is_empty());
        let cols = cols.filter(|cols| !cols.is_empty());
        let (values, column_lengths) = match (rows, cols) {
            (Some(_), Some(_)) => {
                return Err(
                    "Supply rows or cols, not both; clear the previous layout when switching."
                        .into(),
                );
            }
            (Some(rows), None) => {
                if rows.iter().any(|row| row.0.len() != columns.len()) {
                    return Err("Each row must contain exactly one cell per schema column.".into());
                }
                let column_lengths = vec![rows.len(); columns.len()];
                (TableValues::Rows(rows), column_lengths)
            }
            (None, Some(cols)) => {
                if cols.len() != columns.len() {
                    return Err("The number of column buffers must match the schema.".into());
                }
                let column_lengths = cols.iter().map(column_len).collect::<Result<Vec<_>, _>>()?;
                (TableValues::Columns(cols), column_lengths)
            }
            (None, None) => (TableValues::Rows(Vec::new()), vec![0; columns.len()]),
        };

        let checkbox_columns = columns
            .iter()
            .enumerate()
            .filter_map(|(column, &(group_index, column_index))| {
                schema.groups[group_index].columns[column_index]
                    .checkbox
                    .unwrap_or(false)
                    .then_some(column)
            })
            .collect();
        let table = Self {
            schema,
            columns,
            group_ranges,
            checkbox_columns,
            column_lengths,
            values,
        };
        for column in 0..table.columns.len() {
            if table.column_schema(column).checkbox.unwrap_or(false) {
                for row in 0..table.column_num_rows(column) {
                    // TODO(copilot): consider to use boolean here
                    if !matches!(
                        table.cell(row, column).and_then(CellValue::number),
                        Some(0.0 | 1.0)
                    ) {
                        return Err(format!(
                            "Checkbox column '{}' must contain only numeric zero or one.",
                            table.column_schema(column).id
                        ));
                    }
                }
            }
        }
        Ok(table)
    }

    pub fn column_schema(&self, column: usize) -> &TableColumnSchema {
        let (group_index, column_index) = self.columns[column];
        &self.schema.groups[group_index].columns[column_index]
    }

    pub fn num_columns(&self) -> usize {
        self.columns.len()
    }

    pub fn column_group_index(&self, column: usize) -> usize {
        self.columns[column].0
    }

    pub fn column_group(&self, column: usize) -> &TableGroup {
        &self.schema.groups[self.column_group_index(column)]
    }

    pub fn is_checkbox_column(&self, column: usize) -> bool {
        self.column_schema(column).checkbox.unwrap_or(false)
    }

    pub fn is_heatmap_column(&self, column: usize) -> bool {
        let schema = self.column_schema(column);
        schema.heatmap.unwrap_or(false) && !schema.checkbox.unwrap_or(false)
    }

    pub fn checkbox_columns(&self) -> &[usize] {
        &self.checkbox_columns
    }

    pub fn column_options(&self, column: usize) -> &[TableColumnOption] {
        self.column_schema(column)
            .column_dropdown
            .as_deref()
            .unwrap_or_default()
    }

    pub fn sort_scope(&self) -> TableSortScope {
        self.schema.sort_scope.unwrap_or_default()
    }

    pub fn num_sticky_columns(&self) -> usize {
        (self.schema.sticky_columns.unwrap_or(1) as usize).min(self.num_columns())
    }

    pub fn has_subgroups(&self) -> bool {
        self.schema
            .groups
            .iter()
            .any(|group| group.sub_group.is_some())
    }

    pub fn subgroup_ranges(&self) -> impl Iterator<Item = Range<usize>> + '_ {
        self.group_ranges
            .iter()
            .filter(|(start, end)| start < end)
            .map(|&(start, end)| start..end)
    }

    pub fn primary_group_ranges(&self) -> Vec<Range<usize>> {
        let mut ranges: Vec<Range<usize>> = Vec::new();
        let mut previous_primary = None;
        for range in self.subgroup_ranges() {
            let primary = &self.column_group(range.start).primary_group;
            if previous_primary == Some(primary) {
                if let Some(previous) = ranges.last_mut() {
                    previous.end = range.end;
                }
            } else {
                ranges.push(range);
            }
            previous_primary = Some(primary);
        }
        ranges
    }

    pub fn num_rows(&self) -> usize {
        self.column_lengths.iter().copied().max().unwrap_or(0)
    }

    pub fn column_num_rows(&self, column: usize) -> usize {
        self.column_lengths[column]
    }

    pub fn column_max_abs(&self, column: usize) -> f64 {
        fn maximum(values: impl Iterator<Item = f64>) -> f64 {
            values
                .filter(|value| value.is_finite())
                .map(f64::abs)
                .fold(0.0, f64::max)
        }

        match &self.values {
            TableValues::Rows(rows) => {
                maximum(rows.iter().filter_map(|row| match &row.0[column] {
                    TableValue::Number(value) => Some(*value),
                    TableValue::Text(_) => None,
                }))
            }
            TableValues::Columns(cols) => match &cols[column].0 {
                TableBuffer::Text(_) => 0.0,
                TableBuffer::Numeric(tensor) => match &tensor.buffer {
                    TensorBuffer::U8(values) => {
                        maximum(values.iter().map(|&value| f64::from(value)))
                    }
                    TensorBuffer::U16(values) => {
                        maximum(values.iter().map(|&value| f64::from(value)))
                    }
                    TensorBuffer::U32(values) => {
                        maximum(values.iter().map(|&value| f64::from(value)))
                    }
                    TensorBuffer::U64(values) => maximum(values.iter().map(|&value| value as f64)),
                    TensorBuffer::I8(values) => {
                        maximum(values.iter().map(|&value| f64::from(value)))
                    }
                    TensorBuffer::I16(values) => {
                        maximum(values.iter().map(|&value| f64::from(value)))
                    }
                    TensorBuffer::I32(values) => {
                        maximum(values.iter().map(|&value| f64::from(value)))
                    }
                    TensorBuffer::I64(values) => maximum(values.iter().map(|&value| value as f64)),
                    TensorBuffer::F16(values) => maximum(values.iter().map(|value| value.to_f64())),
                    TensorBuffer::F32(values) => {
                        maximum(values.iter().map(|&value| f64::from(value)))
                    }
                    TensorBuffer::F64(values) => maximum(values.iter().copied()),
                },
            },
        }
    }

    pub fn cell(&self, row: usize, column: usize) -> Option<CellValue<'_>> {
        if row >= self.column_num_rows(column) {
            return None;
        }
        Some(match &self.values {
            TableValues::Rows(rows) => match &rows[row].0[column] {
                TableValue::Number(value) => CellValue::Float(*value),
                TableValue::Text(value) => CellValue::Text(value.as_str()),
            },
            TableValues::Columns(cols) => match &cols[column].0 {
                TableBuffer::Text(values) => CellValue::Text(values[row].as_str()),
                TableBuffer::Numeric(tensor) => CellValue::Float(match &tensor.buffer {
                    TensorBuffer::U8(values) => f64::from(values[row]),
                    TensorBuffer::U16(values) => f64::from(values[row]),
                    TensorBuffer::U32(values) => f64::from(values[row]),
                    TensorBuffer::U64(values) => values[row] as f64,
                    TensorBuffer::I8(values) => f64::from(values[row]),
                    TensorBuffer::I16(values) => f64::from(values[row]),
                    TensorBuffer::I32(values) => f64::from(values[row]),
                    TensorBuffer::I64(values) => values[row] as f64,
                    TensorBuffer::F16(values) => values[row].to_f64(),
                    TensorBuffer::F32(values) => f64::from(values[row]),
                    TensorBuffer::F64(values) => values[row],
                }),
            },
        })
    }
}

fn column_len(column: &TableColumn) -> Result<usize, String> {
    match &column.0 {
        TableBuffer::Text(values) => Ok(values.len()),
        TableBuffer::Numeric(tensor) => {
            let length = match &tensor.buffer {
                TensorBuffer::U8(values) => values.len(),
                TensorBuffer::U16(values) => values.len(),
                TensorBuffer::U32(values) => values.len(),
                TensorBuffer::U64(values) => values.len(),
                TensorBuffer::I8(values) => values.len(),
                TensorBuffer::I16(values) => values.len(),
                TensorBuffer::I32(values) => values.len(),
                TensorBuffer::I64(values) => values.len(),
                TensorBuffer::F16(values) => values.len(),
                TensorBuffer::F32(values) => values.len(),
                TensorBuffer::F64(values) => values.len(),
            };
            if tensor.shape.as_ref() != [length as u64] {
                return Err("Numeric columns must be one-dimensional tensors with a matching buffer length.".into());
            }
            Ok(length)
        }
    }
}
