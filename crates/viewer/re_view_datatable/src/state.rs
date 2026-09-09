use std::cmp::Ordering;
use std::collections::BTreeMap;

use rayon::prelude::*;
use re_sdk_types::encodings::{TableColumnOption, TableSortScope};

use crate::data::{CellValue, TableData};

#[derive(Clone, Copy, PartialEq, Eq, re_byte_size::SizeBytes)]
pub(crate) enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, re_byte_size::SizeBytes)]
pub(crate) struct SortKey {
    pub column: usize,
    pub direction: SortDirection,
}

#[derive(Default, re_byte_size::SizeBytes)]
struct SortOrder {
    keys: Vec<SortKey>,
    rows: Vec<usize>,
    dirty: bool,
}

impl SortOrder {
    fn release_rows(&mut self) {
        if self.rows.capacity() > 0 {
            self.rows = Vec::new();
            self.dirty = true;
        }
    }
}

#[derive(Default, re_byte_size::SizeBytes)]
pub(crate) struct TableUiState {
    selected_options: BTreeMap<usize, String>,
    checked: BTreeMap<(usize, usize), bool>,
    table_order: SortOrder,
    group_orders: Vec<SortOrder>,
}

impl TableUiState {
    pub fn selected_option<'a>(
        &self,
        table: &'a TableData,
        column: usize,
    ) -> Option<&'a TableColumnOption> {
        let options = table.column_options(column);
        self.selected_options
            .get(&column)
            .and_then(|selected| options.iter().find(|option| option.id.as_str() == selected))
            .or_else(|| options.first())
    }

    pub fn select_option(&mut self, table: &TableData, column: usize, option: &str) -> bool {
        if column >= table.num_columns()
            || self
                .selected_option(table, column)
                .is_some_and(|current| current.id.as_str() == option)
        {
            return false;
        }
        let options = table.column_options(column);
        if !options
            .iter()
            .any(|candidate| candidate.id.as_str() == option)
        {
            return false;
        }
        if options
            .first()
            .is_some_and(|default| default.id.as_str() == option)
        {
            self.selected_options.remove(&column);
        } else {
            self.selected_options.insert(column, option.to_owned());
        }
        true
    }

    pub fn set_checked(
        &mut self,
        table: &TableData,
        row: usize,
        column: usize,
        checked: bool,
    ) -> bool {
        if column >= table.num_columns() || !table.is_checkbox_column(column) {
            return false;
        }
        let Some(CellValue::Float(original)) = table.cell(row, column) else {
            return false;
        };
        let original = original == 1.0;
        if self
            .checked
            .get(&(row, column))
            .copied()
            .unwrap_or(original)
            == checked
        {
            return false;
        }
        if checked == original {
            self.checked.remove(&(row, column));
        } else {
            self.checked.insert((row, column), checked);
        }
        self.invalidate_sort_for_column(table, column);
        true
    }

    pub fn synchronize(&mut self, table: &TableData, schema_changed: bool) {
        if schema_changed {
            self.table_order = SortOrder::default();
            self.group_orders.clear();
            self.selected_options.clear();
        }
        self.checked.clear();
        self.group_orders
            .resize_with(table.schema.groups.len(), SortOrder::default);
        self.invalidate_sort();
    }

    fn invalidate_sort(&mut self) {
        self.table_order.dirty = true;
        for order in &mut self.group_orders {
            order.dirty = true;
        }
    }

    fn invalidate_sort_for_column(&mut self, table: &TableData, column: usize) {
        let order = match table.sort_scope() {
            TableSortScope::Table => &mut self.table_order,
            TableSortScope::Subgroup => &mut self.group_orders[table.column_group_index(column)],
        };
        if order.keys.iter().any(|key| key.column == column) {
            order.dirty = true;
        }
    }

    pub fn cycle_sort(&mut self, table: &TableData, column: usize, additive: bool) {
        let order = match table.sort_scope() {
            TableSortScope::Table => &mut self.table_order,
            TableSortScope::Subgroup => &mut self.group_orders[table.column_group_index(column)],
        };
        let position = order.keys.iter().position(|key| key.column == column);
        let direction = position.map(|index| order.keys[index].direction);
        let next = match direction {
            None => Some(SortDirection::Ascending),
            Some(SortDirection::Ascending) => Some(SortDirection::Descending),
            Some(SortDirection::Descending) => None,
        };
        if !additive {
            order.keys.clear();
        }
        if let Some(direction) = next {
            if let Some(key) = order.keys.iter_mut().find(|key| key.column == column) {
                key.direction = direction;
            } else {
                order.keys.push(SortKey { column, direction });
            }
        } else {
            order.keys.retain(|key| key.column != column);
        }
        order.dirty = true;
    }

    pub fn sort_key(&self, table: &TableData, column: usize) -> Option<(usize, SortDirection)> {
        let order = match table.sort_scope() {
            TableSortScope::Table => &self.table_order,
            TableSortScope::Subgroup => &self.group_orders[table.column_group_index(column)],
        };
        order
            .keys
            .iter()
            .enumerate()
            .find(|(_, key)| key.column == column)
            .map(|(index, key)| (index + 1, key.direction))
    }

    pub fn prepare(&mut self, table: &TableData) {
        match table.sort_scope() {
            TableSortScope::Table => {
                for order in &mut self.group_orders {
                    order.release_rows();
                }
                sort_order(table, &self.checked, &mut self.table_order);
            }
            TableSortScope::Subgroup => {
                self.table_order.release_rows();
                for order in &mut self.group_orders {
                    sort_order(table, &self.checked, order);
                }
            }
        }
    }

    pub fn source_row(&self, table: &TableData, display_row: usize, column: usize) -> usize {
        let order = match table.sort_scope() {
            TableSortScope::Table => &self.table_order,
            TableSortScope::Subgroup => &self.group_orders[table.column_group_index(column)],
        };
        order.rows.get(display_row).copied().unwrap_or(display_row)
    }

    pub fn row_is_checked(&self, table: &TableData, display_row: usize) -> bool {
        table.checkbox_columns().iter().copied().any(|column| {
            let source_row = self.source_row(table, display_row, column);
            self.cell(table, source_row, column)
                .and_then(CellValue::number)
                == Some(1.0)
        })
    }

    pub fn cell<'a>(
        &self,
        table: &'a TableData,
        row: usize,
        column: usize,
    ) -> Option<CellValue<'a>> {
        cell_value(table, &self.checked, row, column)
    }
}

