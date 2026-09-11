// This is a Rerun type definition for the SDK, not executable code.
// It is parsed by `re_types_builder` to generate the Rust, Python and C++ bindings.

/// The groups and display settings of a [`rerun::archetypes::DataTable`].
#[rerun::rerun_type]
#[rerun(state = "unstable")]
#[rust(derive(Default, PartialEq))]
pub struct TableSchema {
    /// Groups in display and data order.
    pub groups: Vec<rerun::encodings::TableGroup>,

    /// Scope of header-click sorting; defaults to the whole table.
    pub sort_scope: Option<rerun::encodings::TableSortScope>,

    /// Number of leading columns kept visible while scrolling; defaults to one.
    pub sticky_columns: Option<u32>,

    /// One-based summary rows that start visual row groups.
    ///
    /// For example, `[1, 10, 20]` creates groups `[1, 10)`, `[10, 20)`, and `[20, …)`.
    /// Groups are collapsed by default, leaving each summary row visible with a disclosure control.
    /// Sorting orders whole groups by their summary values and therefore uses table-wide scope.
    pub row_groups: Option<Vec<u64>>,
}
