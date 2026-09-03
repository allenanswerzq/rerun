---
title: From entity to visualizer
order: 260
description: How the Viewer discovers compatible entities, creates visualizer instructions, and draws them
---

This page follows one entity from stored component metadata to pixels in a view.

It focuses on the BarChart test:

```text
EntityPath: /time_series
Component:  BarChart:values
View class: BarChart
Visualizer: BarChartVisualizerSystem
```

For the underlying terminology, first read [Entities, components, and Viewer systems](entity-component-systems.md). For the top-down application frame, see [Viewer code architecture](viewer-code-architecture.md). For how the recording stores chunks and components, see [Recording and blueprint storage hierarchy](../logging-and-ingestion/storage-hierarchy.md).

## The complete pipeline

```text
TestContext::new_with_view_class::<BarChartView>()
    |
    v
ViewClassRegistry::add_class::<BarChartView>()
    |
    v
BarChartView::on_register()
    |
    v
register_visualizer::<BarChartVisualizerSystem>()
    |
    v
BarChartVisualizerSystem::visualizer_query_info()
    |
    v
Store immutable VisualizerEntityConfig
    target component   = BarChart:values
    semantic type      = TensorData
    relevant archetype = BarChart
    physical type      = TensorData's Arrow datatype
    |
    v
Test logs a chunk at /time_series
    |
    v
First TestContext::run creates StoreCache
    |
    v
StoreCache creates BarChart subscriber
    |
    v
subscriber.bootstrap(recording EntityDb)
    |
    v
Read ChunkMeta for /time_series
    |
    |-- archetype matches BarChart
    |     `-- add /time_series to indicated entities
    |
    `-- TensorData satisfies the required-component constraint
          `-- add /time_series to visualizable entities
    |
    v
ViewContents::build_data_result_tree()
    |
    |-- the view's entity query includes /time_series
    |-- BarChart is available for this view class
    `-- BarChartView recommends the BarChart visualizer
    |
    v
Create VisualizerInstruction
    entity     = /time_series
    visualizer = BarChart
    mappings   = default or blueprint-configured
    |
    v
Create ViewQuery for the current frame
    timeline  = active timeline
    latest_at = current time
    |
    v
BarChartVisualizerSystem::execute()
    |
    |-- query the actual current component values
    |-- verify that values form a vector
    |-- resolve abscissa, widths, color, and fallbacks
    `-- create BTreeMap<EntityPath, BarChartData>
    |
    v
SystemExecutionOutput
    |
    v
BarChartView::ui()
    |
    `-- create egui_plot bars and draw the pane
```

The pipeline has two major phases:

```text
Discovery phase
    Which visualizers might understand each entity?

Execution phase
    What should those visualizers draw at the current time?
```

Discovery is cheap, schema-driven, and cached. Execution queries actual values and runs for active views each frame.

## Important objects and their lifetimes

| Object | Responsibility | Lifetime |
| --- | --- | --- |
| `ViewClassRegistry` | Stores view classes, visualizer factories, and matching templates | Application |
| `VisualizerEntityConfig` | Immutable matching template for one visualizer type | Application |
| `StoreCache` | Viewer-specific caches for one recording store | Store |
| `VisualizerEntitySubscriber` | Candidate entities for one visualizer in one store | Store |
| `DataQueryResult` | Resolved entity tree and visualizer instructions for one view | Rebuilt as view/store state changes |
| `VisualizerInstruction` | Assigns one entity to one visualizer | Blueprint-derived or heuristic |
| `ViewQuery` | Active instructions, time, and highlights for one frame | Frame |
| `VisualizerExecutionOutput` | Output from one visualizer execution | Frame |
| `SystemExecutionOutput` | All system outputs for one view | Frame |
| `BarChartData` | Prepared CPU-side chart data | Frame |

## Stage 1: register the view class

The BarChart test starts with:

```rust
let mut test_context =
    TestContext::new_with_view_class::<BarChartView>();
```

`new_with_view_class` first creates the miniature Viewer environment, then calls:

```rust
test_context.register_view_class::<BarChartView>();
```

That forwards to:

```rust
ViewClassRegistry::add_class::<BarChartView>(...)
```

The registry stores:

```text
ViewClassIdentifier "BarChart"
    -> Box<dyn ViewClass> containing BarChartView
```

The view class is generally stateless. Individual pane state is stored separately by `(StoreId, ViewId)`.

### `on_register` declares the view's systems

After adding the class, the registry calls:

```rust
BarChartView::on_register(...)
```

The BarChart implementation registers:

```rust
system_registry.register_visualizer::<BarChartVisualizerSystem>()?;
```

This establishes the relationship:

```text
BarChart view class
    `-- may execute BarChartVisualizerSystem
```

It does not execute the visualizer and does not create a pane. It only registers the type and its factory.

## Stage 2: turn query information into a matching template

During registration, the framework constructs a temporary `BarChartVisualizerSystem` and calls:

```rust
visualizer.visualizer_query_info(app_options)
```

BarChart returns:

```rust
VisualizerQueryInfo::single_required_component::<TensorData>(
    &BarChart::descriptor_values(),
    &BarChart::all_components(),
)
```

This describes:

```text
Required target:
    BarChart:values

Expected semantic type:
    rerun.components.TensorData

Relevant archetype:
    rerun.archetypes.BarChart

Components queried during execution:
    BarChart:values
    BarChart:abscissa
    BarChart:widths
    BarChart:color
```

The registry converts this to an immutable `VisualizerEntityConfig`:

```text
VisualizerEntityConfig
    visualizer          = "BarChart"
    relevant_archetype  = rerun.archetypes.BarChart
    constraints         = SingleRequiredComponent
    known enum types    = shared reflection-derived set
```

The constraint records:

- The target component identifier.
- Its semantic component type.
- Accepted physical Arrow datatypes.
- Whether static data is allowed.

This configuration is a template. It has no recording-specific entity paths yet.

## Stage 3: create one subscriber per store and visualizer type

Each recording gets a `StoreCache`.

When `StoreCache::new` runs, it asks the registry:

```rust
view_class_registry.create_entity_subscribers()
```

The registry creates an empty subscriber from every visualizer template:

```text
StoreCache for Recording A
    |-- BarChart subscriber
    |-- Points3D subscriber
    |-- Image subscriber
    `-- ...

StoreCache for Recording B
    |-- BarChart subscriber
    |-- Points3D subscriber
    |-- Image subscriber
    `-- ...
```

There is one subscriber per:

```text
StoreId + ViewSystemIdentifier
```

There is not one subscriber per view pane or entity.

### Why registration order matters

View and visualizer types are normally registered before store caches are created.

The safe order is:

```text
Register view and visualizer types
    -> create StoreCache
        -> create all required subscribers
```

If a `StoreCache` already exists and a new visualizer type is registered afterward, that existing cache does not automatically acquire the newly registered subscriber.

Logging before registration can still be discovered if the cache is only created later, because bootstrap scans existing data. Registering all view types first avoids depending on that timing.

## Stage 4: feed recording schema into the subscriber

Subscribers learn about entity/component combinations in two ways.

### Bootstrap existing data

When a `StoreCache` is created, each subscriber calls:

```rust
subscriber.bootstrap(entity_db)
```

Bootstrap scans:

- Existing physical chunks.
- RRD manifest metadata for virtual or not-yet-loaded chunks.

This makes a newly created subscriber aware of data that arrived before the cache existed.

### Process incremental schema events

When a cache already exists, inserting a chunk produces `ChunkStoreEvent`s. Newly discovered entity/component columns produce:

```text
ChunkStoreDiff::SchemaAddition
```

`StoreCache::on_store_events` forwards those events to every visualizer subscriber.

The subscriber does not process every new row. It reacts when the store schema learns something new, such as:

```text
Previously unseen:
    EntityPath /time_series
    Component BarChart:values
