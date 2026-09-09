// This is a Rerun type definition for the SDK, not executable code.
// It is parsed by `re_types_builder` to generate the Rust, Python and C++ bindings.

/// A group of adjacent columns in a [`rerun::archetypes::DataTable`].
#[rerun::rerun_type]
#[rerun(state = "unstable")]
#[rust(derive(Default, PartialEq))]
pub struct TableGroup {
    /// Stable identity for this group, independent of its labels.
    pub id: String,

    /// Primary group header text.
    pub primary_group: String,

    /// Optional subgroup header text.
    pub sub_group: Option<String>,

    /// Columns in display and data order.
    pub columns: Vec<rerun::encodings::TableColumnSchema>,
}
