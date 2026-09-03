---
title: Viewer code architecture
order: 275
description: How re_viewer, re_viewport, and a concrete view compose during a frame
---

This page traces the core Rust code from the top-level Viewer application to a concrete view such as `BarChartView`.

It focuses on two related paths:

- **Call path:** which function invokes the next function.
- **Data path:** which value is produced and passed to the next layer.

For the meaning of entities, components, visualizers, and systems, first read [Entities, components, and Viewer systems](entity-component-systems.md).

For the bottom-up path from chunk metadata through entity subscribers and visualizer instructions, see [From entity to visualizer](entity-visualizer-pipeline.md).

## The architecture in one picture

```text
re_viewer
    owns the application, stores, registries, and frame loop
        |
        v
re_viewport_blueprint
    describes which views exist and how they are arranged
        |
        v
re_viewport
    prepares every active view and manages the tiled central panel
        |
        v
re_view_bar_chart
    prepares BarChart data and draws one BarChart pane
```

The complete per-frame call path is:

```text
App::ui
    |
    v
App::ui_impl
    |
    v
AppState::show
    |
    |-- load ViewportBlueprint
    |-- build DataQueryResult for every view
    |-- construct ViewerContext
    |
    v
ViewportUi::on_frame_start
    |
    v
ViewportUi::viewport_ui
    |
    v
execute_systems_for_all_views
    |
    v
execute_systems_for_view
    |
    |-- create ViewQuery
    |-- create registered visualizers
    |
    v
BarChartVisualizerSystem::execute
    |
    v
SystemExecutionOutput
    |
    v
TilesDelegate::pane_ui
    |
    v
BarChartView::ui
    |
    v
egui draws the pane
    |
    v
ViewportUi::save_to_blueprint_store
```

No layer directly hard-codes this complete BarChart chain. Registries and trait objects connect the generic layers to the concrete implementation.

## Responsibilities by crate

| Crate | Main responsibility |
| --- | --- |
| `re_viewer` | Own the application, route, frame loop, stores, registries, panels, and long-lived view state |
| `re_viewer_context` | Define shared contracts such as `ViewerContext`, `ViewClass`, `VisualizerSystem`, registries, queries, and outputs |
| `re_viewport_blueprint` | Represent persistent view definitions, entity queries, containers, and tile layout |
| `re_viewport` | Execute active view systems and render the tiled central panel |
| `re_view_bar_chart` | Implement the BarChart view class and its visualizer |
| `re_entity_db` and `re_chunk_store` | Own and query recording and blueprint component data |
| `re_renderer` and egui | Render GPU scene data and immediate-mode UI |

The important dependency direction is:

```text
re_viewer
    -> re_viewport
        -> re_viewer_context contracts

re_viewer
    -> re_view_bar_chart
        -> re_viewer_context contracts
```

`re_viewport` does not depend directly on `re_view_bar_chart`. They meet through `ViewClassRegistry` and shared traits from `re_viewer_context`.

## Composition happens at startup

Before any BarChart frame can run, `re_viewer` registers the available view classes.

The startup code creates a `ViewClassRegistry` in:

```text
crates/viewer/re_viewer/src/default_views.rs
```

Conceptually:

```rust
view_class_registry.add_class::<BarChartView>(...)?;
view_class_registry.add_class::<SpatialView2D>(...)?;
view_class_registry.add_class::<SpatialView3D>(...)?;
view_class_registry.add_class::<TensorView>(...)?;
```

`add_class::<BarChartView>` does two things:

1. Stores a `BarChartView` behind `Box<dyn ViewClass>`.
2. Calls `BarChartView::on_register`.

The BarChart registration is:

```rust
fn on_register(
    &self,
    registry: &mut ViewSystemRegistrator<'_>,
) -> Result<(), ViewClassRegistryError> {
    registry.register_visualizer::<BarChartVisualizerSystem>()?;

    registry.register_fallback_provider::<Corner2D>(
        PlotLegend::descriptor_corner().component,
        |_| Corner2D::RightTop,
    );

    Ok(())
}
```

This tells the generic framework:

```text
View class "BarChart"
    |
    |-- create BarChartVisualizerSystem when executing this view
    `-- use RightTop when PlotLegend:corner is unset