```

This avoids rechecking the same entity/component schema for every temporal update.

## The exact BarChart test takes the bootstrap path

The test sequence is:

```text
1. Register BarChartView
2. Log /time_series into the recording EntityDb
3. Call setup_viewport_blueprint
4. setup_viewport_blueprint calls TestContext::run
5. TestContext::run creates the StoreCache lazily
6. StoreCache creates the BarChart subscriber
7. subscriber.bootstrap scans the existing /time_series chunk
```

Therefore, in this test the subscriber usually does not receive a live callback at the instant `log_entity` runs. It discovers the already-present chunk during bootstrap.

After the cache exists, newly introduced columns use the incremental `SchemaAddition` path.

## Stage 5: reduce a chunk to `ChunkMeta`

The subscriber does not need the complete arrays of component values.

It receives lightweight metadata:

```rust
ChunkMeta {
    entity_path,
    components,
}
```

Each `ChunkComponentMeta` contains:

```text
ComponentDescriptor
    archetype
    component identifier
    component type

Inner Arrow datatype
Whether the column has data
Whether it has been written as static
```

For the BarChart test, the metadata is approximately:

```text
ChunkMeta
    entity_path = /time_series

    component:
        archetype          = rerun.archetypes.BarChart
        identifier         = BarChart:values
        component type     = rerun.components.TensorData
        Arrow datatype     = TensorData's physical representation
        static             = true
```

This is enough to determine whether BarChart is a plausible interpretation.

## Stage 6: determine whether the entity is indicated

`process_entity_components` first compares archetype metadata.

The BarChart matching template says:

```text
relevant archetype = rerun.archetypes.BarChart
```

The logged component says:

```text
component archetype = rerun.archetypes.BarChart
```

They match, so the subscriber records:

```text
BarChart indicated entities:
    /time_series
```

**Indicated** means:

> The way this data was logged strongly suggests using this visualizer.

It is a semantic recommendation signal, not proof that the current value can be drawn.

## Stage 7: determine whether the entity is visualizable

Next, the subscriber evaluates the visualizer's component constraints.

BarChart uses `SingleRequiredComponentConstraint`:

```text
Target:
    BarChart:values

Expected semantic type:
    TensorData

Accepted physical type:
    TensorData's Arrow datatype

Static components:
    allowed
```

The incoming `BarChart:values` column matches both:

```text
Semantic type match:
    TensorData == TensorData

Physical datatype match:
    expected Arrow datatype == incoming Arrow datatype
```

The subscriber records a native semantic match:

```text
BarChart visualizable entities:
    /time_series
        reason:
            SingleRequiredComponentMatch
            target = BarChart:values
            source = BarChart:values
            match  = NativeSemantics
```

**Visualizable** means:

> The entity has a component schema that this visualizer might be able to process.

## Semantic matches versus physical matches

The constraint can recognize more than exact built-in semantics.

### Native semantic match

```text
Incoming component type:
    TensorData

Expected component type:
    TensorData
```

This is the strongest match.

### Physical datatype-only match

A custom component can have a compatible Arrow datatype without using Rerun's `TensorData` semantic type.

```text
Incoming:
    user.SalesValues
    compatible Arrow representation

Expected:
    TensorData
```

The subscriber can record that the physical representation is compatible while preserving that the semantics differ. This information can later help propose component mappings.

It can also inspect nested Arrow fields and record selectors when compatible data is nested inside a struct.

Built-in enum component types require semantic matches because many unrelated enums share the same primitive Arrow datatype.

## Visualizable and indicated are independent

### Logged as BarChart

```text
BarChart:values -> TensorData

visualizable = yes
indicated    = yes
```

### Compatible tensor under another archetype

```text
Tensor:data -> TensorData

visualizable = yes
indicated    = no
```

### BarChart archetype metadata but missing required compatible values

```text
some BarChart-associated optional component

visualizable = no
indicated    = yes
```

The subscriber keeps separate collections:

```text
VisualizableEntities
    EntityPath -> VisualizableReason

IndicatedEntities
    set of EntityPath
