// This is a Rerun type definition for the SDK, not executable code.
// It is parsed by `re_types_builder` to generate the Rust, Python and C++ bindings.

/// A grouped table with row-major or column-major data.
///
/// Displayed by the DataTable view.
#[rerun::rerun_type]
#[docs(view_types = "DataTableView")]
#[rerun(state = "unstable")]
#[rerun(visualizer = "DataTable")]
#[rust(derive(PartialEq))]
pub struct DataTable {
    /// Row-major data, with one cell per schema column in each row.
    #[rerun(no_ui_edit)]
    #[rerun(optional)]
    pub rows: Option<Vec<rerun::components::TableRow>>,

    /// Column-major data, with one buffer per schema column.
    ///
    /// Column lengths may differ; the view uses the longest column as its display height and leaves missing cells blank.
    /// Numeric buffers must be one-dimensional tensors.
    #[rerun(no_ui_edit)]
    #[rerun(optional)]
    pub cols: Option<Vec<rerun::components::TableColumn>>,

    /// Group and column definitions for either data layout.
    #[rerun(required)]
    pub schema: rerun::components::TableSchema,
}
