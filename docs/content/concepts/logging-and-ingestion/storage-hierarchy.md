---
title: Recording and blueprint storage hierarchy
order: 250
description: How applications, stores, entities, chunks, rows, and components fit together
---

Rerun uses the same storage model for two different kinds of data:

- A **recording store** contains the data to visualize.
- A **blueprint store** contains instructions for how the Viewer should visualize it.

Both are built from entity paths, temporal rows, component batches, and Arrow-backed chunks. This page explains how those concepts relate.

## The short version

The following concrete example shows what the logical hierarchy actually contains. The IDs have been shortened to keep it readable; real store, row, view, container, and instruction IDs are longer.

```text
ApplicationId: robot_simulator
|
|-- Recording StoreId: Recording:robot_simulator:run_001
|   `-- EntityDb
|       |
|       |-- EntityPath: /__properties
|       |   |-- RecordingInfo:name
|       |   |     type  = rerun.components.Name
|       |   |     value = "Downtown drive"
|       |   |
|       |   `-- RecordingInfo:start_time
|       |         type  = rerun.components.Timestamp
|       |         value = 2026-09-04T10:00:00Z
|       |
|       |-- EntityPath: /world/lidar
|       |   |
|       |   |-- Points3D:positions
|       |   |     type = rerun.components.Position3D
|       |   |     history:
|       |   |       frame=41, RowId=row_101 -> [[1,2,3], [4,5,6]]
|       |   |       frame=42, RowId=row_104 -> [[1.1,2,3], [4.1,5,6]]
|       |   |
|       |   |-- Points3D:colors
|       |   |     type = rerun.components.Color
|       |   |     static, RowId=row_100 -> [red, green]
|       |   |
|       |   `-- Points3D:radii
|       |         type = rerun.components.Radius
|       |         static, RowId=row_100 -> [0.025]
|       |
|       `-- EntityPath: /metrics/speed
|           `-- Scalars:scalars
|                 type = rerun.components.Scalar
|                 history:
|                   frame=41, RowId=row_102 -> [12.4]
|                   frame=42, RowId=row_105 -> [13.1]
|
`-- Blueprint StoreId: Blueprint:robot_simulator:layout_A
    `-- EntityDb
        |
        |-- EntityPath: /top_panel
        |   `-- PanelBlueprint:state -> Expanded
        |
        |-- EntityPath: /viewport
        |   |-- ViewportBlueprint:root_container -> ContainerId=C1
        |   |-- ViewportBlueprint:auto_layout    -> false
        |   `-- ViewportBlueprint:auto_views     -> false
        |
        |-- EntityPath: /container/C1
        |   |-- ContainerBlueprint:container_kind -> Horizontal
        |   |-- ContainerBlueprint:contents
        |   |     -> [/view/V1, /view/V2]
        |   `-- ContainerBlueprint:col_shares
        |         -> [2.0, 1.0]
        |
        |-- EntityPath: /view/V1
        |   |-- ViewBlueprint:class_identifier -> Spatial3D
        |   |-- ViewBlueprint:display_name      -> "Lidar"
        |   |-- ViewBlueprint:space_origin      -> /world
        |   |
        |   |-- EntityPath: /view/V1/ViewContents
        |   |   `-- ViewContents:query -> ["+ /world/lidar/**"]
        |   |
        |   `-- EntityPath: /view/V1/defaults
        |       `-- Points3D:radii -> [0.04]
        |
        `-- EntityPath: /view/V2
            |-- ViewBlueprint:class_identifier -> TimeSeries
            |-- ViewBlueprint:display_name      -> "Speed"
            |
            |-- EntityPath: /view/V2/ViewContents
            |   `-- ViewContents:query -> ["+ /metrics/speed"]
            |
            |-- EntityPath: /view/V2/PlotLegend
            |   |-- PlotLegend:visible -> true
            |   `-- PlotLegend:corner  -> RightTop
            |
            `-- EntityPath:
                /view/V2/ViewContents/overrides/metrics/speed/visualizers/I1
                |-- VisualizerInstruction:visualizer_type -> SeriesLines
                `-- component mappings and overrides for this instruction
```