```

## The subscriber is an approximate, additive index

The subscriber operates on schema metadata. It does not:

- Query the value at the current time.
- Deserialize the tensor.
- Check whether a tensor is one-dimensional.
- Resolve blueprint overrides or fallbacks.
- Create `BarChartData`.
- Draw bars.

For example, a matrix can have the same `TensorData` schema as a vector. The subscriber may mark its entity as a BarChart candidate, but `BarChartVisualizerSystem::execute` later rejects the current value if `tensor.is_vector()` is false.

The candidate sets also latch:

> Once an entity has matched at any point, it remains a candidate.

Deleting a chunk or moving to a time with no value does not currently remove it from the set. This is safe because exact execution still checks for usable current data.

## Stage 8: expose candidate sets through `StoreCache`

`StoreCache` exposes:

```text
ViewSystemIdentifier -> VisualizableEntities
ViewSystemIdentifier -> IndicatedEntities
```

For the test recording:

```text
"BarChart"
    visualizable:
        /time_series -> native TensorData match

    indicated:
        /time_series
```

`TestContext::run` borrows these collections into `ViewerContext`. The normal Viewer constructs the same information from its active store context.

## Stage 9: apply the view class and entity query

Candidate status alone does not put an entity into a view.

`ViewContents::build_data_result_tree` combines:

```text
Store-level visualizer candidates
    +
visualizers registered for this view class
    +
the view's entity-path query
    +
blueprint visibility and override state
```

### Restrict candidates to the view class

The recording may have candidates for many visualizers:

```text
/time_series
    BarChart candidate
    Tensor candidate
```

For a `BarChartView`, only visualizers registered for the BarChart class are considered:

```text
BarChartView
    `-- BarChartVisualizerSystem
```

### Apply the entity-path filter

The test uses:

```rust
ViewBlueprint::new_with_root_wildcard(
    BarChartView::identifier(),
)
```

Its contents query includes everything under the root:

```text
+ /**
```

Therefore `/time_series` is included.

If the blueprint instead excluded it:

```text
- /time_series
```

the entity would not enter this view even though it remains a valid BarChart candidate for the recording.

### Create a `DataResult`

The query builder creates a node describing `/time_series` in this view:

```text
DataResult
    entity_path              = /time_series
    visible                  = true
    any_visualizers_available = true
    visualizer_instructions  = not resolved yet
    override_base_path       = blueprint path for this view/entity
```

## Stage 10: choose visualizers

There are two ways to choose an entity's visualizers.

### Explicit blueprint configuration

If the blueprint contains active visualizer instruction IDs, those instructions win.

The resolver reads:

```text
Visualizer instruction IDs
Visualizer type
Component mappings
Component overrides
```

This is how the user can explicitly add, remove, or configure visualizers.

### Heuristic recommendation

If the blueprint does not explicitly choose visualizers, the query builder calls:

```rust
view_class.recommended_visualizers_for_entity(...)
```

The default implementation recommends visualizers that are both:

```text
visualizable
AND
indicated
```

BarChart overrides this rule. Because it currently has only one visualizer, it recommends `BarChartVisualizerSystem` whenever that visualizer appears in `visualizers_with_reason`.

This allows a BarChart view to interpret compatible one-dimensional tensor or image data even when it was not logged with a BarChart archetype indicator.

## Stage 11: create a `VisualizerInstruction`

The recommendation becomes a concrete instruction:

```text
VisualizerInstruction
    id                 = deterministic or blueprint-provided UUID
    visualizer_type    = BarChart
    component_mappings = default or configured mappings
    override_base_path = blueprint path for /time_series in this view
```

This is the key handoff:

```text
Subscriber:
    "/time_series could work with BarChart"

VisualizerInstruction:
    "In this particular view, run BarChart for /time_series"
```

The same entity can have different instructions in different views:

```text
BarChart ViewId A:
    /time_series -> BarChart visualizer

Tensor ViewId B:
    /time_series -> Tensor visualizer
```

An entity is data; the instruction chooses one interpretation of that data.

## Stage 12: create the frame-local `ViewQuery`

Before system execution, `new_view_query` walks visible `DataResult`s and groups their instructions:

```text
ViewQuery
    view_id      = BarChart pane's ViewId
    space_origin = /
    timeline     = active TimeControl timeline
    latest_at    = current TimeControl cursor
    highlights   = current selection and hover state

    instructions by visualizer:
        BarChart:
            (/time_series, instruction)
```

Only visible data results contribute active instructions.

This transforms a view-level entity tree into the work list used by visualizer systems.

## Stage 13: instantiate and execute systems

`execute_systems_for_view` asks `ViewClassRegistry` to create fresh system collections for the view class:

```rust
let context_systems =
    registry.new_context_collection(view.class_identifier());

let visualizers =
    registry.new_visualizer_collection(view.class_identifier());
```

For BarChart:

```text
context systems = no specialized context system
visualizers     = BarChartVisualizerSystem
```

The execution framework:

1. Runs context systems first.
2. Skips visualizers with no active instructions.
3. Runs active visualizers, potentially in parallel.

The generic call:

```rust
visualizer.execute(&view_ctx, &view_query, &context_systems)
```

dynamically dispatches to:

```rust
BarChartVisualizerSystem::execute(...)
```

## Stage 14: query and validate actual values

Execution is where exact, time-dependent work happens.

For every BarChart instruction, `BarChartVisualizerSystem::execute`:

1. Builds a latest-at query from `ViewQuery`.
2. Resolves recording data, mappings, blueprint overrides, defaults, and fallbacks.
3. Gets the current `BarChart:values` as `TensorData`.
4. Skips the entity if no value exists.
5. Checks `tensor.is_vector()`.
6. Determines how many bars are needed.
7. Resolves abscissa values.
8. Resolves color.
9. Reads or generates widths.
10. Creates `BarChartData`.

For the test:

```text
/time_series
    values   = [0.0, 1.01, ..., 100.0]
    abscissa = fallback [0, 1, ..., 99]
    widths   = fallback [1, 1, ..., 1]
    color    = fallback color
```

The visualizer produces:

```rust
BTreeMap<EntityPath, BarChartData>
```

and stores it in:

```rust
VisualizerExecutionOutput::with_visualizer_data(charts)
```

## Stage 15: pass output to the view

The framework combines the outputs for the pane:

```text
SystemExecutionOutput
    `-- result for visualizer "BarChart"
          `-- BTreeMap<EntityPath, BarChartData>
```

`ViewportUi` gives the map of prepared view outputs to `TilesDelegate`.

When egui reaches the BarChart pane, `TilesDelegate::pane_ui`:

1. Selects the output for that `ViewId`.
2. Gets the registered `BarChartView` as `&dyn ViewClass`.
3. Gets the pane's persistent `ViewState`.
4. Calls:

```rust
view_class.ui(
    ctx,
    missing_chunk_reporter,
    ui,
    view_state,
    view_query,
    system_output,
)
```

Dynamic dispatch reaches:

```rust
BarChartView::ui(...)
```

## Stage 16: retrieve typed data and draw

`BarChartView::ui` retrieves the value produced by its visualizer:

```rust
let charts = system_output
    .visualizer_data_or_default::<BTreeMap<EntityPath, BarChartData>>(
        BarChartVisualizerSystem::identifier(),
    )?;
```

The producer and consumer agree on:

```text
Visualizer identifier:
    BarChart

Rust payload type:
    BTreeMap<EntityPath, BarChartData>
```

The view converts each `BarChartData` into:

```text
egui_plot::Bar
egui_plot::BarChart
legend entries
hover and selection interactions
```

Finally:

```rust
plot_ui.bar_chart(chart);
```

draws the bars.

## Discovery path versus execution path

The same component information is examined at two different levels.

| Phase | Reads | Purpose |
| --- | --- | --- |
| Subscriber discovery | Entity path, descriptors, semantic types, Arrow datatypes | Build cheap candidate sets |
| Visualizer execution | Actual component values at current time | Validate and prepare display data |

For BarChart:

```text
Discovery:
    "This entity has a TensorData-compatible column."

Execution:
    "At frame 42, that column contains a vector with these values."
```

