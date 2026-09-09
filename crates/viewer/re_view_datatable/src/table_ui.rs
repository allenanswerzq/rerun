use std::collections::BTreeMap;

use egui::{Align, Align2, Id, Layout, Rect, RichText, Sense, Ui, UiBuilder, Vec2};
use egui_table::{HeaderCellInfo, HeaderRow, TableDelegate};
use re_log_types::EntityPath;
use re_ui::UiExt as _;
use re_viewer_context::ViewEventKind;

use crate::data::{CellValue, TableData};
use crate::state::{SortDirection, TableUiState};

const CELL_PADDING: f32 = 4.0;
const HEADER_PADDING: f32 = 2.0;
const HEADER_CONTROL_WIDTH: f32 = 14.0;
const HEADER_SEPARATOR_WIDTH: f32 = 2.0;
const MIN_HEADER_LABEL_WIDTH: f32 = 16.0;
const SORT_MARKER_WIDTH: f32 = 14.0;
const WIDTH_SAMPLES: usize = 129;
const MAX_HEATMAP_MIX: f32 = 0.4;

enum Action {
    Sort {
        column: usize,
        additive: bool,
    },
    Select {
        column: usize,
        option: String,
    },
    Check {
        row: usize,
        column: usize,
        checked: bool,
    },
}

pub(crate) fn table_ui(
    ui: &mut Ui,
    id: Id,
    data: &TableData,
    magnitudes: &[f64],
    state: &mut TableUiState,
    entity_path: &EntityPath,
) -> Vec<ViewEventKind> {
    if data.num_columns() == 0 {
        ui.weak("No columns in the table schema.");
        return Vec::new();
    }
    state.prepare(data);
    let headers = header_rows(data);
    let num_rows = data.num_rows();
    let mut delegate = Delegate {
        data,
        magnitudes,
        state,
        id,
        header_rows: headers.len(),
        group_headers: GroupHeaders::default(),
        checked_row: None,
        numeric_column_grew: false,
        actions: Vec::new(),
    };
    ui.scope_builder(UiBuilder::new().id(Id::new(id.value())), |ui| {
        apply_table_style(ui);
        ui.painter().rect_filled(
            ui.available_rect_before_wrap(),
            0.0,
            ui.visuals().panel_fill,
        );
        let table = egui_table::Table::new().id_salt("table");
        let columns = column_definitions(ui, data, table.get_id(ui), num_rows);
        let table_rect = ui.available_rect_before_wrap();
        let row_gutter_right = table_rect.left() + columns[0].current;
        let response = table
            .columns(columns)
            .num_rows(num_rows as u64)
            .num_sticky_cols(1 + data.num_sticky_columns())
            .headers(headers)
            .show(ui, &mut delegate);
        delegate.group_headers.show(ui, id);
        paint_row_gutter_divider(ui, row_gutter_right, response.rect.y_range());
        if delegate.numeric_column_grew {
            ui.ctx().request_discard("DataTable numeric column grew");
        }
    });
    if !delegate.actions.is_empty() {
        ui.ctx().request_repaint();
    }
    delegate
        .actions
        .into_iter()
        .filter_map(|action| action.apply(data, state, entity_path))
        .collect()
}

impl Action {
    fn apply(
        self,
        data: &TableData,
        state: &mut TableUiState,
        entity_path: &EntityPath,
    ) -> Option<ViewEventKind> {
        match self {
            Self::Sort { column, additive } => {
                state.cycle_sort(data, column, additive);
                None
            }
            Self::Select { column, option } => {
                if !state.select_option(data, column, &option) {
                    return None;
                }
                Some(ViewEventKind::DataTableColumnChanged {
                    entity_path: entity_path.clone(),
                    group_id: data.column_group(column).id.to_string(),
                    column_id: data.column_schema(column).id.to_string(),
                    option_id: option,
                })
            }
            Self::Check {
                row,
                column,
                checked,
            } => {
                if !state.set_checked(data, row, column, checked) {
                    return None;
                }
                Some(ViewEventKind::DataTableCheckboxChanged {
                    entity_path: entity_path.clone(),
                    group_id: data.column_group(column).id.to_string(),
                    column_id: data.column_schema(column).id.to_string(),
                    row_index: row as u64,
                    checked,
                })
            }
        }
    }
}

