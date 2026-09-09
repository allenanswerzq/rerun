// This is a Rerun type definition for the SDK, not executable code.
// It is parsed by `re_types_builder` to generate the Rust, Python and C++ bindings.

/// The ordered groups and columns of a [`rerun::archetypes::DataTable`].
///
/// Group IDs must be unique within the table, and column IDs must be unique within their group.
/// Metadata and values are independent components; log compatible schema and data together when changing the structure.
#[rerun::rerun_type]
#[rerun(state = "unstable")]
#[rust(derive(Default, PartialEq))]
pub struct TableSchema {
    pub schema: rerun::encodings::TableSchema,
}
