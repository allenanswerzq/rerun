// This is a Rerun type definition for the SDK, not executable code.
// It is parsed by `re_types_builder` to generate the Rust, Python and C++ bindings.

/// A numeric tensor or an array of text values for table data.
///
/// The numeric variant preserves its shape and numeric element type without
/// wrapping each element in a [`rerun::encodings::TableValue`].
#[rerun::rerun_type]
#[repr(i8)]
#[rerun(state = "unstable")]
#[rust(arrow_opt)]
#[rust(derive(PartialEq))]
pub enum TableBuffer {
    /// Numeric values with a shape and a typed buffer.
    Numeric(rerun::encodings::TensorData) = 1,

    /// A one-dimensional array of UTF-8 text values.
    Text(Vec<String>) = 2,
}