Read one recording leaf from left to right:

```text
Recording:robot_simulator:run_001
    + /world/lidar
    + Points3D:positions
    + frame 42
    -> RowId row_104
    -> component batch [[1.1,2,3], [4.1,5,6]]
```

Read one blueprint leaf the same way:

```text
Blueprint:robot_simulator:layout_A
    + /view/V2/PlotLegend
    + PlotLegend:corner
    -> RightTop
```

The tree above is a logical view. The values shown at its leaves are physically encoded as component batches in rows and chunks. For example, the two `/world/lidar` updates can be rows in a chunk tagged with the `/world/lidar` entity path.

The physical storage hierarchy inside either `EntityDb` is:

```text
EntityDb
    `-- StorageEngine
          |-- ChunkStore
          |     `-- ChunkId -> Chunk
          |           |-- one EntityPath
          |           |-- RowId column
          |           |-- zero or more timeline columns
          |           `-- zero or more component columns
          |                 `-- one component batch per populated row
          |
          `-- QueryCache
```

These are two views of the same data:

- The **logical view** groups values by entity and component.
- The **physical view** groups rows into chunks for efficient transport, storage, and queries.

## One important correction

It is tempting to draw this:

```text
EntityPath
    |-- ChunkId
    `-- RowId
```

That is useful as a first approximation, but it is not a strict ownership tree.

The more precise relationships are:

```text
Chunk
    |-- has one ChunkId
    |-- is tagged with one EntityPath
    `-- contains several rows
          `-- each row has one RowId
```

An entity can have many chunks, and a chunk belongs to one entity path. However:

- A `ChunkId` does not encode an `EntityPath`.
- A `RowId` does not encode its `ChunkId`.
- A row can appear in more than one physical chunk during processing.
- `ChunkId` and `RowId` are explicitly independent identifiers.

Think of entity paths as the logical organization and chunks as physical packages carrying parts of that logical data.

## IDs at a glance

| Identifier | Identifies | Scope |
| --- | --- | --- |
| `ApplicationId` | A related family of recordings and blueprints | Viewer/application |
| `StoreId` | One logical recording or blueprint store | Globally meaningful store key |
| `EntityPath` | One entity inside a store | Unique within a `StoreId` |
| `ComponentIdentifier` | One component column at an entity | Unique at an `EntityPath` |
| `ChunkId` | One physical chunk | Assumed unique within a recording |
| `RowId` | One logical logged row | Normally globally unique |
| `ViewId` | One view instance in a blueprint | Unique within blueprint contents |
| `ContainerId` | One layout container in a blueprint | Unique within blueprint contents |
| `VisualizerInstructionId` | One configured use of a visualizer | Within a view's configuration |

The IDs answer different questions:

```text
ApplicationId:            Which family of recordings is this?
StoreId:                  Which database?
EntityPath:               Which logical entity in that database?
ComponentIdentifier:      Which data field on that entity?
Timeline + time:          At what logical time?
RowId:                    Which logged update wins if times tie?
ChunkId:                  Which physical package contains rows?
ViewId:                   Which Viewer pane?
VisualizerInstructionId: Which configured visualization of an entity?
```

## Application ID: grouping related stores

An `ApplicationId` is a user-facing name such as:

```text
robot_simulator
training_run
bar_chart_example
```

It groups recordings that have compatible meaning. In the local Viewer, recordings with the same application ID can use the same blueprint.

For example:

```text
ApplicationId = robot_simulator
    |
    |-- recording run_001
    |-- recording run_002
    `-- shared Viewer blueprint
```

The `ApplicationId` is not itself a database and does not directly contain entity data. It is part of each `StoreId` and is used by the Viewer to associate stores.

For catalog-backed data, the catalog object model uses dataset entries and segments instead. See [Recordings](recordings.md) and [Catalog object model](../query-and-transform/catalog-object-model.md).

## Store ID: identifying a whole database

A `StoreId` contains three pieces:

```rust
StoreId {
    kind,
    application_id,
    recording_id,
}
```

For example:

```text
Recording:robot_simulator:run_001
Blueprint:robot_simulator:550e8400-e29b-41d4-a716-446655440000
```

The `kind` is either:

```text
Recording
Blueprint
```

A `StoreId` identifies an entire `EntityDb`, not a single entity.

```text
StoreId
    `-- EntityDb
          |-- /camera
          |-- /lidar
          |-- /robot
          `-- /metrics/speed