fn header_rows(data: &TableData) -> Vec<HeaderRow> {
    let mut rows = vec![HeaderRow {
        height: 24.0,
        groups: data
            .primary_group_ranges()
            .into_iter()
            .map(|range| range.start + 1..range.end + 1)
            .collect(),
    }];
    if data.has_subgroups() {
        rows.push(HeaderRow {
            height: 20.0,
            groups: data
                .subgroup_ranges()
                .map(|range| range.start + 1..range.end + 1)
                .collect(),
        });
    }
    rows.push(HeaderRow::new(24.0));
    rows
}

fn column_definitions(
    ui: &Ui,
    data: &TableData,
    table_id: Id,
    num_rows: usize,
) -> Vec<egui_table::Column> {
    let mut table_state = egui_table::TableState::load(ui.ctx(), table_id).unwrap_or_default();
    let digits = num_rows.max(1).ilog10() as usize + 1;
    let row_width = (text_width(ui, "8".repeat(digits)) + 2.0 * CELL_PADDING)
        .ceil()
        .max(24.0);
    let row_id = table_id.with("row_numbers");
    table_state.col_widths.insert(row_id, row_width);
    let mut columns = vec![
        egui_table::Column::new(row_width)
            .range(row_width..=row_width)
            .resizable(false)
            .id(row_id),
    ];
    for column in 0..data.num_columns() {
        let column_id = table_id.with((
            &data.column_group(column).id,
            &data.column_schema(column).id,
        ));
        let minimum = if data.column_options(column).is_empty() {
            24.0
        } else {
            DropdownHeaderLayout::minimum_width()
        };
        let width = table_state
            .col_widths
            .get(&column_id)
            .copied()
            .unwrap_or_else(|| initial_column_width(ui, data, column))
            .max(minimum);
        columns.push(
            egui_table::Column::new(width)
                .range(minimum..=f32::INFINITY)
                .id(column_id),
        );
    }
    fit_group_titles(ui, data, table_id, &mut columns);
    table_state.col_widths.retain(|id, _| {
        columns
            .iter()
            .enumerate()
            .any(|(index, column)| column.id_for(index) == *id)
    });
    for (index, column) in columns.iter().enumerate() {
        table_state
            .col_widths
            .insert(column.id_for(index), column.current);
    }
    table_state.store(ui.ctx(), table_id);
    columns
}

fn fit_group_titles(ui: &Ui, data: &TableData, table_id: Id, columns: &mut [egui_table::Column]) {
    let groups = std::iter::chain(
        data.primary_group_ranges()
            .into_iter()
            .map(|range| (range, true)),
        data.subgroup_ranges().map(|range| (range, false)),
    );
    let measured: Vec<_> = groups
        .filter_map(|(range, primary)| {
            let group = data.column_group(range.start);
            let label = if primary {
                group.primary_group.as_str()
            } else {
                group.sub_group.as_ref().map_or("", |label| label.as_str())
            };
            if label.is_empty() {
                return None;
            }
            let width =
                (text_width(ui, group_header_text(label, primary)) + 2.0 * CELL_PADDING).ceil();
            Some((range, width))
        })
        .collect();
    let metrics = measured
        .iter()
        .fold(Id::new("group_title_metrics"), |metrics, (range, width)| {
            metrics.with((range, width.to_bits()))
        });
    let metrics_id = table_id.with("group_title_metrics");
    let changed = ui.ctx().data_mut(|cache| {
        if cache.get_temp::<Id>(metrics_id) == Some(metrics) {
            false
        } else {
            cache.insert_temp(metrics_id, metrics);
            true
        }
    });
    if !changed {
        return;
    }
    let mut available =
        (ui.available_width() - columns.iter().map(|column| column.current).sum::<f32>()).max(0.0);
    for (range, required) in measured {
        let current: f32 = columns[range.start + 1..=range.end]
            .iter()
            .map(|column| column.current)
            .sum();
        let Some(column) = range
            .clone()
            .find(|&column| matches!(data.cell(0, column), Some(CellValue::Text(_))))
        else {
            continue;
        };
        let extra = (required - current).max(0.0).min(available);
        columns[column + 1].current += extra;
        available -= extra;
    }
}