```

The visualizer registration stores:

- Its identifier.
- A factory that can create `Box<dyn VisualizerSystem>`.
- Its query requirements.
- The view classes that use it.

The registry therefore contains recipes and relationships, not the result for any particular view instance.

## Type versus instance

Several identifiers are involved because the framework separates types from instances:

| Identifier | Identifies | Example |
| --- | --- | --- |
| `ViewClassIdentifier` | A kind of view | BarChart |
| `ViewId` | One pane instance | One specific BarChart pane |
| `ViewSystemIdentifier` | A kind of visualizer or context system | BarChart visualizer |
| `VisualizerInstructionId` | One configured use of a visualizer | Process `/sales` as bars |
| `EntityPath` | One recorded entity | `/sales` |

This allows:

```text
One BarChartView class
    |
    |-- ViewId A: revenue
    |     |-- /sales/2025
    |     `-- /sales/2026
    |
    `-- ViewId B: latency
          `-- /server/latency
```

Both panes use the same registered class and visualizer types, but have different blueprints, state, queries, and output.

### One pane is assembled from several objects

A live view pane is not represented by one large object. The framework composes it from:

| Piece | Purpose |
| --- | --- |
| `ViewBlueprint` | Persistent definition: class, contents, space origin, properties, and defaults |
| `&dyn ViewClass` | Shared behavior for this kind of view |
| `&mut dyn ViewState` | Transient interaction state for this specific recording and `ViewId` |
| `ViewQuery` | Frame-local entities, instructions, time, and highlights |
| `SystemExecutionOutput` | Frame-local prepared data and diagnostics |

For one BarChart pane:

```text
ViewBlueprint for ViewId 42
    +
registered BarChartView
    +
ViewState for (recording, ViewId 42)
    +
this frame's ViewQuery
    +
this frame's SystemExecutionOutput
    =
one rendered BarChart pane
```

## What persists and what is rebuilt?

Understanding ownership makes the code much easier to follow.

| Value | Lifetime | Owner |
| --- | --- | --- |
| `ViewClassRegistry` | Application lifetime | `App` |
| Recording and blueprint `EntityDb`s | Store lifetime | `StoreHub` |
| `ViewStates` | Across frames, per recording and `ViewId` | `AppState` |
| `ViewportBlueprint` | Loaded snapshot for one frame | `ViewportUi` |
| `ViewerContext` | Borrowed context for one UI pass | Constructed by `AppState::show` |
| `ViewQuery` | One view execution in one frame | `re_viewport` |
| Visualizer instances | Created for each active view execution | `ViewClassRegistry` factories |
| `SystemExecutionOutput` | Prepared data for one view in one frame | `re_viewport` |

`ViewportUi` is deliberately temporary. It is recreated from the blueprint store every frame:

```rust
let viewport_ui = ViewportUi::new(
    ViewportBlueprint::from_db(blueprint_db, &blueprint_query),
);
```

This does not reset interaction state. Long-lived per-pane state is kept separately in `AppState::view_states`.

For BarChart, `new_state` currently returns `Box<()>`, so it has no specialized persistent state. A spatial view instead keeps state such as its camera controller under the same `ViewStates` mechanism.

## Phase 1: enter the Viewer frame

eframe asks the Viewer to repaint by calling `App::ui`.

The high-level application frame handles tasks such as:

- Advancing time.
- Polling data sources.
- Processing fetched chunks.
- Maintaining application navigation.
- Preparing rendering.

It eventually calls `App::ui_impl`, which calls:

```rust
self.state.show(
    // ...
    active_store_context,
    storage_context,
    &self.view_class_registry,
    // ...
);
```

`AppState::show` is the point where the general application frame enters route-specific UI. The viewport path is used for a local recording route.

## Phase 2: load the viewport blueprint

Near the start of `AppState::show`, `re_viewer` loads the viewport description from the blueprint database:

```rust
let viewport_ui = active_store_context.map(|store_context| {
    ViewportUi::new(ViewportBlueprint::from_db(
        store_context.blueprint,
        &blueprint_query,
    ))
});
```

`ViewportBlueprint` describes:

- Which `ViewId`s exist.
- The `ViewBlueprint` for each view.
- Containers and tabs.
- The `egui_tiles` layout tree.
- The maximized view.
- Deferred layout commands.

This is configuration, not rendered output. For example:

```text
ViewId 42
    class        = BarChart
    space origin = /
    contents     = include /sales/**

Layout
    root tabs
        `-- ViewId 42
```

## Phase 3: resolve which entities belong to each view

For every `ViewBlueprint`, `AppState::show` calls:

```rust
view.contents.build_data_result_tree(...)
```

The resulting `DataQueryResult` describes the entities selected by that view and their visualizer instructions.

Conceptually:

```text
DataQueryResult for ViewId 42
    /sales/2025
        visible = true
        instruction = BarChart visualizer

    /sales/2026
        visible = true
        instruction = BarChart visualizer
```

This phase decides **what should be processed**. It does not yet build the final `BarChartData` for the current time.

The results are stored as:

```text
ViewId -> DataQueryResult
```

## Phase 4: construct `ViewerContext`

`AppState::show` gathers the resources needed by view code into a borrowed `ViewerContext`:

```rust
let ctx = ViewerContext {
    app_ctx,
    store_context,
    visualizable_entities_per_visualizer,
    indicated_entities_per_visualizer,
    query_results,
    time_ctrl,
    blueprint_time_ctrl,
    blueprint_query,
};
```

This is dependency injection through a context object. It gives generic and concrete view code access to:

- Recording and blueprint stores.
- Current timeline and time.
- The view query results.
- The view class and fallback registries.
- Selection and hover state.
- Renderer and egui contexts.
- Command sending.

The context borrows these resources. Neither `re_viewport` nor `BarChartView` takes ownership of the application.

### The context objects narrow the scope

The related context types form a progression:

```text
AppContext
    application-wide resources; valid even without an active recording
        |
        v
ViewerContext
    adds the active recording, blueprint, time control, and view query results
        |
        v
ViewContext
    adds one ViewBlueprint and its ViewState
```

`AppState::show` creates `AppContext` and `ViewerContext`. During system execution, `ViewBlueprint::bundle_context_with_state` creates the narrower `ViewContext` passed to visualizers.

After constructing `ViewerContext`, `AppState::show` calls:

```rust
viewport_ui.on_frame_start(&ctx);
```

This performs viewport-level frame setup, including automatic view-spawn heuristics.

## Phase 5: give the central panel to `ViewportUi`

`re_viewer` owns the surrounding application panels:

```text
+--------------------------------------------------+
| Top panel                              re_viewer |
+------------+------------------------+------------+
| Left panel |                        | Right      |
| re_viewer  |      re_viewport       | panel      |
|            |     tiled views        | re_viewer  |
+------------+------------------------+------------+
| Time panel                             re_viewer |
+--------------------------------------------------+
```

It delegates the central panel with:

```rust
egui::CentralPanel::default().show(ui, |ui| {
    viewport_ui.viewport_ui(ui, &ctx, view_states);
});
```

At this point, `re_viewer` has prepared the application context and view membership. `re_viewport` now owns the prepare-and-draw sequence for the central views.

## Phase 6: prepare all active views

`ViewportUi::viewport_ui` calls:

```rust
let executed_systems_per_view =
    execute_systems_for_all_views(ctx, &tree, &blueprint.views, view_states);
```

The result has this shape:

```rust
HashMap<ViewId, (ViewQuery, SystemExecutionOutput)>
```

Only active pane tiles are executed. Different views can be prepared in parallel with Rayon:

```text
worker 1 -> prepare BarChart ViewId 42
worker 2 -> prepare Spatial3D ViewId 73
worker 3 -> prepare Tensor ViewId 91
```

This preparation happens before egui draws the panes sequentially.

## Phase 7: build one `ViewQuery`

For each active view, `new_view_query` reads its `DataQueryResult` and groups visible visualizer instructions by `ViewSystemIdentifier`.

For the BarChart view:

```text
ViewQuery
    view_id   = 42
    timeline  = frame
    latest_at = 100

    active_visualizer_instructions
        BarChart
            |-- /sales/2025
            `-- /sales/2026
```

The distinction is:

```text
DataQueryResult
    resolved entity tree and configured instructions

ViewQuery
    active instructions plus current time and highlights
```

## Phase 8: instantiate and run registered systems

`execute_systems_for_view` asks the registry to instantiate the systems registered for the view class:

```rust
let mut context_systems =
    registry.new_context_collection(view.class_identifier());

let visualizers =
    registry.new_visualizer_collection(view.class_identifier());
```

For BarChart:

```text
context systems = none specialized
visualizers     = BarChartVisualizerSystem
```

`run_view_systems` executes:

1. Context systems first.
2. Visualizer systems second.

Each visualizer without active instructions is skipped. Otherwise, the generic call:

```rust
visualizer.execute(&view_ctx, query, context_systems)
```

uses dynamic dispatch. For this pane, it reaches:

```rust
BarChartVisualizerSystem::execute(...)
```

The execution framework knows only `dyn VisualizerSystem`; the registry determines the concrete type.

## Phase 9: prepare BarChart data

`BarChartVisualizerSystem::execute` iterates over the BarChart instructions in the `ViewQuery`:

```rust
for (data_result, instruction) in
    view_query.iter_visualizer_instruction_for(Self::identifier())
{
    // Query and prepare this entity.
}
```

For each entity, it:

1. Queries `BarChart:values` at the current time.
2. Checks that the tensor is a vector.
3. Resolves abscissa and color.
4. Reads widths or generates fallback widths.
5. Creates `BarChartData`.

The typed result is:

```rust
BTreeMap<EntityPath, BarChartData>
```

For example:

```text
/sales/2025 -> BarChartData { values, abscissa, widths, color }
/sales/2026 -> BarChartData { values, abscissa, widths, color }
```

The visualizer stores it in `VisualizerExecutionOutput`:

```rust
Ok(output.with_visualizer_data(charts))
```

Internally, typed visualizer data is stored by `TypeId` behind `Box<dyn Any + Send + Sync>`. This allows every visualizer to return its own Rust type through one common output container.

## Phase 10: assemble `SystemExecutionOutput`

The framework combines the results for one view:

```text
SystemExecutionOutput for ViewId 42
    |
    |-- context systems
    |
    `-- visualizer results
          `-- "BarChart"
                `-- BTreeMap<EntityPath, BarChartData>
```

Then `execute_systems_for_all_views` builds:

```text
ViewId 42 -> (ViewQuery, BarChart SystemExecutionOutput)
ViewId 73 -> (ViewQuery, Spatial3D SystemExecutionOutput)
ViewId 91 -> (ViewQuery, Tensor SystemExecutionOutput)
```

`ViewportUi` moves this map into `TilesDelegate`.

## Phase 11: lay out tiles and draw one pane

`ViewportUi` asks `egui_tiles` to draw the layout:

```rust
tree.ui(&mut tiles_delegate, ui);
```

When `egui_tiles` reaches ViewId 42, it calls `TilesDelegate::pane_ui`.

`pane_ui`:

1. Looks up the `ViewBlueprint`.
2. Removes `(ViewQuery, SystemExecutionOutput)` for this `ViewId`.
3. Looks up `&dyn ViewClass` in the registry.
4. Gets the persistent `ViewState`.
5. Calls the generic view UI method.

The call is:

```rust
class.ui(
    ctx,
    &missing_chunk_reporter,
    ui,
    view_state,
    &query,
    system_output,
)
```

For ViewId 42, `class` points to `BarChartView`, so dynamic dispatch calls:

```rust
BarChartView::ui(...)
```

The viewport does not need a `match` over every concrete view class.

## Phase 12: `BarChartView::ui` consumes the output

The BarChart UI retrieves the exact type inserted by its visualizer:

```rust
let charts = system_output
    .visualizer_data_or_default::<BTreeMap<EntityPath, BarChartData>>(
        BarChartVisualizerSystem::identifier(),
    )?;
```

The lookup uses two keys:

1. `BarChartVisualizerSystem::identifier()` chooses the visualizer result.
2. `BTreeMap<EntityPath, BarChartData>` chooses the typed payload inside that result.

The producer and consumer match:

```text
Producer:
    BarChartVisualizerSystem
    with_visualizer_data(BTreeMap<EntityPath, BarChartData>)

Consumer:
    BarChartView
    visualizer_data_or_default::<BTreeMap<EntityPath, BarChartData>>(...)
