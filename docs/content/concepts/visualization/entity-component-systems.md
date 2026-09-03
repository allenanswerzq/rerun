---
title: Entities, components, and Viewer systems
order: 250
description: How the Viewer turns component data into visualizations
---

Rerun's data model is inspired by [Entity Component System (ECS)](https://en.wikipedia.org/wiki/Entity_component_system):

| Concept | Question it answers | Rerun example |
| --- | --- | --- |
| Entity | Which thing is this? | `/sales`, `/world/car`, `/camera` |
| Component | What data does it have? | `TensorData`, `Color`, `Position3D` |
| System | What should be done with that data? | Prepare bars, points, images, or meshes |

The Viewer applies this separation when it turns recorded data into a visualization. Rerun is not a textbook game-engine ECS, however: its components are temporal Arrow data, and Viewer systems normally query and derive data rather than mutate a world.

## The data side

### Entities provide identity

An entity is identified by its [entity path](../logging-and-ingestion/entity-path.md):

```text
/sales
/world/car
/world/car/camera
```

Logging to the same path again updates that entity at another time; it does not create a new entity.

### Components hold data

A component is one typed piece of data associated with an entity, such as a position, color, tensor, or radius. Components contain data but do not decide how that data should be displayed.

Rerun distinguishes two related names:

| Name | Meaning | Example |
| --- | --- | --- |
| Component type | The shape and meaning of the values | `rerun.components.TensorData` |
| Component identifier | The role those values play in an archetype | `BarChart:values` |

Several fields can use the same component type for different purposes. The identifier preserves that purpose:

```text
BarChart:values  -> TensorData
BarChart:abscissa -> TensorData
```

### Archetypes bundle components for logging

An archetype is a convenient, meaningful group of components. For example:

```rust
rec.log(
    "sales",
    &rerun::BarChart::new([8_i64, 4, 9].as_slice()),
)?;
```

Conceptually, this logs:

```text
Entity:
    /sales

Component:
    BarChart:values -> TensorData([8, 4, 9])
```

The stored recording does not need to keep a long-lived `BarChart` Rust object. It stores component batches and their descriptors in Arrow-backed chunks. The archetype is primarily the convenient SDK representation used to construct those components.

See [Entities and Components](../logging-and-ingestion/entity-component.md) for more about the recording data model.

## The Viewer side

The Viewer introduces three important concepts:

| Viewer concept | Responsibility |
| --- | --- |
| Visualizer instruction | Assign an entity to a visualizer, including component mappings |
| Visualizer system | Query components and prepare display-ready data |
| View class | Draw and control an entire pane |

### Visualizer instructions route entities

An entity is not permanently tied to one presentation. Compatible tensor data might be shown as a tensor or as a bar chart.

A visualizer instruction records the chosen interpretation:

```text
Entity /sales
    -> use the BarChart visualizer
    -> source BarChart:values from its tensor data
```

A view can have many instructions for the same visualizer:

```text
BarChart view
    /sales/2025 -> BarChart visualizer
    /sales/2026 -> BarChart visualizer
```

### Visualizer systems prepare data

A visualizer system is a registered worker that:

1. Declares which components it can use.
2. Receives instructions for matching entities.
3. Queries those components at the current time.
4. Resolves optional values and fallbacks.
5. Produces typed data for the view.

The BarChart visualizer requires values compatible with `TensorData`. It also reads optional abscissa, width, and color components. Its output is conceptually:

```text
BarChart visualizer output
    /sales/2025 -> BarChartData
    /sales/2026 -> BarChartData
```

The visualizer does not own the pane, legend, zoom controls, or final egui drawing. It only prepares the data they need.

### View classes draw panes

A view class owns the presentation and interaction for one kind of pane. `BarChartView`, for example, handles:

- Plot layout
- Grid and background
- Legend placement
- Zoom and pointer interaction
- Selection
- Converting prepared `BarChartData` into egui bars

This separation makes the execution model approximately:

```rust
let prepared_data = visualizer.execute(query);
view.ui(prepared_data);
```

The real framework uses registries and trait objects so the viewport can perform this sequence without knowing whether the pane is a BarChart, Spatial3D view, tensor view, or another view type.

## A BarChart frame

The complete BarChart path is:

```text
Recording
    /sales
    BarChart:values = [8, 4, 9]
        |
        v
View query
    /sales has a BarChart visualizer instruction
        |
        v
BarChartVisualizerSystem::execute
    queries values, abscissa, widths, and color
    creates BTreeMap<EntityPath, BarChartData>
        |
        v
VisualizerExecutionOutput
    stores the typed map
        |
        v
SystemExecutionOutput
    contains all visualizer results for this view
        |
        v
BarChartView::ui
    retrieves the typed map
    builds egui bars, legend, and interactions
        |
        v
Screen
```