fn initial_column_width(ui: &Ui, data: &TableData, column: usize) -> f32 {
    if data.is_checkbox_column(column) {
        return 24.0;
    }
    let character_allowance = text_width(ui, "0");
    let step = data.column_num_rows(column).div_ceil(WIDTH_SAMPLES).max(1);
    let mut widest_number = 0.0_f32;
    let mut widths: Vec<_> = (0..data.column_num_rows(column))
        .step_by(step)
        .filter_map(|row| data.cell(row, column))
        .map(|value| {
            let width = text_width(ui, cell_text(value));
            if value.number().is_some() {
                widest_number = widest_number.max(width);
                width + character_allowance
            } else {
                width
            }
        })
        .collect();
    widths.sort_unstable_by(f32::total_cmp);
    let median = if widths.is_empty() {
        0.0
    } else {
        (widths[(widths.len() - 1) / 2] + widths[widths.len() / 2]) * 0.5
    };
    let content_width = median.min(240.0).max(widest_number + character_allowance);
    let options = data.column_options(column);
    let header_width = if options.is_empty() {
        text_width(
            ui,
            RichText::new(data.column_schema(column).label.as_str()).size(13.0),
        ) + character_allowance
            + 2.0 * CELL_PADDING
    } else {
        options
            .iter()
            .map(|option| text_width(ui, RichText::new(option.label.as_str()).size(13.0)))
            .fold(0.0, f32::max)
            + DropdownHeaderLayout::controls_width()
            + 2.0 * HEADER_PADDING
    };
    (content_width + 2.0 * CELL_PADDING)
        .max(header_width)
        .ceil()
        .max(24.0)
}

fn text_width(ui: &Ui, text: impl Into<egui::WidgetText>) -> f32 {
    text.into()
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::FontSelection::Default,
        )
        .size()
        .x
}

fn apply_table_style(ui: &mut Ui) {
    let tokens = ui.tokens();
    let style = ui.style_mut();
    style.spacing.item_spacing = Vec2::new(4.0, 0.0);
    style.spacing.interact_size.y = 18.0;
    style.spacing.scroll.floating = false;
    style.spacing.scroll.foreground_color = false;
    style.spacing.scroll.bar_width = 10.0;
    style.spacing.scroll.handle_min_length = 24.0;
    style.wrap_mode = Some(egui::TextWrapMode::Truncate);
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.hovered.bg_stroke =
        egui::Stroke::new(1.0, tokens.table_interaction_hovered_bg_stroke);
    style.visuals.widgets.active.bg_stroke =
        egui::Stroke::new(1.0, tokens.table_interaction_active_bg_stroke);
}

struct Delegate<'a> {
    data: &'a TableData,
    magnitudes: &'a [f64],
    state: &'a TableUiState,
    id: Id,
    header_rows: usize,
    group_headers: GroupHeaders<'a>,
    checked_row: Option<(u64, bool)>,
    numeric_column_grew: bool,
    actions: Vec<Action>,
}

impl TableDelegate for Delegate<'_> {
    fn header_cell_ui(&mut self, ui: &mut Ui, info: &HeaderCellInfo) {
        let visible = ui.max_rect().intersect(ui.clip_rect());
        if visible.width() < 1.0 / ui.pixels_per_point()
            || !visible.is_positive()
            || info.col_range.start > self.data.num_columns()
        {
            return;
        }
        let content_id = info.table_id.with((
            "data_table_header",
            info.row_nr,
            info.col_range.start,
            info.col_range.end,
        ));
        ui.scope_builder(UiBuilder::new().id(content_id), |ui| {
            self.header_cell_contents_ui(ui, info, visible);
        });
    }

    fn row_ui(&mut self, ui: &mut Ui, row: u64) {
        let fill = if self.row_is_checked(row) {
            Some(selected_row_fill(ui))
        } else if row % 2 == 1 {
            Some(ui.visuals().faint_bg_color)
        } else {
            None
        };
        if let Some(fill) = fill {
            ui.painter().rect_filled(ui.max_rect(), 0.0, fill);
        }
    }

    fn cell_ui(&mut self, ui: &mut Ui, info: &egui_table::CellInfo) {
        let rect = ui.max_rect();
        let visible = rect.intersect(ui.clip_rect());
        if visible.width() < 1.0 / ui.pixels_per_point() || !visible.is_positive() {
            return;
        }
        let source_row = info
            .col_nr
            .checked_sub(1)
            .map_or(info.row_nr as usize, |column| {
                self.state
                    .source_row(self.data, info.row_nr as usize, column)
            });
        let content_id = Id::new((
            "data_table_cell",
            info.table_id.value(),
            source_row,
            info.col_nr,
        ));
        self.cell_contents_ui(ui, info, rect, source_row, content_id);
    }

    fn default_row_height(&self) -> f32 {
        20.0
    }
}

