// This is a Rerun type definition for the SDK, not executable code.
// It is parsed by `re_types_builder` to generate the Rust, Python and C++ bindings.

/// One row of a [`rerun::archetypes::DataTable`], in schema column order.
///
/// Checkbox cells use `Number(0.0)` for unchecked and `Number(1.0)` for checked.
#[rerun::rerun_type]
#[rerun(state = "unstable")]
#[rust(derive(Default, PartialEq))]
pub struct TableRow {
    pub values: Vec<rerun::encodings::TableValue>,
}
