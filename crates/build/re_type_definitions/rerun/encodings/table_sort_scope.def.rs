// This is a Rerun type definition for the SDK, not executable code.
// It is parsed by `re_types_builder` to generate the Rust, Python and C++ bindings.

/// How sorting reorders rows in a [`rerun::archetypes::DataTable`].
#[rerun::rerun_type]
#[repr(u8)]
#[rerun(state = "unstable")]
#[rust(arrow_opt)]
#[rust(derive(Copy, PartialEq, Eq))]
pub enum TableSortScope {
    /// Sort all columns together.
    #[default]
    Table = 1,

    /// Sort each schema group independently.
    Subgroup = 2,
}