impl Delegate<'_> {
    fn header_cell_contents_ui(&mut self, ui: &mut Ui, info: &HeaderCellInfo, visible: Rect) {
        paint_header_frame(ui, info.row_nr + 1 == self.header_rows);
        let Some(column) = info.col_range.start.checked_sub(1) else {
            if info.row_nr + 1 == self.header_rows {
                row_number_ui(ui, ui.id().with("row_number"), "#");
            }
            return;
        };
        if info.row_nr + 1 < self.header_rows {
            let group = self.data.column_group(column);
            let label = if info.row_nr == 0 {
                group.primary_group.as_str()
            } else {
                group.sub_group.as_ref().map_or("", |label| label.as_str())
            };
            self.group_headers.collect_fragment(info, label, visible);
        } else {
            self.column_header_ui(ui, column);
        }
    }

    fn cell_contents_ui(
        &mut self,
        ui: &mut Ui,
        info: &egui_table::CellInfo,
        rect: Rect,
        source_row: usize,
        content_id: Id,
    ) {
        ui.painter().vline(
            rect.right() - 0.5,
            rect.y_range(),
            egui::Stroke::new(0.5, ui.tokens().table_header_stroke_color),
        );
        let Some(column) = info.col_nr.checked_sub(1) else {
            let number = (info.row_nr + 1).to_string();
            let response = row_number_ui(ui, content_id, &number);
            response.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Label,
                    ui.is_enabled(),
                    format!("Row {number}"),
                )
            });
            return;
        };
        let row = source_row;
        let Some(value) = self.state.cell(self.data, row, column) else {
            return;
        };
        if self.data.is_checkbox_column(column) {
            self.checkbox_ui(ui, content_id, row, column, value.number() == Some(1.0));
        } else {
            if !self.row_is_checked(info.row_nr)
                && self.data.is_heatmap_column(column)
                && let Some(number) = value.number()
            {
                paint_numeric_background(ui, rect, info.row_nr, number, self.magnitudes[column]);
            }
            let response = value_ui(ui, content_id, value);
            if value.number().is_some() {
                let required_width = (response.rect.width() + 2.0 * CELL_PADDING).ceil();
                ui.set_min_width(required_width);
                if required_width > rect.width() + 0.5 {
                    self.numeric_column_grew = true;
                }
            }
        }
    }
}