```

The view then:

- Loads plot background and legend properties from the blueprint.
- Uses fallback providers when those properties are unset.
- Converts every `BarChartData` into `egui_plot::Bar` values.
- Handles plot interaction and selection.
- Draws the plot and legend into its assigned tile rectangle.

## Phase 13: persist user changes

The `ViewportBlueprint` is treated as a frame snapshot. Layout changes are queued as deferred `ViewportCommand`s rather than mutating the stored blueprint while it is being rendered.

After the central panel finishes, `AppState::show` calls:

```rust
viewport_ui.save_to_blueprint_store(&ctx);
```

This:

1. Drains deferred viewport commands.
2. Applies them to the frame-local blueprint.
3. Simplifies the tile tree.
4. Writes changed layout components to the blueprint `EntityDb`.

The next frame loads the updated blueprint again.

Other interactions, such as selection and application-level actions, are sent through `CommandSender` so the owning application layer can process them.

## Call flow versus data flow

The same frame can now be read in two ways.

### Call flow

```text
App::ui
  -> AppState::show
    -> ViewportUi::viewport_ui
      -> execute_systems_for_all_views
        -> BarChartVisualizerSystem::execute
      -> TilesDelegate::pane_ui
        -> BarChartView::ui
```

### Data flow

```text
recording EntityDb
    |
    v
DataQueryResult
    |
    v
ViewQuery
    |
    v
BTreeMap<EntityPath, BarChartData>
    |
    v
VisualizerExecutionOutput
    |
    v
SystemExecutionOutput
    |
    v
egui BarChart pane
```

The call flow explains control. The data flow explains why the `system_output` parameter in `BarChartView::ui` contains the result of `BarChartVisualizerSystem::execute`.

## Why the composition looks complex

A direct BarChart application could write:

```rust
let charts = query_bar_charts(recording);
draw_bar_charts(ui, charts);
```

Rerun adds infrastructure because it must support:

- Many view classes.
- Many instances of each view class.
- Several visualizers composed into one view.
- Parallel data preparation before sequential egui drawing.
- Recording data and persistent blueprint configuration.
- Per-view state that survives frames.
- Remote chunks, loading state, diagnostics, selection, and overrides.
- Extensible registration without a central match statement.

The framework version is still conceptually:

```text
resolve contents
    -> prepare data
        -> draw view
            -> persist interaction
```

## Core files to read

Read these files in order to follow the implementation:

1. `crates/viewer/re_viewer/src/default_views.rs`
   - Registers `BarChartView` and other built-in view classes.
2. `crates/viewer/re_view_bar_chart/src/view_class.rs`
   - Registers the BarChart visualizer and implements the pane UI.
3. `crates/viewer/re_viewer/src/app/ui.rs`
   - Calls `AppState::show` from the application frame.
4. `crates/viewer/re_viewer/src/app_state.rs`
   - Loads the viewport blueprint, resolves view contents, builds `ViewerContext`, and delegates the central panel.
5. `crates/viewer/re_viewport/src/viewport_ui.rs`
   - Coordinates system execution, tile layout, and calls each `ViewClass::ui`.
6. `crates/viewer/re_viewport/src/system_execution.rs`
   - Creates `ViewQuery`, instantiates registered systems, and calls `VisualizerSystem::execute`.
7. `crates/viewer/re_view_bar_chart/src/visualizer_system.rs`
   - Queries components and creates `BarChartData`.
8. `crates/viewer/re_viewer_context/src/view/visualizer_system.rs`
   - Defines the visualizer contract and typed output container.
9. `crates/viewer/re_viewer_context/src/view/system_execution_output.rs`
   - Groups all system results for one view and exposes typed retrieval.
10. `crates/viewer/re_viewport_blueprint/src/viewport_blueprint.rs`
    - Defines the persisted viewport model and deferred save-back.

## Compact mental model

```text
re_viewer
    owns everything and begins the frame

ViewClassRegistry
    knows which implementations exist

ViewportBlueprint
    says which view instances should exist

ViewerContext
    lends application and store resources to view code

ViewportUi
    prepares and arranges active views

VisualizerSystem::execute
    converts recording components into prepared view data

ViewClass::ui
    turns prepared data into one interactive pane
```

For BarChart specifically:

```text
AppState::show
    -> ViewportUi
        -> BarChartVisualizerSystem::execute
            -> BTreeMap<EntityPath, BarChartData>
        -> BarChartView::ui
            -> egui plot
```
