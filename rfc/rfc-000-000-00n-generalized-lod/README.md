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

Updated sketch, staying within the attached format. This revises the prior `CascadeProduction` proposal in the directions you listed. 

### 3.2: `CascadeProduction`

`CascadeProduction<S>` owns a concrete `Cascade`, maintains its local chunk table, and listens to source data selected by `S::QueryData`. On update, it:

1. Computes and inserts/updates `CascadePosition<S::PositionData>`.
2. Uses `Cascade::new_chunks(previous, current)` to identify newly relevant chunks.
3. Runs the requirement builder over both new chunks and existing chunks.
4. Uses the requirement’s signal to decide whether each chunk should be visible, hidden, or removed.
5. Spawns culling marker entities for indirect listeners when chunks are hidden or removed.

#### 3.2.1: Core Components

```rust
#[derive(Component)]
pub struct CascadeProduction<S> {
    pub cascade: Cascade,
    pub table: CascadeTable,
    pub marker: PhantomData<S>,
}

pub struct CascadeTable {
    pub table: HashMap<Chunk, Entity>,
}

#[derive(Component, Clone)]
pub struct CascadePosition<S> {
    pub previous: Option<AaBb>,
    pub current: AaBb,
    pub data: S,
}
```

`Chunk`, `Cascade`, and `AaBb` are concrete types. `S` is only used to distinguish typed production flows.

#### 3.2.2: Requirement Signal

Requirements are inserted directly as components. They also describe the desired endemic action for a chunk.

```rust
pub enum RequirementSignal {
    Visible,
    Hidden,
    Remove,
}

pub trait CascadeRequirement: Component + Clone {
    fn signal(&self) -> RequirementSignal;
}
```

The name `RequirementSignal` is probably clearer than `Signal`, because it avoids colliding with broader engine/event terminology.

#### 3.2.3: Requirement Builder

```rust
pub trait RequirementBuilder<R>: Component + Default + Clone
where
    R: CascadeRequirement,
{
    fn build<S>(
        &self,
        position: &CascadePosition<S>,
        chunk: Chunk,
    ) -> R;
}
```

The builder receives both the current cascade position and the chunk, so requirements can depend on metadata in `CascadePosition<S>` as well as spatial chunk identity.

#### 3.2.4: Source Trait

```rust
pub trait CascadeProductionSource: Send + Sync + 'static {
    type PositionData: Clone + Send + Sync + 'static;
    type Requirement: CascadeRequirement;
    type Builder: RequirementBuilder<Self::Requirement>;

    type QueryData: QueryData;
    type QueryFilter: QueryFilter = ();

    fn entity(item: &<Self::QueryData as QueryData>::Item<'_, '_>) -> Entity;

    fn current_position(
        item: &<Self::QueryData as QueryData>::Item<'_, '_>,
    ) -> AaBb;

    fn position_data(
        item: &<Self::QueryData as QueryData>::Item<'_, '_>,
    ) -> Self::PositionData;
}
```

#### 3.2.5: Culling Marker

```rust
#[derive(Component)]
pub struct ChunkCulling {
    pub signal: RequirementSignal,
}
```

When a chunk is hidden or removed, production acts directly and also spawns:

```rust
(chunk, ChunkCulling { signal })
```

...for indirect `ChunkTracker` listeners.

#### 3.2.6: System

```rust
pub fn produce_cascade<S>(
    mut commands: Commands,
    mut query: Query<
        (
            S::QueryData,
            &mut CascadeProduction<S>,
            Option<&CascadePosition<S::PositionData>>,
            Option<&S::Builder>,
        ),
        S::QueryFilter,
    >,
)
where
    S: CascadeProductionSource,
{
    for (item, mut production, old_position, builder) in &mut query {
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
            &mut production,
            &position,
            &builder,
        );
    }
}
```