impl Delegate<'_> {
    fn row_is_checked(&mut self, row: u64) -> bool {
        if let Some((cached_row, checked)) = self.checked_row
            && cached_row == row
        {
            return checked;
        }
        let checked = self.state.row_is_checked(self.data, row as usize);
        self.checked_row = Some((row, checked));
        checked
    }

    fn column_header_ui(&mut self, ui: &mut Ui, column: usize) {
        ui.style_mut().interaction.selectable_labels = false;
        if let Some(current) = self.state.selected_option(self.data, column) {
            let label_width = self
                .data
                .column_options(column)
                .iter()
                .map(|option| text_width(ui, RichText::new(option.label.as_str()).size(13.0)))
                .fold(0.0, f32::max);
            let Some(layout) = DropdownHeaderLayout::new(ui.max_rect(), label_width) else {
                return;
            };
            let mut label_ui = ui.new_child(
                UiBuilder::new()
                    .max_rect(layout.label)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            label_ui.shrink_clip_rect(layout.label);
            label_ui
                .add(egui::Label::new(RichText::new(current.label.as_str()).size(13.0)).truncate());
            for control in [layout.dropdown, layout.sort] {
                header_separator(ui, control.left() - HEADER_SEPARATOR_WIDTH * 0.5, control);
            }
            self.dropdown_ui(ui, column, layout.dropdown);
            self.sort_button_ui(ui, column, layout.sort);
        } else {
            self.sortable_label_ui(
                ui,
                column,
                ui.max_rect().shrink2(Vec2::new(CELL_PADDING, 0.0)),
            );
        }
    }

    fn sort_button_ui(&mut self, ui: &Ui, column: usize, rect: Rect) {
        if !rect.intersect(ui.clip_rect()).is_positive() {
            return;
        }
        self.sort_interaction(ui, column, rect);
        let sort = self.state.sort_key(self.data, column);
        let icon = match sort {
            Some((_, SortDirection::Descending)) => &re_ui::icons::ARROW_DOWN,
            _ => &re_ui::icons::ARROW_UP,
        };
        let color = if sort.is_some() {
            ui.visuals().text_color()
        } else {
            ui.visuals().weak_text_color()
        };
        let icon_rect = Align2::CENTER_CENTER.anchor_size(rect.center(), Vec2::splat(10.0));
        icon.as_image().tint(color).paint_at(ui, icon_rect);
    }

    fn sort_interaction(&mut self, ui: &Ui, column: usize, rect: Rect) {
        let schema = self.data.column_schema(column);
        let name = if schema.label.is_empty() {
            schema.id.as_str()
        } else {
            schema.label.as_str()
        };
        let description = match self.state.sort_key(self.data, column) {
            None => format!("Sort {name}"),
            Some((priority, SortDirection::Ascending)) => {
                format!("Sort {name}: ascending, priority {priority}")
            }
            Some((priority, SortDirection::Descending)) => {
                format!("Sort {name}: descending, priority {priority}")
            }
        };
        let response = ui
            .interact(
                rect,
                self.id
                    .with(("sort", &self.data.column_group(column).id, &schema.id)),
                Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(format!(
                "{description}\nShift-click to add or change a sort key"
            ));
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                ui.is_enabled(),
                format!("Sort {name}"),
            )
        });
        if response.hovered() {
            ui.painter()
                .rect_filled(rect, 0.0, ui.visuals().widgets.hovered.weak_bg_fill);
        }
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect.shrink(0.5),
                0.0,
                ui.visuals().selection.stroke,
                egui::StrokeKind::Inside,
            );
        }
        if response.clicked() {
            self.actions.push(Action::Sort {
                column,
                additive: ui.input(|input| input.modifiers.shift),
            });
        }
    }

    fn sortable_label_ui(&mut self, ui: &mut Ui, column: usize, rect: Rect) {
        if !rect.intersect(ui.clip_rect()).is_positive() {
            return;
        }
        self.sort_interaction(ui, column, rect);
        let schema = self.data.column_schema(column);
        let sort = self.state.sort_key(self.data, column);
        let marker_width = match sort {
            Some((priority, _)) if priority > 1 => 24.0,
            Some(_) => SORT_MARKER_WIDTH,
            None => 0.0,
        };
        let text_rect = Rect::from_min_size(
            rect.min,
            Vec2::new((rect.width() - marker_width).max(0.0), rect.height()),
        );
        let mut label_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(text_rect)
                .layout(Layout::left_to_right(Align::Center)),
        );
        label_ui.shrink_clip_rect(text_rect);
        let label = label_ui
            .add(egui::Label::new(RichText::new(schema.label.as_str()).size(13.0)).truncate());
        if let Some((priority, direction)) = sort {
            let marker_rect =
                Rect::from_min_max(egui::pos2(label.rect.right() + 2.0, rect.top()), rect.max);
            let mut marker_ui = ui.new_child(
                UiBuilder::new()
                    .max_rect(marker_rect)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            marker_ui.shrink_clip_rect(marker_rect);
            marker_ui.spacing_mut().item_spacing.x = 0.0;
            let icon = match direction {
                SortDirection::Ascending => &re_ui::icons::ARROW_UP,
                SortDirection::Descending => &re_ui::icons::ARROW_DOWN,
            };
            marker_ui.small_icon(icon, Some(ui.visuals().weak_text_color()));
            if priority > 1 {
                marker_ui.label(RichText::new(priority.to_string()).small().weak());
            }
        }
    }

    fn dropdown_ui(&mut self, ui: &mut Ui, column: usize, rect: Rect) {
        if !rect.intersect(ui.clip_rect()).is_positive() {
            return;
        }
        let Some(current) = self.state.selected_option(self.data, column) else {
            return;
        };
        let mut dropdown_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(rect)
                .layout(Layout::left_to_right(Align::Center)),
        );
        dropdown_ui.shrink_clip_rect(rect);
        let style = dropdown_ui.style_mut();
        style.spacing.button_padding = Vec2::new(2.0, 0.0);
        style.spacing.icon_width = 8.0;
        style.spacing.icon_spacing = 0.0;
        style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        let response = egui::ComboBox::from_id_salt(self.id.with((
            "dropdown",
            &self.data.column_group(column).id,
            &self.data.column_schema(column).id,
        )))
        .selected_text("")
        .width(rect.width())
        .truncate()
        .show_ui(&mut dropdown_ui, |ui| {
            for option in self.data.column_options(column) {
                if ui
                    .selectable_label(option.id == current.id, option.label.as_str())
                    .clicked()
                    && option.id != current.id
                {
                    self.actions.push(Action::Select {
                        column,
                        option: option.id.to_string(),
                    });
                }
            }
        })
        .response
        .on_hover_text(format!("Change {}", self.data.column_schema(column).label));
        response.widget_info(|| {
            let mut info = egui::WidgetInfo::new(egui::WidgetType::ComboBox);
            info.enabled = ui.is_enabled();
            info.current_text_value = Some(current.label.to_string());
            info
        });
    }

    fn checkbox_ui(&mut self, ui: &mut Ui, id: Id, row: usize, column: usize, mut checked: bool) {
        let rect = Align2::CENTER_CENTER.anchor_size(ui.max_rect().center(), Vec2::splat(18.0));
        let mut checkbox_ui = ui.new_child(
            UiBuilder::new()
                .id(id)
                .max_rect(rect)
                .layout(Layout::left_to_right(Align::Center)),
        );
        let response = checkbox_ui.re_checkbox(&mut checked, "");
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Checkbox,
                ui.is_enabled(),
                checked,
                format!("{} row {row}", self.data.column_schema(column).id),
            )
        });
        if response.changed() {
            self.actions.push(Action::Check {
                row,
                column,
                checked,
            });
        }
    }
}