```

### Store merging

All incoming data with the same `StoreId` is treated as part of the same logical store.

Two processes can intentionally contribute to one recording:

```text
process A -> Recording:robot:run_001 -> /camera
process B -> Recording:robot:run_001 -> /lidar
```

The Viewer pools the data:

```text
Recording:robot:run_001
    |-- /camera
    `-- /lidar
```

Accidentally reusing a `StoreId` can therefore merge unrelated data.

## StoreBundle and StoreHub

At runtime, the Viewer usually has several stores open.

A `StoreBundle` is the basic collection:

```rust
StoreBundle {
    stores: IndexMap<StoreId, EntityDb>,
}
```

Conceptually:

```text
StoreBundle
    |-- StoreId A -> recording EntityDb
    |-- StoreId B -> recording EntityDb
    `-- StoreId C -> blueprint EntityDb
```

`StoreHub` is the Viewer-level coordinator around that bundle. In addition to the stores, it tracks:

- The active and default blueprint for each application.
- Per-store Viewer caches and subscribers.
- Store usage and opening order.
- Local table stores.
- Other Viewer lifecycle state.

The distinction is:

```text
StoreBundle
    owns StoreId -> EntityDb

StoreHub
    coordinates those stores for the Viewer
```

## EntityDb: one recording or blueprint

An `EntityDb` is the in-memory owner for one `StoreId`.

It combines:

```text
Store identity
StorageEngine
Entity-path indexes
Timeline metadata
Manifest and remote-fetch state
Ingestion statistics
Viewer-oriented indexes
```

Its storage engine contains:

```text
ChunkStore
    owns and indexes chunks

QueryCache
    caches resolved query results
```

The same `EntityDb` implementation is used for recordings and blueprints. Their `StoreKind` and component semantics distinguish them.

## Entity paths: logical identities inside a store

An `EntityPath` identifies one logical thing inside an `EntityDb`:

```text
/world/car
/world/car/camera
/world/car/lidar
/metrics/speed
```

Entity paths form a hierarchy:

```text
/
|-- world
|   `-- car
|       |-- camera
|       `-- lidar
`-- metrics
    `-- speed
```

Intermediate paths do not need stored data. Logging `/world/car/camera` implicitly creates the path hierarchy needed to address it.

The same path can exist independently in several stores:

```text
StoreId run_001: /camera
StoreId run_002: /camera
```

Therefore, a globally meaningful entity address starts with:

```text
(StoreId, EntityPath)
```

See [The Entity Path Hierarchy](entity-path.md) for path syntax and hierarchy behavior.

## Components: fields attached to an entity

Components contain the actual values associated with an entity:

```text
/world/car
    Transform3D:translation
    Transform3D:rotation

/world/car/lidar
    Points3D:positions
    Points3D:colors
    Points3D:radii
```

A component column is described by a `ComponentDescriptor`:

```rust
ComponentDescriptor {
    archetype,
    component,
    component_type,
}
```

For example:

```text
archetype:      rerun.archetypes.Points3D
component:      Points3D:positions
component_type: rerun.components.Position3D
```

The fields have different jobs:

| Field | Meaning |
| --- | --- |
| Archetype name | Which semantic bundle introduced the field |
| Component identifier | The field's role, such as `Points3D:positions` |
| Component type | How to interpret each value, such as `Position3D` |

At a given entity path, the component identifier selects the logical component history:

```text
(StoreId, EntityPath, ComponentIdentifier)
```

## Rows: one logical update

One logical row groups one or more component batches with:

- One entity path.
- One `RowId`.
- A time point.

Conceptually:

```text
Row
    entity path = /world/points
    row id      = row_...
    time point:
        frame    = 42
        log_time = 2026-09-04T10:00:00Z
    components:
        Points3D:positions = [[1, 2, 3], [4, 5, 6]]
        Points3D:colors    = [red]