#### 3.2.7: Phase 1: Update `CascadePosition`

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
    let current = S::current_position(item);

    let position = CascadePosition {
        previous: old_position.map(|old| old.current),
        current,
        data: S::position_data(item),
    };

    commands.entity(entity).insert(position.clone());

    position
}
```

#### 3.2.8: Phase 2: Resolve Builder

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

#### 3.2.9: Phase 3: Update Chunks

```rust
fn update_cascade_chunks<S>(
    commands: &mut Commands,
    producer: Entity,
    production: &mut CascadeProduction<S>,
    position: &CascadePosition<S::PositionData>,
    builder: &S::Builder,
)
where
    S: CascadeProductionSource,
{
    let new_chunks: Vec<Chunk> = production
        .cascade
        .new_chunks(position.previous, position.current)
        .collect();

    apply_requirements_to_new_chunks::<S>(
        commands,
        producer,
        production,
        position,
        builder,
        &new_chunks,
    );

    apply_requirements_to_existing_chunks::<S>(
        commands,
        production,
        position,
        builder,
    );
}
```

#### 3.2.10: Apply Requirements to New Chunks

```rust
fn apply_requirements_to_new_chunks<S>(
    commands: &mut Commands,
    producer: Entity,
    production: &mut CascadeProduction<S>,
    position: &CascadePosition<S::PositionData>,
    builder: &S::Builder,
    new_chunks: &[Chunk],
)
where
    S: CascadeProductionSource,
{
    for &chunk in new_chunks {
        let requirement = builder.build(position, chunk);

        match requirement.signal() {
            RequirementSignal::Visible => {
                let chunk_entity = production
                    .table
                    .table
                    .get(&chunk)
                    .copied()
                    .unwrap_or_else(|| {
                        let entity = commands.spawn(chunk).id();
                        commands.entity(producer).add_child(entity);
                        production.table.table.insert(chunk, entity);
                        entity
                    });

                commands
                    .entity(chunk_entity)
                    .insert((requirement, Visibility::Visible));
            }

            RequirementSignal::Hidden => {
                let chunk_entity = production
                    .table
                    .table
                    .get(&chunk)
                    .copied()
                    .unwrap_or_else(|| {
                        let entity = commands.spawn(chunk).id();
                        commands.entity(producer).add_child(entity);
                        production.table.table.insert(chunk, entity);
                        entity
                    });

                commands
                    .entity(chunk_entity)
                    .insert((requirement, Visibility::Hidden));

                commands.spawn((
                    chunk,
                    ChunkCulling {
                        signal: RequirementSignal::Hidden,
                    },
                ));
            }

            RequirementSignal::Remove => {
                commands.spawn((
                    chunk,
                    ChunkCulling {
                        signal: RequirementSignal::Remove,
                    },
                ));
            }
        }
    }
}
```

#### 3.2.11: Apply Requirements to Existing Chunks

```rust
fn apply_requirements_to_existing_chunks<S>(
    commands: &mut Commands,
    production: &mut CascadeProduction<S>,
    position: &CascadePosition<S::PositionData>,
    builder: &S::Builder,
)
where
    S: CascadeProductionSource,
{
    let existing: Vec<(Chunk, Entity)> = production
        .table
        .table
        .iter()
        .map(|(&chunk, &entity)| (chunk, entity))
        .collect();

    for (chunk, entity) in existing {
        let requirement = builder.build(position, chunk);

        match requirement.signal() {
            RequirementSignal::Visible => {
                commands
                    .entity(entity)
                    .insert((requirement, Visibility::Visible));
            }

            RequirementSignal::Hidden => {
                commands
                    .entity(entity)
                    .insert((requirement, Visibility::Hidden));

                commands.spawn((
                    chunk,
                    ChunkCulling {
                        signal: RequirementSignal::Hidden,
                    },
                ));
            }

            RequirementSignal::Remove => {
                production.table.table.remove(&chunk);

                commands.entity(entity).despawn_recursive();

                commands.spawn((
                    chunk,
                    ChunkCulling {
                        signal: RequirementSignal::Remove,
                    },
                ));
            }
        }
    }
}
```

#### 3.2.12: Plugin

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

The main structural change is that `CascadeProduction<S>` is now the durable owner of both the concrete `Cascade` and the chunk table. That removes the optional table path entirely and makes each typed production flow self-contained.


### 3.3: `ChunkTracker`

#### 3.3.1: Integration with RFC-142: Gimme

The proposed spatial storage engine is currently [Gimme](/rfc/rfc-000-000-142-gimme/README.md). Accordingly, we have prepared an integration guide [here](./integration-with-gimme/README.md).

### 3.4: `ChunkEntityTracker`

## Milestones