struct DropdownHeaderLayout {
    label: Rect,
    dropdown: Rect,
    sort: Rect,
}

impl DropdownHeaderLayout {
    fn controls_width() -> f32 {
        2.0 * (HEADER_CONTROL_WIDTH + HEADER_SEPARATOR_WIDTH)
    }

    fn minimum_width() -> f32 {
        Self::controls_width() + MIN_HEADER_LABEL_WIDTH + 2.0 * HEADER_PADDING
    }

    fn new(rect: Rect, preferred_label_width: f32) -> Option<Self> {
        let content = rect.shrink2(Vec2::new(HEADER_PADDING, 0.0));
        if !content.is_positive() || content.width() < Self::controls_width() {
            return None;
        }
        let label_width = preferred_label_width.min(content.width() - Self::controls_width());
        let label = Rect::from_min_size(content.min, Vec2::new(label_width, content.height()));
        let control_size = Vec2::new(HEADER_CONTROL_WIDTH, content.height());
        let dropdown = Rect::from_min_size(
            egui::pos2(label.right() + HEADER_SEPARATOR_WIDTH, content.top()),
            control_size,
        );
        let sort = dropdown.translate(Vec2::new(
            HEADER_CONTROL_WIDTH + HEADER_SEPARATOR_WIDTH,
            0.0,
        ));
        Some(Self {
            label,
            dropdown,
            sort,
        })
    }
}

struct GroupHeader<'a> {
    label: &'a str,
    rect: Rect,
}

/// Merge sticky and scrolling fragments before placing each group label.
#[derive(Default)]
struct GroupHeaders<'a> {
    cells: BTreeMap<(usize, usize), GroupHeader<'a>>,
}

impl<'a> GroupHeaders<'a> {
    fn collect_fragment(&mut self, info: &HeaderCellInfo, label: &'a str, rect: Rect) {
        self.cells
            .entry((info.row_nr, info.group_index))
            .and_modify(|header| header.rect = header.rect.union(rect))
            .or_insert(GroupHeader { label, rect });
    }

