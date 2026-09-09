// This is a Rerun type definition for the SDK, not executable code.
// It is parsed by `re_types_builder` to generate the Rust, Python and C++ bindings.

/// Header and presentation metadata for one [`rerun::archetypes::DataTable`] column.
#[rerun::rerun_type]
#[rerun(state = "unstable")]
#[rust(derive(Default, PartialEq))]
pub struct TableColumnSchema {
    /// Stable identity within the group, independent of the displayed header.
    pub id: String,

    /// Header text when no selectable options are provided.
    pub label: String,

    /// Optional column dropdown choices, with unique option IDs.
    ///
    /// The first option is selected by default.
    /// An absent or empty list uses `label` as a plain text header.
    pub column_dropdown: Option<Vec<rerun::encodings::TableColumnOption>>,

    /// Render body cells as checkboxes instead of text when true.
    ///
    /// Defaults to false; checkbox data must contain only numeric zero or one.
    pub checkbox: Option<bool>,

    /// Color numeric cell backgrounds by their magnitude and sign when true.
    ///
    /// Defaults to false and has no effect on text or checkbox cells.
    pub heatmap: Option<bool>,
}
