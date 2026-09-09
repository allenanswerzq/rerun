// This is a Rerun type definition for the SDK, not executable code.
// It is parsed by `re_types_builder` to generate the Rust, Python and C++ bindings.

/// One dropdown option for a [`rerun::archetypes::DataTable`] column.
#[rerun::rerun_type]
#[rerun(state = "unstable")]
#[rust(derive(Default, PartialEq, Eq))]
pub struct TableColumnOption {
    /// Stable identity reported when this option is selected.
    pub id: String,

    /// Text displayed in the header and its menu.
    pub label: String,
}