```

### RowId

A `RowId` identifies a logical row update. Fresh row IDs are normally monotonic and unique.

It has several important jobs:

- Distinguishing separate logging operations.
- Breaking ties when multiple rows have the same timeline value.
- Helping caches identify immutable component values.
- Approximating logging order for garbage collection.

If two values have the same entity, component, timeline, and time, the greater `RowId` wins a latest-at query.

A `RowId` is not a child encoded inside a `ChunkId`. The two identifiers are independent.

### TimePoint

A row can have a value on several timelines:

```text
frame    = 42
log_tick = 105
log_time = 2026-09-04T10:00:00Z
```

This collection is the row's time point.

A static row has no ordinary timeline columns and applies across all times. See [Static data](static.md).

## Component batches and instances

The value of one component in one row is always a batch:

```text
Points3D:positions =
    [[1, 2, 3],
     [4, 5, 6],
     [7, 8, 9]]
```

This batch contains three component instances.

Another component in the same row can have a different batch length:

```text
Points3D:colors = [red]
```

Viewer archetype semantics may clamp that single color across all three positions:

```text
position 0 -> red
position 1 -> red
position 2 -> red
```

An instance is not normally a separate Rerun entity. One `/lidar` entity can contain a batch of 100,000 `Position3D` instances.

See [Component Batches](batches.md) for joining and clamping semantics.

## Chunks: physical packages of rows

A `Chunk` is the main physical unit of:

- Ingestion
- Transport
- Storage
- Query indexing
- Store events
- Compaction
- Garbage collection

Each chunk has:

```text
ChunkId
EntityPath
RowId column
Timeline columns
Component columns
```

A simplified chunk looks like:

```text
ChunkId: chunk_ABC
EntityPath: /world/points

RowId       frame    Points3D:positions       Points3D:colors
---------   -----    ----------------------   ---------------
row_001     40       [[0,0,0], [1,1,1]]      [red]
row_002     41       [[2,2,2], [3,3,3]]      null
row_003     42       [[4,4,4], [5,5,5]]      [blue, green]
```

Important properties:

- One chunk contains data for exactly one entity path.
- One entity path can have many chunks.
- A chunk contains one or more rows.
- Component columns are sparse, so a row can omit a component.
- Every populated component cell is an Arrow list containing a component batch.

### ChunkId

A `ChunkId` identifies one physical chunk.

Chunk IDs are assumed unique within one recording. If the chunk store receives the same `ChunkId` twice, it treats both values as the same chunk and keeps only one.

A transformed, split, compacted, or otherwise modified chunk receives a new `ChunkId`.

The chunk ID says nothing about:

- The entity path.
- The time range.
- Its row IDs.
- Its components.

Those properties are stored as chunk metadata and columns.

## Logical and physical views together

Suppose the SDK logs:

```rust
rec.set_time_sequence("frame", 42);
rec.log(
    "world/points",
    &rerun::Points3D::new([
        [1.0, 2.0, 3.0],
        [4.0, 5.0, 6.0],
    ])
    .with_colors([[255, 0, 0]]),
)?;
```

The logical interpretation is:

```text
ApplicationId: my_app
    `-- Recording StoreId: my_app/run_001
          `-- EntityPath: /world/points
                |-- Points3D:positions history
                `-- Points3D:colors history
```

The new logical row is:

```text
RowId: row_123
TimePoint: frame=42

Points3D:positions batch:
    [[1,2,3], [4,5,6]]

Points3D:colors batch:
    [red]
```

The physical representation is eventually part of a chunk:

```text
ChunkId: chunk_456
EntityPath: /world/points

RowId     frame    Points3D:positions       Points3D:colors
-------   -----    ----------------------   ---------------
row_123   42       [[1,2,3], [4,5,6]]      [red]
```

The SDK batcher may append more rows for `/world/points` to that chunk before sending it.

## Blueprint stores use the same data model

