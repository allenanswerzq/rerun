use re_log_types::StoreId;

use crate::ViewId;

/// A view-originated interaction for the application embedding the viewer.
///
/// Views send these through [`crate::CommandSender::send_view_event`] when an interaction occurs.
/// They are transient messages, not blueprint properties or logged recording data.
/// See [`ViewEventKind`] for the supported interactions and their typed data.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ViewEvent {
    /// Recording in which the interaction occurred.
    pub store_id: StoreId,

    /// View instance in which the interaction occurred.
    #[serde(with = "crate::blueprint_id_serde")]
    pub view_id: ViewId,

    #[serde(flatten)]
    pub kind: ViewEventKind,
}

impl ViewEvent {
    pub fn new(store_id: StoreId, view_id: ViewId, kind: ViewEventKind) -> Self {
        Self {
            store_id,
            view_id,
            kind,
        }
    }
}

/// Explicit interactions emitted by views.
///
/// Add new interactions as variants with typed fields, and keep the `ViewEventKind`
/// type in `rerun_js/web-viewer/index.ts` in sync.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ViewEventKind {
    /// First empty event, more will follow
    Empty,

    /// A `DataTable` column dropdown changed.
    DataTableColumnChanged {
        #[serde(with = "re_log_types::entity_path_serde")]
        entity_path: re_log_types::EntityPath,
        group_id: String,
        column_id: String,
        option_id: String,
    },

    /// A `DataTable` checkbox changed, identified by its unsorted source row.
    DataTableCheckboxChanged {
        #[serde(with = "re_log_types::entity_path_serde")]
        entity_path: re_log_types::EntityPath,
        group_id: String,
        column_id: String,
        row_index: u64,
        checked: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_event_wire_format() {
        let event = ViewEvent::new(
            StoreId::recording("view_events", "test"),
            ViewId::random(),
            ViewEventKind::Empty,
        );
        let value = serde_json::to_value(&event).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "store_id": {
                    "kind": "Recording",
                    "application_id": "view_events",
                    "recording_id": "test",
                },
                "view_id": event.view_id.uuid().to_string(),
                "type": "empty",
            })
        );
        assert_eq!(
            serde_json::from_value::<ViewEvent>(value.clone()).unwrap(),
            event
        );

        let mut invalid = value;
        invalid["type"] = serde_json::json!("unknown");
        assert!(serde_json::from_value::<ViewEvent>(invalid).is_err());
    }

    #[test]
    fn data_table_event_wire_format() {
        let variants = [
            (
                ViewEventKind::DataTableColumnChanged {
                    entity_path: "table".into(),
                    group_id: "stocks".into(),
                    column_id: "return".into(),
                    option_id: "week".into(),
                },
                "data_table_column_changed",
            ),
            (
                ViewEventKind::DataTableCheckboxChanged {
                    entity_path: "table".into(),
                    group_id: "stocks".into(),
                    column_id: "checked".into(),
                    row_index: 7,
                    checked: true,
                },
                "data_table_checkbox_changed",
            ),
        ];
        for (kind, tag) in variants {
            let event =
                ViewEvent::new(StoreId::recording("tables", "test"), ViewId::random(), kind);
            let value = serde_json::to_value(&event).unwrap();
            assert_eq!(value["type"], tag);
            assert_eq!(value["entity_path"], "/table");
            assert_eq!(value["group_id"], "stocks");
            assert_eq!(serde_json::from_value::<ViewEvent>(value).unwrap(), event);
        }
    }
}
