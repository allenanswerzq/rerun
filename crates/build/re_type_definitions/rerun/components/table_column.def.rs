// This is a Rerun type definition for the SDK, not executable code.
// It is parsed by `re_types_builder` to generate the Rust, Python and C++ bindings.

/// One column of a [`rerun::archetypes::DataTable`].
///
/// Numeric data must be a one-dimensional tensor.
/// Checkbox columns use numeric zero for unchecked and one for checked.
#[rerun::rerun_type]
#[rerun(state = "unstable")]
#[rust(derive(PartialEq))]
pub struct TableColumn {
    pub values: rerun::encodings::TableBuffer,
}
