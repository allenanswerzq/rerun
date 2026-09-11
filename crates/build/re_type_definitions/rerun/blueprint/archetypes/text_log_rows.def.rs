// This is a Rerun type definition for the SDK, not executable code.
// It is parsed by `re_types_builder` to generate the Rust, Python and C++ bindings.

/// Configuration of the text log rows.
#[rerun::rerun_type]
#[rerun(scope = "blueprint")]
#[rerun(state = "unstable")]
pub struct TextLogRows {
    /// Log levels to display.
    ///
    /// Defaults to showing all logged levels.
    #[rerun(optional)]
    pub filter_by_log_level: Option<Vec<rerun::components::TextLogLevel>>,

    /// Whether to show the newest log entries first.
    ///
    /// Defaults to oldest first.
    #[rerun(optional)]
    pub newest_first: Option<rerun::blueprint::components::Enabled>,
}
