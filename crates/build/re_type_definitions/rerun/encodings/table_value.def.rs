// This is a Rerun type definition for the SDK, not executable code.
// It is parsed by `re_types_builder` to generate the Rust, Python and C++ bindings.

/// A number or text value, suitable for a cell in a mixed-type table row.
///
/// Numbers use 64-bit floating point.
/// For homogeneous numeric columns that retain their element type, use
/// [`rerun::encodings::TableBuffer`] instead.
#[rerun::rerun_type]
#[repr(i8)]
#[rerun(state = "unstable")]
#[rust(arrow_opt)]
#[rust(derive(PartialEq))]
pub enum TableValue {
    /// A 64-bit floating-point number.
    Number(f64) = 1,

    /// A UTF-8 text value.
    Text(String) = 2,
}
