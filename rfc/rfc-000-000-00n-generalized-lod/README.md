# RFC-N: Generalized LOD

## Table of Contents

## 1: Summary

To this point, Maybraid has used its cascade-based LOD system in an ad hoc manner. While we propose maintaining the base cascade, we provide a common pattern for its intended usage. Namely: 

1. A variably-typed `CascadePosition` system is responsible for updating a `CascadePosition` on tracked entities, respecting updates in the game world. The tracked `CascadePosition` is the prompt for `CascadeChunk` production. 
2. A variably-typed `CascadeProduction` system is responsible for responding to changes in `CascadePositions` and spawning `CascadeChunks` with a `CascadeRequirement` level as children. It also updates a table mapping `CascadeChunks` to child entities. When a chunk needs to be culled, the `CascadeProduction` system acts directly, either marking `Hidden` or removing the chunk entity and all of its children. However, it also spawns `(CascadeChunk, ChunkCulling)` entity, for `ChunkTracker` systems which respond indirectly. 
3. A variably-typed `ChunkTracker` system is responsible for responding to new chunks and chunk culling, and dispatching tasks to meet chunk requirements. Usually, these will be tasks querying or generating over a spatial index, e.g., [RFC-142: Gimme](/rfc/rfc-000-000-142-gimme/README.md). When a `ChunkTracker` system wishes to respond directly to `ChunkProduction`, it should insert its results as children--allowing `ChunkProduction` to manage culling. Otherwise, it should not insert is results as children and should manage culling itself. 
4. A variably-typed `ChunkEntityTracker` system is responsible for responding to updates in the position of `ChunkManaged` entities--mainly so that the entities are not prematurely culled. `ChunkEntityTracker` systems use lookups to a parent `CascadeProduction` node to identify the appropriate chunk to which the child should be reattached.

## 2: Prior Art

## 3: Design

### 3.1: Cascade


![LOD Cascade](./assets/lod-cascade.png)

