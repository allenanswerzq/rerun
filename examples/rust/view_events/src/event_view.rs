use rerun::external::egui;
use rerun::external::re_log_types::EntityPath;
use rerun::external::re_sdk_types::ViewClassIdentifier;
use rerun::external::re_ui;
use rerun::external::re_viewer_context::{
    MissingChunkReporter, SystemExecutionOutput, ViewClass, ViewClassLayoutPriority,
    ViewClassRegistryError, ViewClassUiOutput, ViewEvent, ViewEventKind, ViewQuery,
    ViewSpawnHeuristics, ViewState, ViewSystemExecutionError, ViewSystemRegistrator, ViewerContext,
};

#[derive(Default)]
pub struct EventView;

impl ViewClass for EventView {
    fn identifier() -> ViewClassIdentifier {
        "ViewEvents".into()
    }

    fn display_name(&self) -> &'static str {
        "View events"
    }

    fn help(&self, _os: egui::os::OperatingSystem) -> re_ui::Help {
        re_ui::Help::new("View events")
    }

    fn on_register(
        &self,
        _system_registry: &mut ViewSystemRegistrator<'_>,
    ) -> Result<(), ViewClassRegistryError> {
        Ok(())
    }

    fn new_state(&self) -> Box<dyn ViewState> {
        Box::new(())
    }

    fn layout_priority(&self) -> ViewClassLayoutPriority {
        ViewClassLayoutPriority::Low
    }

    fn spawn_heuristics(
        &self,
        _ctx: &ViewerContext<'_>,
        _include_entity: &dyn Fn(&EntityPath) -> bool,
    ) -> ViewSpawnHeuristics {
        ViewSpawnHeuristics::root()
    }

    fn ui(
        &self,
        ctx: &ViewerContext<'_>,
        _missing_chunk_reporter: &MissingChunkReporter,
        ui: &mut egui::Ui,
        _state: &mut dyn ViewState,
        query: &ViewQuery<'_>,
        _system_output: SystemExecutionOutput,
    ) -> Result<ViewClassUiOutput, ViewSystemExecutionError> {
        if ui.button("Send event").clicked() {
            ctx.command_sender().send_view_event(ViewEvent::new(
                ctx.recording().store_id().clone(),
                query.view_id,
                ViewEventKind::Empty,
            ));
        }
        Ok(ViewClassUiOutput::default())
    }
}