A blueprint is not a separate storage technology. It is another `EntityDb` containing components in chunks.

The difference is what the entity paths and components mean:

```text
Recording store
    entity paths represent user data

Blueprint store
    entity paths represent Viewer configuration
```

A representative blueprint hierarchy is:

```text
Blueprint StoreId
    |
    |-- /top_panel
    |-- /blueprint_panel
    |-- /selection_panel
    |-- /time_panel
    |
    |-- /viewport
    |
    |-- /container/<ContainerId>
    |
    `-- /view/<ViewId>
          |-- /ViewContents
          |-- /PlotBackground
          |-- /PlotLegend
          |-- /defaults
          `-- /ViewContents/overrides/<recording-entity>/visualizers/<instruction-id>
```

These paths are internal implementation details and may evolve, but they illustrate how Viewer configuration maps onto the normal entity/component model.

### `/viewport`

The viewport path stores top-level layout information such as:

- Root container ID.
- Maximized view ID.
- Auto-layout state.
- Auto-view state.
- Previously processed view recommendations.

### `/container/<ContainerId>`

Each container entity stores:

- Container kind: tabs, horizontal, vertical, or grid.
- Child `ViewId`s and `ContainerId`s.
- Display name.
- Column or row shares.
- Active tab.
- Visibility.
- Grid column count.

`ContainerId` is UUID-backed and becomes part of the container's blueprint entity path.

### `/view/<ViewId>`

Each view entity stores:

- View class, such as `BarChart` or `Spatial3D`.
- Display name.
- Space origin.
- Visibility.

`ViewId` identifies one pane instance, not a view type.

For example:

```text
/view/A -> class BarChart, displays /sales
/view/B -> class BarChart, displays /latency
```

Both use the `BarChart` view class but have different `ViewId`s and configuration.

### View contents

The `ViewContents` property stores entity-query expressions:

```text
+ /**
- /debug/**
```

These expressions decide which recording entity paths are candidates for the view.

### View properties

View-specific properties are stored below the view path using their archetype short name:

```text
/view/<ViewId>/PlotBackground
/view/<ViewId>/PlotLegend
```

For a BarChart view, `PlotLegend` can contain:

```text
PlotLegend:visible
PlotLegend:corner
```

### Defaults and overrides

Blueprints can also store:

- Default component values for a view.
- Per-entity component overrides.
- Visualizer instructions.
- Component source mappings for those instructions.

This is why a visualizer query resolves more than recording data. Its priority can include:

```text
Blueprint override
    -> recording value
        -> blueprint default
            -> fallback provider
                -> generic placeholder
```

### Blueprint updates are temporal rows

When the user edits a blueprint property, the Viewer does not mutate a serialized struct in place. It logs another component row into the blueprint store.

The Viewer then uses latest-at queries on the blueprint timeline to reconstruct current state:

```text
older row: PlotLegend:corner = RightTop
newer row: PlotLegend:corner = LeftBottom

current blueprint value = LeftBottom
```

This reuse of temporal storage supports blueprint history, undo, and redo.

## How RRD files preserve the hierarchy

An RRD file is a sequence of framed messages. The main application-level messages are:

```text
SetStoreInfo
    announces StoreId and store metadata

ArrowMsg(StoreId, chunk)
    associates one encoded chunk with a store

BlueprintActivationCommand
    activates a fully transmitted blueprint
```

One RRD can contain multiple stores:

```text
file.rrd
    |-- Recording StoreId A
    |     `-- chunks
    |-- Recording StoreId B
    |     `-- chunks
    `-- Blueprint StoreId C
          `-- chunks
```

The `.rrd` and `.rbl` extensions use the same file format:

- `.rrd` conventionally contains recording data and may also contain blueprints.
- `.rbl` conventionally contains one blueprint.

The file does not need a nested directory structure. Each message carries enough metadata to reconstruct:

```text
StoreId -> EntityDb -> chunks -> entity/component histories
```

See [RRD format](rrd-format.md) for framing and encoding details.

## How a query locates data

A latest-at component query conceptually provides:

```text
StoreId
EntityPath
Timeline
Time
ComponentIdentifier
```