We adopt much of the cascade design from commit [`28b8a11`](https://github.com/ramate-io/maybraid/blob/28b8a11d59244d7826b5634387b13a1b94fc9454/util/chunk/src/cascade.rs) in Maybraid's exploratory phase. We remove, however, the embedded resolution design, preferring to delegate the task of assigning resolutions to chunks to the `ChunkTracker`, if such is fit.

We fix a **leaf scale** $s_0 > 0$ and **depth** $K \in \mathbb{N}$. Given a focal $\mathbf{p} \in \mathbb{R}^3$, the cascade itself only names a **finite** set of axis-aligned **footprints** $(B, O)$: a cube $B \subset \mathbb{R}^3$ and an optional AABB subtraction $O$ so the effective region is $B \setminus O$ when $O$ is set. A **resolution tag** $r$ is **not** part of this step; downstream, a `ChunkTracker` (or whatever policy we plug in) maps each footprint to the $r$ it needs for sampling, meshing, or Gimme queries. That keeps the cascade a **pure geometry** recipe while the exploratory code still threads a `ResolutionMap` through the same shapes for convenience.

We snap $\mathbf{p}$ to the $s_0$-lattice for the innermost cell:

$$
\mathbf{o}_0(\mathbf{p}) \;=\; s_0 \left\lfloor \frac{\mathbf{p}}{s_0} \right\rfloor,
\qquad
B_0 \;=\; [\mathbf{o}_0,\, \mathbf{o}_0 + s_0 \mathbf{1}].
$$

For each ring $k = 0,\ldots,K-1$ we use edge length $s_k = s_0\,3^{k}$. Ring $k$ is the **hollow** $3\times3\times3$ shell at that scale (27 cells minus the center, so **26** cubes). Nesting those shells with the leaf fills a single solid cube of side:

$$
\sigma \;=\; s_0\,3^{K};
$$

We call that box the **hull** $H(\mathbf{p})$.

If we want a coarse skirt, we pick $G \ge \sigma$ (often $G = \sigma\,2^m$), a bounded index set $\mathcal{I} \subset \mathbb{Z}^3$, and tile $G$-cubes around $\mathbf{p}$. We **reuse one** omission $O = H(\mathbf{p})$ on every coarse tile so we do not double-cover the hull. No config for the grid means we return an empty grid set.

We write...

$$
\mathcal{W}(\mathbf{p}) \;=\; \mathcal{W}_{\mathrm{cascade}}(\mathbf{p}) \;\cup\; \mathcal{W}_{\mathrm{grid}}(\mathbf{p})
$$

...for the geometric footprints that should be “live” at $\mathbf{p}$. When we move $\mathbf{p}' \to \mathbf{p}$, we use the cheap trigger

$$
\mathrm{needs}(\mathbf{p}, \mathbf{p}') \;\Longleftrightarrow\; \mathbf{o}_0(\mathbf{p}) \neq \mathbf{o}_0(\mathbf{p}')
$$

and the incremental chunks

$$
\Delta \mathcal{W} \;=\; \bigl(\mathcal{W}_{\mathrm{cascade}}(\mathbf{p}) \setminus \mathcal{W}_{\mathrm{cascade}}(\mathbf{p}')\bigr) \;\cup\; \bigl(\mathcal{W}_{\mathrm{grid}}(\mathbf{p}) \setminus \mathcal{W}_{\mathrm{grid}}(\mathbf{p}')\bigr).
$$

We compare footprints by the same structured identity we use in code (origin, size, omission, and so on—**excluding** resolution), so those set differences stay meaningful once $r$ is attached elsewhere.

**Construction (pseudocode).**

```rust
let s = |k: u32| s0 * 3f32.powi(k as i32); // s(k) = s0 * 3^k

let mut w_cascade = HashSet::new();
w_cascade.insert(leaf(o0(p), s0)); // geometry only; ChunkTracker picks r

let mut anchor = o0(p) - Vec3::splat(s0);
for k in 0..K {
    w_cascade.extend(hollow_shell(anchor, s(k))); // 26 cubes, not 27
    anchor -= Vec3::splat(s(k + 1));
}

let w_grid = coarse_tiles(p, G, &indices, /* omit = */ hull(p));
(w_cascade, w_grid)
```

**After a move (pseudocode).**

```rust
if needs(p_old, p_new) {
    let mut delta = HashSet::new();
    delta.extend(w_cascade(p_new).difference(&w_cascade(p_old)).copied());
    delta.extend(w_grid(p_new).difference(&w_grid(p_old)).copied());
    // ChunkTracker (or another policy) maps each footprint to a resolution r when we act.
    return (delta, w_union(p_new)); // new work only; full snapshot if we want it
}
```

This accomplishes a **nested shell LOD** at $s_0, 3s_0, 9s_0, \ldots$ and, if we ask for it, a **coarse band** with a **single hull-shaped hole**—tight detail near $\mathbf{p}$, cheaper stuff far away, without stamping the hull twice. Resolution is a **separate** knob we turn when we actually consume $\mathcal{W}$, not when we build the shells.

### 3.2: `CascadeProduction`

`CascadeProduction<S>` owns a concrete `Cascade` and listens to source data selected by `S::QueryData`. On update, it:

1. Computes and inserts/updates `CascadePosition<S::PositionData>`.
2. Ensures a local `CascadeTable<S::PositionData>` exists.
3. Asks the concrete `Cascade` which `Chunk`s should exist for the current position.
4. Spawns, updates, hides, or despawns chunk entities.
5. Emits culling marker entities for indirect listeners.

#### 3.2.1: Core components

```rust
#[derive(Component)]
pub struct CascadeProduction<S> {
    pub cascade: Cascade,
    pub marker: PhantomData<S>,
}

#[derive(Component, Clone)]
pub struct CascadePosition<S> {
    pub position: Vec3,
    pub data: S,
}

#[derive(Component)]
pub struct CascadeTable<S> {
    pub table: HashMap<Chunk, Entity>,
    pub marker: PhantomData<S>,
}

impl<S> Default for CascadeTable<S> {
    fn default() -> Self {
        Self {
            table: HashMap::default(),
            marker: PhantomData,
        }
    }
}
```

`Chunk` is concrete and already exists. `Cascade` is also concrete and decides which chunks correspond to a position.


#### 3.2.2: Requirements

Requirements are inserted directly as components. No wrapper is needed.

```rust
pub trait RequirementBuilder<R>: Component + Default + Clone {
    fn build(&self, chunk: Chunk) -> R;
}
```

If a producer entity has no builder, the system inserts `S::Builder::default()` and uses that default builder immediately.

#### 3.2.3: Source Trait

```rust
pub trait CascadeProductionSource: Send + Sync + 'static {
    type PositionData: Clone + Send + Sync + 'static;
    type Requirement: Component + Clone;
    type Builder: RequirementBuilder<Self::Requirement>;

    type QueryData: QueryData;
    type QueryFilter: QueryFilter = ();

    fn entity(item: &<Self::QueryData as QueryData>::Item<'_, '_>) -> Entity;

    fn cascade_position(
        item: &<Self::QueryData as QueryData>::Item<'_, '_>,
    ) -> CascadePosition<Self::PositionData>;

    fn cull_mode(_chunk: Chunk) -> CascadeCullMode {
        CascadeCullMode::Despawn
    }
}
```

`S` is the type marker distinguishing one production flow from another. This allows the same entity to hold multiple independent pairs like:

```rust
CascadeProduction<Foo>
CascadePosition<Foo>
CascadeTable<Foo>

CascadeProduction<Bar>
CascadePosition<Bar>
CascadeTable<Bar>
```

### 3.2.4: Culling Signal

```rust
#[derive(Clone, Copy)]
pub enum CascadeCullMode {
    Hide,
    Despawn,
}

#[derive(Component)]
pub struct ChunkCulling {
    pub mode: CascadeCullMode,
}
```

When a chunk is culled, production acts directly on the chunk entity and also spawns:

```rust
(chunk, ChunkCulling { mode })
```

...for indirect listeners.


### 3.2.5: System

```rust
pub fn produce_cascade<S>(
    mut commands: Commands,
    mut query: Query<
        (
            S::QueryData,
            &CascadeProduction<S>,
            Option<&CascadePosition<S::PositionData>>,
            Option<&mut CascadeTable<S::PositionData>>,
            Option<&S::Builder>,
        ),
        S::QueryFilter,
    >,
)
where
    S: CascadeProductionSource,
{
    for (item, production, old_position, table, builder) in &mut query {
        let entity = S::entity(&item);

        let position = update_cascade_position::<S>(
            &mut commands,
            entity,
            &item,
            old_position,
        );

        let builder = resolve_requirement_builder::<S>(
            &mut commands,
            entity,
            builder,
        );

        update_cascade_chunks::<S>(
            &mut commands,
            entity,
            production,
            table,
            &position,
            &builder,
        );
    }
}
```

#### 3.2.6: Phase 1: Update `CascadePosition`

```rust
fn update_cascade_position<S>(
    commands: &mut Commands,
    entity: Entity,
    item: &<S::QueryData as QueryData>::Item<'_, '_>,
    old_position: Option<&CascadePosition<S::PositionData>>,
) -> CascadePosition<S::PositionData>
where
    S: CascadeProductionSource,
{
    let position = S::cascade_position(item);

    if old_position
        .map(|old| old.position != position.position)
        .unwrap_or(true)
    {
        commands.entity(entity).insert(position.clone());
    }

    position
}
```

You may want `PositionData: PartialEq` if you also want to skip inserts when only `data` is unchanged.

#### 3.2.7: Phase 2: Resolve Builder

```rust
fn resolve_requirement_builder<S>(
    commands: &mut Commands,
    entity: Entity,
    builder: Option<&S::Builder>,
) -> S::Builder
where
    S: CascadeProductionSource,
{
    match builder {
        Some(builder) => builder.clone(),
        None => {
            let builder = S::Builder::default();
            commands.entity(entity).insert(builder.clone());
            builder
        }
    }
}
```

#### 3.2.8: Phase 3: Update Chunks

```rust
fn update_cascade_chunks<S>(
    commands: &mut Commands,
    entity: Entity,
    production: &CascadeProduction<S>,
    table: Option<&mut <CascadeTable<S::PositionData>>>,
    position: &CascadePosition<S::PositionData>,
    builder: &S::Builder,
)
where
    S: CascadeProductionSource,
{

    // todo: we actually want to put CascadeTable inside of CascadeProduction, we can thus remove the generic.
    // this is because we always want there to be a table. 
    let Some(mut table) = table else {
        commands
            .entity(entity)
            .insert(CascadeTable::<S::PositionData>::default());
        return;
    };

    let desired: Vec<Chunk> = production
        .cascade
        .chunks_for_position(position.position)
        .collect();

    spawn_or_update_chunks::<S>(
        commands,
        entity,
        &mut table,
        &desired,
        builder,
    );

    cull_stale_chunks::<S>(
        commands,
        &mut table,
        &desired,
    );
}
```

#### 3.2.9: Spawn Update Chunks

```rust
fn spawn_or_update_chunks<S>(
    commands: &mut Commands,
    producer: Entity,
    table: &mut CascadeTable<S::PositionData>,
    desired: &[Chunk],
    builder: &S::Builder,
)
where
    S: CascadeProductionSource,
{
    for &chunk in desired {
        let requirement = builder.build(chunk);

        match table.table.get(&chunk).copied() {
            Some(chunk_entity) => {
                commands.entity(chunk_entity).insert(requirement);
            }
            None => {
                let chunk_entity = commands
                    .spawn((chunk, requirement))
                    .id();

                commands.entity(producer).add_child(chunk_entity);
                table.table.insert(chunk, chunk_entity);
            }
        }
    }
}
```

#### 3.2.10: Cull stale chunks

```rust
fn cull_stale_chunks<S>(
    commands: &mut Commands,
    table: &mut CascadeTable<S::PositionData>,
    desired: &[Chunk],
)
where
    S: CascadeProductionSource,
{
    let desired: HashSet<Chunk> = desired.iter().copied().collect();

    let stale: Vec<Chunk> = table
        .table
        .keys()
        .filter(|chunk| !desired.contains(chunk))
        .copied()
        .collect();

    for chunk in stale {
        let Some(chunk_entity) = table.table.remove(&chunk) else {
            continue;
        };

        let mode = S::cull_mode(chunk);

        match mode {
            CascadeCullMode::Hide => {
                commands.entity(chunk_entity).insert(Visibility::Hidden);
            }
            CascadeCullMode::Despawn => {
                commands.entity(chunk_entity).despawn_recursive();
            }
        }

        commands.spawn((chunk, ChunkCulling { mode }));
    }
}
```

#### 3.2.11: Plugin

```rust
pub struct CascadeProductionPlugin<S> {
    marker: PhantomData<S>,
}

impl<S> Default for CascadeProductionPlugin<S> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<S> Plugin for CascadeProductionPlugin<S>
where
    S: CascadeProductionSource,
{
    fn build(&self, app: &mut App) {
        app.add_systems(Update, produce_cascade::<S>);
    }
}
```

### 3.3: `ChunkTracker`

#### 3.3.1: Integration with RFC-142: Gimme

The proposed spatial storage engine is currently [Gimme](/rfc/rfc-000-000-142-gimme/README.md). Accordingly, we have prepared an integration guide [here](./integration-with-gimme/README.md).

### 3.4: `ChunkEntityTracker`

## Milestones