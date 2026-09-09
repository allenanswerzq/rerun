# DataTable requirements

Target: `re_view_datatable`, using `egui_table`.
Define UI behavior first; design the archetype and storage schema later.

## 1. Groups

A DataTable contains multiple groups.
Each group contains `primary_group`, an optional `sub_group`, and its columns.

## 2. Data layout

Support both row-major and column-major data with the same logical table behavior.
Column-major columns may have different lengths; missing cells are blank.

## 3. Sorting

Sort by one or multiple columns, each with an ascending or descending direction.
Preserve the specified priority order: the first sort column is primary, and subsequent columns break ties.
The sort scope can be the whole DataTable or a specific subgroup.
Subgroup sorting must not reorder other subgroups.

## 4. Selectable column headers

A column header can offer selectable text options such as `1d`, `3d`, and `1w`. <!-- NOLINT -->
Changing the selection updates the header text and emits an event identifying the group, column, and selected option.

## 5. Checkbox columns

A checkbox column contains only checkboxes in its body cells.
Clicking a checkbox changes its checked state and emits an event identifying the group, column, row, and new checked state.

## 6. Value-based colors

The UI colors numeric cells using a red/green scale based on their values.
Colors and range mapping are UI concerns, not fields in the DataTable archetype or schema.

## 7. Appearance

Use the [egui_table demo](../../egui_table/demo/src/table_demo.rs) as the visual reference.
Keep rows and padding compact, separators subtle, and group headers clear so more data fits on screen without sacrificing readability.
Automatically truncate long text with an ellipsis to fit the available width, without wrapping or increasing row height.

Header-change and checkbox events use explicitly typed `ViewEventKind` variants through `CommandSender::send_view_event`.