    fn show(&self, ui: &mut Ui, id: Id) {
        for (&(row, group), header) in &self.cells {
            if header.label.is_empty() || header.rect.width() < 48.0 {
                continue;
            }
            let rect = header.rect.shrink2(Vec2::new(4.0, 0.0));
            let mut label_ui = ui.new_child(
                UiBuilder::new()
                    .id_salt(id.with(("group_header", row, group)))
                    .max_rect(rect)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            label_ui.shrink_clip_rect(rect);
            let text = group_header_text(header.label, row == 0);
            label_ui
                .add(egui::Label::new(text).truncate())
                .on_hover_text(header.label);
        }
    }
}

fn header_separator(ui: &Ui, horizontal: f32, rect: Rect) {
    for dot in 0..5 {
        ui.painter().circle_filled(
            egui::pos2(horizontal, rect.center().y - 6.0 + dot as f32 * 3.0),
            0.5,
            ui.tokens().table_header_stroke_color,
        );
    }
}

fn group_header_text(label: &str, primary: bool) -> RichText {
    if primary {
        RichText::new(label).size(13.0).strong()
    } else {
        RichText::new(label).size(12.0)
    }
}

fn paint_header_frame(ui: &Ui, column_header: bool) {
    let rect = ui.max_rect();
    let stroke = egui::Stroke::new(0.5, ui.tokens().table_header_stroke_color);
    if column_header {
        ui.painter()
            .rect_filled(rect, 0.0, ui.visuals().faint_bg_color);
        ui.painter()
            .hline(rect.x_range(), rect.bottom() - 0.5, stroke);
    }
    ui.painter()
        .vline(rect.right() - 0.5, rect.y_range(), stroke);
}

fn paint_row_gutter_divider(ui: &Ui, horizontal: f32, vertical: egui::Rangef) {
    ui.painter().vline(
        horizontal - 0.5,
        vertical,
        egui::Stroke::new(1.0, ui.tokens().table_header_stroke_color),
    );
}

fn paint_numeric_background(ui: &Ui, rect: Rect, row: u64, number: f64, maximum: f64) {
    let strength = normalized_magnitude(number, maximum);
    if strength == 0.0 {
        return;
    }
    let target = if number < 0.0 {
        ui.visuals().error_fg_color
    } else {
        ui.tokens().success_text_color
    };
    let row_color = if row % 2 == 1 {
        ui.visuals().faint_bg_color
    } else {
        ui.visuals().panel_fill
    };
    let fill = row_color.lerp_to_gamma(target, MAX_HEATMAP_MIX * strength);
    ui.painter().rect_filled(rect.shrink(0.5), 0.0, fill);
}

fn selected_row_fill(ui: &Ui) -> egui::Color32 {
    ui.visuals()
        .panel_fill
        .lerp_to_gamma(ui.visuals().selection.bg_fill, 0.45)
}

fn normalized_magnitude(number: f64, maximum: f64) -> f32 {
    if !number.is_finite() || !maximum.is_finite() || maximum <= 0.0 {
        return 0.0;
    }
    (number.abs() / maximum).clamp(0.0, 1.0) as f32
}

fn row_number_ui(ui: &mut Ui, id: Id, text: &str) -> egui::Response {
    let rect = ui.max_rect().shrink2(Vec2::new(4.0, 0.0));
    let mut number_ui = ui.new_child(
        UiBuilder::new()
            .id(id)
            .max_rect(rect)
            .layout(Layout::right_to_left(Align::Center)),
    );
    number_ui.shrink_clip_rect(rect);
    number_ui.add(egui::Label::new(RichText::new(text).weak()).truncate())
}

fn value_ui(ui: &mut Ui, id: Id, value: CellValue<'_>) -> egui::Response {
    let layout = match value {
        CellValue::Float(_) => Layout::right_to_left(Align::Center),
        CellValue::Text(_) => Layout::left_to_right(Align::Center),
    };
    let rect = ui.max_rect().shrink2(Vec2::new(4.0, 0.0));
    let mut value_ui = ui.new_child(UiBuilder::new().id(id).max_rect(rect).layout(layout));
    value_ui.shrink_clip_rect(rect);
    let label = egui::Label::new(cell_text(value));
    let label = match value {
        CellValue::Float(_) => label.extend(),
        CellValue::Text(_) => label.truncate(),
    };
    value_ui.add(label).on_hover_ui(|ui| match value {
        CellValue::Float(number) => {
            ui.label(number.to_string());
        }
        CellValue::Text(text) => {
            ui.label(text);
        }
    })
}

fn cell_text(value: CellValue<'_>) -> String {
    match value {
        CellValue::Float(number) => re_format::FloatFormatOptions::DEFAULT_f64
            .with_decimals(2)
            .with_strip_trailing_zeros(false)
            .format(number),
        CellValue::Text(text) => text.to_owned(),
    }
}