For example:

```text
store:     Recording:robot:run_001
entity:    /world/car
timeline:  frame
time:      42
component: Transform3D:translation
```

The query process is approximately:

```text
StoreBundle uses StoreId to find EntityDb
    |
    v
ChunkStore indexes find candidate chunks for:
    EntityPath + Timeline + ComponentIdentifier
    |
    v
Rows at or before time 42 are considered
    |
    v
Time chooses the latest candidate
    |
    v
RowId breaks an exact timestamp tie
    |
    v
The component batch is returned
```

The query result may contain several instances because the selected component cell is a batch.

## What is persisted versus derived?

Not every runtime object is stored.

### Persisted or transported

```text
StoreId and StoreInfo
ChunkId
EntityPath
RowId
Timeline values
Component descriptors
Component batches
Blueprint configuration components
```

### Reconstructed or cached at runtime

```text
Entity path trees
Chunk time indexes
QueryCache entries
Visualizable-entity sets
Indicated-entity sets
ViewportBlueprint Rust structures
ViewQuery
SystemExecutionOutput
BarChartData and other visualizer outputs
```

For example, `BarChartData` is derived each frame from stored components. It is not written into the recording.

## Table data is separate

This hierarchy describes recording and blueprint stores carried by `LogMsg` and RRD files.

Rerun also has table-oriented data used by catalog and dataframe workflows. The Viewer keeps local table stores separately:

```text
StoreHub
    |-- StoreBundle<StoreId, EntityDb>
    `-- TableStores<TableId, TableStore>
```

`TableMsg` is intentionally not a `LogMsg`, and table stores are not serialized into RRD files through this recording hierarchy.

## How TestContext mirrors the hierarchy

`TestContext` creates a miniature version of the same runtime structure:

```text
TestContext
    |
    |-- recording_store_id
    |
    `-- StoreHub
          `-- StoreBundle
                |-- recording StoreId -> recording EntityDb
                `-- blueprint StoreId -> blueprint EntityDb
```

When a test calls:

```rust
test_context.log_entity("time_series", |builder| {
    // Add components.
});
```

the two addressing steps are:

```text
recording_store_id
    selects the recording EntityDb

/time_series
    selects the entity inside that EntityDb
```

When a test creates a view:

```text
blueprint StoreId
    selects the blueprint EntityDb

ViewId
    becomes /view/<uuid> inside that blueprint
```

This is why tests need both store IDs and entity/view IDs.

## A better overall hierarchy

The most complete compact model is:

```text
ApplicationId
    |
    |-- Recording StoreId
    |     `-- EntityDb
    |           `-- StorageEngine
    |                 |-- ChunkStore
    |                 |     `-- ChunkId -> Chunk
    |                 |           |-- EntityPath
    |                 |           |-- RowId[]
    |                 |           |-- TimelineColumn[]
    |                 |           `-- ComponentColumn[]
    |                 |                 `-- ComponentBatch per row
    |                 |                       `-- component instances
    |                 `-- QueryCache
    |
    `-- Blueprint StoreId
          `-- EntityDb
                `-- same chunk/row/component representation
                      |
                      |-- /viewport
                      |-- /container/<ContainerId>
                      |-- /view/<ViewId>
                      `-- panel and view-property paths
```

Remember that the lower physical portion is indexed in several ways. It is not literally stored as nested objects in exactly this shape.

## Final mental model

```text
ApplicationId
    groups related recordings and blueprints

StoreId
    selects one complete recording or blueprint database

EntityPath
    selects one logical entity inside that database

ComponentIdentifier
    selects one data field on that entity

Timeline + Time
    selects when to read that field

RowId
    identifies the logged update and breaks timestamp ties

ChunkId
    identifies the physical package carrying rows

Component batch
    is the value stored in one populated component cell

Instance
    is one element inside that batch

ViewId
    identifies one Viewer pane stored as entities/components in a blueprint
```

The central idea is:

> Rerun stores both user data and Viewer configuration as component batches on entity paths, grouped into temporal rows and Arrow chunks inside a store identified by `StoreId`.