This separation is central to the architecture.

## Test path versus production path

### BarChart test

```text
Register view
    -> log data
        -> first TestContext::run
            -> lazily create StoreCache
                -> bootstrap subscribers from existing chunks
```

### Running Viewer

At application startup, built-in views are registered before recordings are shown.

When a recording becomes active:

```text
Create StoreCache
    -> bootstrap subscribers from data already loaded
```

As new data streams in:

```text
Insert chunk
    -> update store schema
        -> emit SchemaAddition for new entity/component columns
            -> update subscribers incrementally
```

Both paths produce the same cached candidate collections.

## What each stage knows

| Stage | Knows entity? | Knows view? | Knows current value/time? | Draws? |
| --- | ---: | ---: | ---: | ---: |
| `VisualizerEntityConfig` | No | View-class relationships only | No | No |
| `VisualizerEntitySubscriber` | Yes | No specific pane | No | No |
| `DataQueryResult` | Yes | Yes | Mostly configuration/query range | No |
| `VisualizerInstruction` | Yes | Yes | No | No |
| `ViewQuery` | Yes | Yes | Knows active timeline/time | No |
| `VisualizerSystem::execute` | Yes | Yes | Yes | Prepares draw or typed data |
| `ViewClass::ui` | Yes, through output | Yes | Receives prepared current data | Yes |

## Why not just inspect everything every frame?

Without subscribers, the Viewer could do:

```text
For every frame:
    for every visible view:
        for every entity:
            for every registered visualizer:
                inspect every component datatype
```

For a large recording, that cost can grow roughly with:

```text
frames x views x entities x visualizers x components
```

The subscriber changes the expensive discovery dimension:

```text
When schema changes:
    inspect the new entity/component metadata
    cache candidate relationships

Every frame:
    look up candidates
    execute only selected visualizer instructions
```

Other benefits include:

- Custom component mappings based on compatible physical datatypes.
- Multiple visualizers for one entity.
- Different interpretations in different views.
- Explicit blueprint overrides.
- Parallel visualizer execution.
- Remote-manifest discovery without loading every chunk.

## Why an entity may not appear

When debugging a missing entity, follow the pipeline in order.

### 1. Was the visualizer registered?

Check:

```text
ViewClassRegistry
    contains the view class
    contains the visualizer type
    associates the visualizer with that view class
```

### 2. Does the subscriber consider the entity visualizable?

Check:

- Required semantic type.
- Physical Arrow datatype.
- Static-data allowance.
- Buffer/format constraints for visualizers that need both.

### 3. Is the entity included by the view query?

An entity can be a valid candidate but excluded by:

```text
- /time_series
```

or by the view's space/origin rules.

### 4. Was a visualizer instruction created?

Check:

- Explicit `ActiveVisualizers` blueprint state.
- `recommended_visualizers_for_entity`.
- Component mappings.
- Entity visibility.

### 5. Is there a usable value at the current time?

The subscriber is time-independent, but execution uses:

```text
active timeline + current latest-at time
```

An entity can be a candidate while having no value at the current query.

### 6. Does exact validation pass?

BarChart requires the current tensor to be a vector. A matrix can pass schema discovery but produce no `BarChartData`.

### 7. Did the view retrieve the expected output type?

The visualizer identifier and Rust payload type used by the producer and consumer must agree.

## Compact mental model

```text
Registration
    teaches the Viewer what each visualizer understands

Subscriber
    indexes which entities might match

View contents query
    chooses which candidate entities belong in one pane

Recommendation or blueprint override
    chooses how each entity should be interpreted

VisualizerInstruction
    records that entity-to-visualizer assignment

ViewQuery
    adds current time and visibility

VisualizerSystem::execute
    queries and validates actual current values

SystemExecutionOutput
    carries prepared data

ViewClass::ui
    turns that data into pixels and interactions
```

The central handoff is:

```text
Entity schema
    -> candidate visualizers
        -> selected visualizer instruction
            -> current component values
                -> prepared view data
                    -> drawn view
```