fn cell_value<'a>(
    table: &'a TableData,
    checked: &BTreeMap<(usize, usize), bool>,
    row: usize,
    column: usize,
) -> Option<CellValue<'a>> {
    if !checked.is_empty()
        && table.is_checkbox_column(column)
        && let Some(value) = checked.get(&(row, column))
    {
        Some(CellValue::Float(if *value { 1.0 } else { 0.0 }))
    } else {
        table.cell(row, column)
    }
}

fn sort_order(table: &TableData, checked: &BTreeMap<(usize, usize), bool>, order: &mut SortOrder) {
    if !order.dirty {
        return;
    }
    order.dirty = false;
    order.rows.clear();
    if order.keys.is_empty() {
        return;
    }
    order.rows.extend(0..table.num_rows());
    let compare_rows = |left_row: &usize, right_row: &usize| {
        for key in &order.keys {
            let left = cell_value(table, checked, *left_row, key.column);
            let right = cell_value(table, checked, *right_row, key.column);
            let rank = |value: Option<CellValue<'_>>| match value {
                None => 2,
                Some(CellValue::Float(number)) if number.is_nan() => 1,
                _ => 0,
            };
            let comparison = match (rank(left), rank(right), left, right) {
                (0, 0, Some(left), Some(right)) => match key.direction {
                    SortDirection::Ascending => left.compare(right),
                    SortDirection::Descending => right.compare(left),
                },
                (left_rank, right_rank, _, _) => left_rank.cmp(&right_rank),
            };
            if comparison != Ordering::Equal {
                return comparison;
            }
        }
        left_row.cmp(right_row)
    };
    if order.rows.len() < 4_096 {
        order.rows.sort_unstable_by(compare_rows);
    } else {
        order.rows.par_sort_unstable_by(compare_rows);
    }
}