The viewport coordinates this lifecycle:

1. `ViewportUi` calls `execute_systems_for_all_views`.
2. The execution framework finds the systems registered for each active view.
3. It dynamically calls `VisualizerSystem::execute`.
4. For a BarChart view, that dispatches to `BarChartVisualizerSystem::execute`.
5. The framework collects the result in `SystemExecutionOutput`.
6. When egui reaches the pane, the viewport calls the registered `ViewClass::ui`.
7. For that pane, dynamic dispatch calls `BarChartView::ui` and moves in the corresponding `system_output`.

The data inserted by the visualizer:

```rust
output.with_visualizer_data(charts)
```

is retrieved by the view using the same visualizer identifier and Rust type:

```rust
system_output.visualizer_data_or_default::<
    BTreeMap<EntityPath, BarChartData>
>(BarChartVisualizerSystem::identifier())
```

## Visualizable versus indicated

When choosing instructions automatically, the Viewer asks two different questions:

### Is the entity visualizable?

Does it contain data that the visualizer can technically process?

A one-dimensional tensor can be visualizable as a BarChart because its values are compatible with the required `TensorData`.

### Is the visualizer indicated?

Does the logged archetype strongly suggest that interpretation?

```text
Logged as BarChart:
    visualizable = yes
    indicated    = yes

Logged as a compatible 1D Tensor:
    visualizable = yes
    indicated    = no
```

The default recommendation logic is conservative and normally selects visualizers that are both visualizable and indicated. The BarChart view overrides that rule: because it has only one visualizer, it also recommends that visualizer for compatible one-dimensional tensors and images.

## Why BarChart has one visualizer

A visualizer type is a kind of data-to-display conversion, not an entity or series.

The BarChart view currently supports one conversion:

```text
BarChart-compatible components -> bars
```

One `BarChartVisualizerSystem` can therefore process many entities:

```text
/sales/2025 --\
/sales/2026 ----> BarChartVisualizerSystem -> several BarChartData values
/sales/2027 --/
```

A Spatial3D view is different because it combines many forms of visualization:

```text
Points3DVisualizer ---\
Boxes3DVisualizer -----\
Mesh3DVisualizer --------> Spatial3D view
CameraVisualizer --------/
ImageVisualizer --------/
```

The same framework handles both the simple one-visualizer BarChart and the many-visualizer spatial view.

## Context systems

Some visualizers need shared derived information. A context system computes that information once for the view so several visualizers can consume it.

For example, a spatial transform context can answer where entities are relative to the view origin:

```text
Transform context
    |-- Points3D visualizer
    |-- Boxes3D visualizer
    |-- Mesh3D visualizer
    `-- Camera visualizer
```

The two categories of Viewer system are therefore:

| System kind | Produces |
| --- | --- |
| Context system | Shared information used by other systems |
| Visualizer system | Draw data or typed data consumed by the view |

BarChart does not need a specialized context system; its one visualizer can prepare everything the pane requires.

## Why use this architecture?

The separation provides several benefits:

- **Composition:** one view can combine many visualizers.
- **Parallel work:** component queries and preparation can run before sequential egui layout.
- **Extensibility:** views and visualizers register through common interfaces instead of a central match statement.
- **Reuse:** shared context can serve several visualizers.
- **Consistency:** loading state and diagnostics flow through the same execution framework.

For a small view, the framework can look more complicated than a direct `query_then_draw` function. Its value becomes clearer in views such as Spatial2D and Spatial3D, where many independent visualizers share one pane.

## Reading the Rust names

When reading the Viewer source, this translation can help:

| Source name | Plain-language meaning |
| --- | --- |
| `BarChartVisualizerSystem` | BarChart data preparer |
| `VisualizerSystem::execute` | Query and prepare this visualizer's data |
| `VisualizerExecutionOutput` | Result from one visualizer |
| `SystemExecutionOutput` | Combined prepared data for one view |
| `BarChartView::ui` | Draw and interact with the BarChart pane |
| `ViewContextSystem` | Shared data preparer |
| `ViewSystemRegistrator` | View plug-in registrar |
| `ViewportUi` | Scheduler and window manager for all view panes |

The shortest mental model is:

```text
Entity = identity
Component = recorded data
System = data-processing behavior
View = presentation
Viewport = coordinator
```

Continue with [From entity to visualizer](entity-visualizer-pipeline.md) for the complete discovery, instruction, execution, and drawing pipeline.
