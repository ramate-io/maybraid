# RFC-N: Gimme BVH

## Table of Contents

## 1: Motivation

Gimme BVH is a simple AaBb-based BVH system proposed for spatial storage, querying, and retrieval system for Maybraid. It seeks to address the needs to efficiently determine spatial entities available within an AaBb region, as well as to handle generation. The API is designed with Bevy in mind and is intended to roughly intended to handle the following operations:

> [!NOTE]
> The name is a play on words. AaBb is an anagram for the band ABBA. ABBA has a song called "Gimme! Gimme! Gimme!" The system is also concerned with "giving me" spatially indexed objects. 

```rust
pub trait BvhObject<B: Bvh> {

    /// The global identifier of the object.
    /// This should be consistent across sessions. 
    fn id(&self) -> Id;

    /// The region bounded by the object.
    fn aabb(&self) -> AaBb;

    /// Generates all of the new instances within a region. 
    fn generate(bvh: &B, region: AaBb) -> impl Iter<Item = Self>;
}

pub struct Bvh<O: BvhObject<Self>> {
    // ...
}

impl <O: BvhObject<Self>> Bvh<O> {

    /// Inserts an object into the Bvh, 
    /// updating the existing object if it exists.
    fn insert(&mut self, object: O); 

    /// Updates the region of an existing item by [Id].
    /// Returns the previous region.
    fn update(&mut self, id: &Id, region: AaBb) -> Some(AaBb);

    /// Gets a stored BvhObject by Id
    fn get(&self, id: &Id) -> Some<O>;

    /// Gets some kind of itertable to ids reflecting all AaBb
    /// objects which intersect with the bounding region. 
    /// For performance considerations, this may be best as a more complex query type. 
    /// Often BVH will be an enum, and we will want to retrieve the 
    fn query_ids(&self, region: AaBb) -> impl Iter<Item = Id>;

    /// Generates new objects for the region
    fn generate(&self, region: AaBb) -> impl Iter<Item = O> {
        O::generate(self, region)
    }

}
```

However, in order to handle more complex queries, asynchronicity, object movement subtleties, and first-class Bevy support, we provide a more intricate design. 

## 2: Prior Art

## 3: Design

### 3.1: Spatial Index

Gimme BVH uses a simple hierarchical spatial index for storing BVH objects. The structure is an implicit, multi-resolution grid where each level defines a uniform subdivision of space, and objects are indexed by the cells they intersect at an appropriate scale.

#### 3.1.1: Insertion

A base scale $d_0 = (x, y, z) \in \mathbb{Z}^3$ is defined. Each subsequent level $d$ represents a uniform scaling such that cell size is:

```math
d_n = 2^n \cdot d_0
````

Thus, each increase in level doubles the cell size along all dimensions.

Upon insertion, an object is assigned a canonical level corresponding to the smallest cell size that can fully bound its AaBb. Let the object's extent be $(x, y, z)$; then:

```math
d_{\text{object}} = \left\lceil \log_2 \max(x, y, z) \right\rceil
```

> [!NOTE]
> Since AaBb extents can be represented as unsigned integers, by offsetting the coordinate space, this computation can be performed efficiently using bit operations.

```rust
fn ceil_log2_u32(n: u32) -> u32 {
    assert!(n > 0);
    if n <= 1 {
        0
    } else {
        u32::BITS - (n - 1).leading_zeros()
    }
}
```

At level $d_{\text{object}}$, the object is inserted into all grid cells it intersects. Given an AaBb with bounds $e = (e_{\min}, e_{\max})$, we define:

```math
c_d(x) = \left\lfloor \frac{x}{2^d \cdot d_0} \right\rfloor
```

Then all intersecting cells are given by:

```math
C_d(e) = \{ (i, j, k) \in \mathbb{Z}^3 \mid c_d(e_{\min}) \le (i, j, k) \le c_d(e_{\max}) \}
```

The object (or its identifier) is inserted into each such cell.

```rust
struct SpatialIndex<Id> { 
    cells: HashMap<(D, Cell), HashSet<Id>>, 
    values: HashMap<Id, AaBb>
}
```

#### 3.1.2: Querying

A query is defined by an AaBb region $e \in E$ and a set of levels $D' \subseteq D$.

For each level $d \in D'$, we compute the range of intersecting cells:

```math
c_{\min} = c_d(e_{\min}), \quad c_{\max} = c_d(e_{\max})
```

and enumerate all cells:

```math
C_d(e) = \{ (i, j, k) \in \mathbb{Z}^3 \mid c_{\min} \le (i, j, k) \le c_{\max} \}
```

All object identifiers stored in these cells are collected. Since objects may appear in multiple cells or levels, results must be deduplicated. The final result is:

```math
R = \bigcup_{d \in D'} \bigcup_{c \in C_d(e)} \text{table}[d][c]
```

Each candidate should then be filtered via exact AaBb intersection to remove false positives.

In pseudocode:

```rust
impl<Id> SpatialIndex<Id>
where
    Id: Copy + Eq + std::hash::Hash,
{
    pub fn iter_all(&self) -> impl Iterator<Item = (Id, AaBb)> + '_ {
        self.values.iter().map(|(&id, &aabb)| (id, aabb))
    }

    pub fn query_iter<'a>(
        &'a self,
        region: AaBb,
        levels: impl Iterator<Item = D>,
    ) -> impl Iterator<Item = (Id, AaBb)> + 'a {
        let mut seen = HashSet::new();
        let mut result = Vec::new();

        for d in levels {
            let cell_size = scale_to_cell_size(d);

            let min = region.min / cell_size;
            let max = region.max / cell_size;

            for x in min.x..=max.x {
                for y in min.y..=max.y {
                    for z in min.z..=max.z {
                        let key = (d, Cell { x, y, z });

                        if let Some(bucket) = self.cells.get(&key) {
                            for &id in bucket {
                                if seen.insert(id) {
                                    if let Some(&aabb) = self.values.get(&id) {
                                        if aabb.intersects(&region) {
                                            result.push((id, aabb));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        result.into_iter()
    }

    pub fn sub_index(
        &self,
        region: AaBb,
        levels: impl Iterator<Item = D>,
    ) -> SpatialIndex<Id> {
        let mut sub = SpatialIndex {
            cells: HashMap::new(),
            values: HashMap::new(),
        };

        for (id, aabb) in self.query_iter(region, levels) {
            sub.insert(id, aabb);
        }

        sub
    }
}
```


We can further optimize sub-index construction by using a "view", keeping references to matching `(D, Cell)` pairs:

```rust
struct SpatialIndexView<'a, Id> {
    cells: HashMap<(D, Cell), &'a HashSet<Id>>,
    values: &'a HashMap<Id, AaBb>,
}

pub struct SpatialIndexViewMut<'a, Id> {
    cells: HashMap<(D, Cell), &'a HashSet<Id>>,
    values: &'a HashMap<Id, AaBb>,

    // Local mutable overlay.
    overlay: SpatialIndex<Id>,
}

impl<Id> SpatialIndex<Id>
where
    Id: Copy + Eq + Hash,
{
    pub fn view<'a>(
        &'a self,
        region: AaBb,
        levels: impl Iterator<Item = D>,
    ) -> SpatialIndexView<'a, Id> {
        let mut cells = HashMap::new();

        for d in levels {
            let cell_size = scale_to_cell_size(d);

            let min = region.min / cell_size;
            let max = region.max / cell_size;

            for x in min.x..=max.x {
                for y in min.y..=max.y {
                    for z in min.z..=max.z {
                        let key = (d, Cell { x, y, z });

                        if let Some(bucket) = self.cells.get(&key) {
                            cells.insert(key, bucket);
                        }
                    }
                }
            }
        }

        SpatialIndexView {
            cells,
            values: &self.values,
        }
    }

    pub fn view_mut<'a>(
        &'a self,
        region: AaBb,
        levels: impl Iterator<Item = D>,
    ) -> SpatialIndexViewMut<'a, Id> {
        let view = self.view(region, levels);

        SpatialIndexViewMut {
            cells: view.cells,
            values: view.values,
            overlay: SpatialIndex {
                cells: HashMap::new(),
                values: HashMap::new(),
            },
        }
    }
}
```

This view tends to most useful after materialization on a single thread, e.g., spawning components. 

#### 3.1.3: Typing

The Gimme spatial index is designed to provide a broadphase, type-agnostic index on globally identified game entities. In particular, we want:

1. The ability to ergonomically identify intersections across different types. 
2. The ability narrow types for particular spatial queries. 

Identifiers can hold for arbitrary types and only need be materialized from backing storage when needed. 

Within a game world, spatial identifiers often need to be mapped to spawned entities. For example, a(n) `Id` stored in the spatial index may correspond to a Bevy `Entity`. A spatial query can then be reified into typed values by looking up the entity through one or more Bevy `Query` objects.

When entities may support multiple materialized views, the reification system can use optional query fields, `Or` filters, or multiple specialized queries.

```rust
fn perform_materialized_action(
    spatial_index: Res<SpatialIndex<Id>>,
    id_to_entity: Res<HashMap<Id, Entity>>,
    regions: Query<&AaBb, With<NeedsMyAction>>,
    objects: Query<(
        Entity,
        &Id,
        &MyType,
        Option<&MyOtherType>,
        Option<&MyOptionalType>,
    )>,
) {
    for region in &regions {
        let view = spatial_index.view(*region, relevant_levels());

        let mut materialized = MaterializedView::new();

        for (id, bounds) in view.query_iter(*region) {
            let Some(&entity) = id_to_entity.get(&id) else {
                continue;
            };

            let Ok((_, id, my_type, maybe_other, maybe_optional)) = objects.get(entity) else {
                continue;
            };

            let value = MaterializedType::from_parts(
                *id,
                bounds,
                my_type,
                maybe_other,
                maybe_optional,
            );

            materialized.insert(*id, value);
        }

        run_action_over_materialized_view(materialized);
    }
}
```

This pattern keeps the spatial index type-agnostic while allowing systems to construct typed working sets as needed. The spatial index answers only **which identifiers may be relevant**; Bevy queries determine **what those identifiers currently are**.

Such a type-agnostic approach will naturally cause over-fetching. For the most part, this should be reasonable. However, re-use of [3.2.1.2: Optimistic Drafts](#3212-optimistic-drafts) is advised.

> [!TIP]
> We expand upon typing patterns in [3.3.1: Materialization](#331-materialization)

### 3.2: Concurrency

For basic usage, developers may rely on existing synchronization primitives such as Bevy resources, queries, or standard library locks. However, Gimme’s spatial index is often accessed heavily for both reads and writes. To manage contention without over-constraining performance, we provide two distinct write modes:

- **Exclusive writes** for correctness-critical updates
- **Optimistic drafts** for parallel, mostly-independent updates

Additionally, we recommend structuring spatial data into multiple layers, e.g., static vs. stateful, to further reduce contention.


#### 3.2.1: Write APIs

We identify two common categories of writes:

1. **Mutual exclusion:**
   Updates that must observe and modify the current authoritative state without interference. These are typical for stateful gameplay logic and tightly coupled systems.

2. **Optimistic independence:**
   Updates that are mostly independent, or whose correctness can be resolved through caller-provided ordering (e.g., sequence numbers). These are common in procedural generation, streaming, and asynchronous workflows.

These modes are intentionally **mutually exclusive at the point of access**. A caller must explicitly choose between exclusive access and draft-based updates, and committing drafts which were held over an exclusive lock is invalid. 

```rust
pub struct ExclusiveVersion(u64);
pub struct DraftVersion(u64);

/// Global version of the spatial index.
pub enum Version {
    /// Set after an exclusive write. Invalidates older drafts.
    Exclusive(ExclusiveVersion),
    /// Set after draft application.
    Draft(DraftVersion),
}

pub enum SequenceNumber {
    /// Always applies.
    Agnostic,
    /// Applies only if newer than the stored value.
    Number(u64),
}

pub struct BimodalSpatialIndex<Entity> {
    spatial_index: SpatialIndex<Entity>,
    sequence_numbers: HashMap<Entity, SequenceNumber>,
    version: Version,
}

pub struct ExclusiveSpatialIndex<'a, Entity> {
    index: &'a mut BimodalSpatialIndex<Entity>,
    sub_index: SpatialIndex<Entity>,
}

pub struct DraftSpatialIndex<Entity> {
    sub_index: SpatialIndex<Entity>,
    draft_sequence_numbers: HashMap<Entity, SequenceNumber>,
    version: DraftVersion,
}

impl<Entity> BimodalSpatialIndex<Entity> {
    fn exclusive(
        &mut self,
        region: AaBb,
        levels: impl Iterator<Item = D>,
    ) -> ExclusiveSpatialIndex<'_, Entity>;

    fn draft(
        &mut self,
        region: AaBb,
        levels: impl Iterator<Item = D>,
    ) -> DraftSpatialIndex<Entity>;

    fn apply_draft(
        &mut self,
        draft: DraftSpatialIndex<Entity>,
    ) -> Result<(), ApplyError>;
}
```

##### 3.2.1.1: Exclusive Writes

Exclusive writes provide **full, immediate authority** over the spatial index.

When `exclusive` is called:

- All in-flight drafts are **logically invalidated**
- The spatial index enters an **exclusive version**
- The caller obtains a mutable view over a queried region

This ensures:

- No concurrent draft can apply stale updates
- The caller observes and mutates a **consistent, current state**
- Complex or stateful operations can be performed safely

Exclusive writes are appropriate for:

- Stateful gameplay updates (e.g., entity movement, inventory, quest state)
- Coherent transformations (e.g., replacing structures or regions)
- Systems that cannot tolerate stale reads

This model is intentionally **forceful**. It prioritizes correctness over fairness, under the assumption that exclusive writes are relatively rare or occur during controlled phases (e.g., loading, synchronization).

##### 3.2.1.2: Optimistic Drafts

Drafts provide a **parallel, non-blocking write model**.

A draft:

- Is created from a snapshot of the spatial index at a `DraftVersion`
- Contains a local `sub_index` representing a region
- Tracks edits via `draft_sequence_numbers`

When `apply_draft` is called:

- The draft is applied only if it is not invalidated by a newer `Exclusive` version
- Writes are merged using **sequence number semantics**

Sequence numbers enable **freshness control**:

- `Agnostic` writes always apply
- `Number(n)` writes apply only if `n` is greater than the stored value

This allows safe handling of:

- Asynchronous operations, e.g., network requests
- Parallel generation pipelines
- Out-of-order completion

Drafts are appropriate for:

- Procedural generation
- Streaming and LOD systems
- Cache construction
- Background tasks

Importantly, drafts do **not guarantee application**. They are optimistic:

- They may be invalidated by exclusive writes
- They may be superseded by newer sequence numbers

Failure handling is expected to be **externalized**, e.g., via ECS systems that detect stale or missing results when `draft_apply` returns a(n) `Err`. 

##### 3.2.1.3: Intermodal Fairness

The base API **favors exclusive writes**:

- Exclusive access immediately invalidates drafts
- And, drafts never block exclusive writes

This bias reflects the assumption that correctness-critical operations must not be delayed.

However, this can lead to contention if exclusive writes are frequent.

Several extensions can improve fairness:

**1. Layered Spatial Indices**

Split data into multiple indices, e.g., static vs. dynamic, as we elaborate upon in [3.2.2: Ground and State](#322-ground-and-state-indexes):

- Static/generative layers primarily use drafts
- Dynamic/stateful layers may use exclusive writes more often

Queries are composed via read-through:

1. Query stateful layer
2. Query static layer
3. Merge results, with stateful overriding static

This reduces cross-system interference.

**2. Sequence-Based Freshness**

Instead of enforcing ordering globally, drafts use sequence numbers to resolve conflicts locally.

This avoids:

- Global queues
- Blocking on slow operations
- Stale overwrites from asynchronous work

**3. Optional Admission Control (Future Work)**

A softer model could allow:

- Temporarily preventing new drafts during exclusive phases
- Allowing existing drafts to complete

This improves fairness but increases complexity and is not part of the base design.

#### 3.2.2: Ground and State Indexes

In practice, contention can often be reduced further by separating the spatial index into two logical layers: **Ground** and **State**.

- **Ground:** stores the base spatial data for the world. This is typically the output of generation, loading, streaming, or other environment-construction processes. Ground is often relatively stable, but it is **not required to be immutable**. Rather, Ground is the layer whose contents are not primarily driven by live agent actions.
- **State:** stores the spatial data most directly affected by characters, game agents, simulation, or other decision systems. This includes dynamic repositioning, transient objects, and modifications to previously generated artifacts.

This distinction is semantic rather than absolute. An artifact may first be constructed by an operation over the Ground index and later be moved, replaced, or otherwise updated through the State index. In this sense, State acts as the more immediate and authoritative layer for live gameplay.

The two layers are each implemented as a `BimodalSpatialIndex<Entity>`:

```rust
pub struct HierarchicalSpatialIndex<Entity> {
    ground: BimodalSpatialIndex<Entity>,
    state: BimodalSpatialIndex<Entity>,
}
```

The intended use is:

- **Ground** favors optimistic drafts, generation, and background construction
- **State** favors exclusive writes or other correctness-sensitive updates
- Queries are composed through a read-through process

When querying, the State layer is consulted first, then the Ground layer. If both layers contain the same logical entity, the State layer is treated as authoritative for the purposes of exact value lookup.

This avoids forcing all systems into the same contention regime. Generation systems can mostly interact with Ground, while live gameplay systems interact with State.

##### 3.2.2.1: Read-through Queries

A read-through query composes the two layers into a single result set. At a high level:

1. Query the State index and collect candidate entities
2. Query the Ground index and collect candidate entities
3. Deduplicate the combined result
4. When resolving exact values, prefer State to Ground.

In pseudocode:

```rust
impl<Entity> HierarchicalSpatialIndex<Entity> {
    fn query(
        &self,
        region: AaBb,
        levels: impl Iterator<Item = D> + Clone,
    ) -> Vec<Entity> {
        let mut result = Vec::new();
        let mut seen = HashSet::new();

        self.state.spatial_index.query(region, levels.clone(), |entity| {
            if seen.insert(entity) {
                result.push(entity);
            }
        });

        self.ground.spatial_index.query(region, levels, |entity| {
            if seen.insert(entity) {
                result.push(entity);
            }
        });

        result
    }

    fn get_aabb(&self, entity: &Entity) -> Option<AaBb> {
        self.state
            .spatial_index
            .values
            .get(entity)
            .copied()
            .or_else(|| self.ground.spatial_index.values.get(entity).copied())
    }
}
```

This composition is side effect free. Querying Ground does not imply promotion into State.

##### 3.2.2.2: Drafting by Layer

Because the two layers are separate bimodal indices, either write mode may be used at either layer:

- `ground.draft(...)`
- `ground.exclusive(...)`
- `state.draft(...)`
- `state.exclusive(...)`

However, the intended pattern is that:

- Ground is primarily updated through drafts and asynchronous generation.
- State is primarily updated through a greater use of controlled exclusive updates. 

This is a usage guideline, not a type-level restriction.

In pseudocode:

```rust
impl<Entity> HierarchicalSpatialIndex<Entity> {
    fn ground_draft(
        &mut self,
        region: AaBb,
        levels: impl Iterator<Item = D>,
    ) -> DraftSpatialIndex<Entity> {
        self.ground.draft(region, levels)
    }

    fn state_exclusive(
        &mut self,
        region: AaBb,
        levels: impl Iterator<Item = D>,
    ) -> ExclusiveSpatialIndex<'_, Entity> {
        self.state.exclusive(region, levels)
    }
}
```

##### 3.2.2.3: Promotion from Ground to State

It is common for an artifact to be first created in Ground and later be modified through State. For example:

- a generated structure is created in Ground
- a character moves, damages, or repurposes it
- the updated representation is written into State

This does not require removing the original value from Ground. Instead, State may simply shadow Ground for the affected entity.

In pseudocode:

```rust
impl<Entity> HierarchicalSpatialIndex<Entity> {
    fn promote_to_state(
        &mut self,
        entity: Entity,
        new_aabb: AaBb,
    ) {
        let mut state = self.state.exclusive(new_aabb, std::iter::empty());
        state.insert(entity, new_aabb);
    }
}
```

More generally, the same logical entity may exist in both layers, but State is treated as the authoritative layer for live queries and exact-value resolution.

##### 3.2.2.4: Contention Reduction

This layered model reduces logical contention by separating systems with different mutation profiles:

- **Ground** absorbs long-lived generation and streaming work
- **State** absorbs immediate gameplay and simulation updates

Without this split, exclusive writes from dynamic systems may repeatedly invalidate long-running generative drafts. With the split, most such interference disappears, since the systems are no longer writing against the same underlying index.

This model is therefore especially suitable when:

- generated world content is relatively independent of moment-to-moment simulation
- dynamic gameplay objects require stricter update semantics
- asynchronous generation should not be repeatedly invalidated by live entity movement

### 3.3: Materialization and Persistence

We will often want to store both the spatial index and the objects generated or otherwise indexed. This may be to the file system, a database, or even a multiplayer network. In this section, we cover recommendations for handling both. 

#### 3.3.1: Materialization

As mentioned in [3.1.3: Typing](#313-typing), the spatial index is intentionally type-agnostic. It stores identifiers and AaBb bounds, not concrete domain objects. Systems that need typed values should therefore **materialize** a spatial result into one or more typed working sets.

There are two common patterns:

1. **Type Segregation:** build one materialized spatial index per required type by filtering over the agnostic spatial draft. Construction is roughly $O(\text{types} \cdot \text{objects})$, but each typed index is simple and direct to use.
2. **Type Unification:** build one materialized index over an enum or tagged value containing all relevant types. Construction is roughly $O(\text{objects})$, but typed access requires matching or filtering.

> [!NOTE]  
> We generally recommend **type segregation**. It is simpler, more flexible, and composes cleanly with Bevy queries and draft-based generation.

A materialized spatial index can be represented by a small trait:

```rust
pub trait MaterializedSpatialIndex<T> {
    type Id;

    fn get(&self, region: AaBb) -> impl Iterator<Item = (Self::Id, &T)>;

    fn insert(
        &mut self,
        id: Self::Id,
        value: T,
        region: AaBb,
    );
}
```

One possible implementation is a typed wrapper over a draft:

```rust
pub struct TypedDraft<Id, T> {
    spatial: DraftSpatialIndex<Id>,
    values: HashMap<Id, T>,
}

impl<Id, T> MaterializedSpatialIndex<T> for TypedDraft<Id, T>
where
    Id: Copy + Eq + std::hash::Hash,
{
    type Id = Id;

    fn get(&self, region: AaBb) -> impl Iterator<Item = (Id, &T)> {
        self.spatial
            .sub_index
            .query_iter(region, all_relevant_levels())
            .filter_map(|(id, _bounds)| {
                self.values.get(&id).map(|value| (id, value))
            })
    }

    fn insert(
        &mut self,
        id: Id,
        value: T,
        region: AaBb,
    ) {
        self.spatial.sub_index.insert(id, region);
        self.spatial
            .draft_sequence_numbers
            .insert(id, SequenceNumber::Agnostic);
        self.values.insert(id, value);
    }
}
```

For type segregation, a materialization pass builds multiple typed drafts from the same underlying spatial draft:

```rust
pub struct SegregatedDraft<Id> {
    source: DraftSpatialIndex<Id>,

    terrain: TypedDraft<Id, TerrainTile>,
    structures: TypedDraft<Id, Structure>,
    actors: TypedDraft<Id, ActorProxy>,
}
```

In Bevy, this can be produced by querying the type tables separately:

```rust
fn build_segregated_draft(
    mut spatial: ResMut<BimodalSpatialIndex<Id>>,
    id_to_entity: Res<HashMap<Id, Entity>>,
    terrain_query: Query<(&Id, &TerrainTile)>,
    structure_query: Query<(&Id, &Structure)>,
    actor_query: Query<(&Id, &ActorProxy)>,
    requests: Query<&AaBb, With<NeedsGenerationDraft>>,
) {
    for region in &requests {
        let source = spatial.draft(*region, relevant_levels());

        let mut draft = SegregatedDraft {
            terrain: TypedDraft {
                spatial: source.clone_empty(),
                values: HashMap::new(),
            },
            structures: TypedDraft {
                spatial: source.clone_empty(),
                values: HashMap::new(),
            },
            actors: TypedDraft {
                spatial: source.clone_empty(),
                values: HashMap::new(),
            },
            source,
        };

        for (id, bounds) in draft.source.sub_index.query_iter(*region, relevant_levels()) {
            let Some(&entity) = id_to_entity.get(&id) else {
                continue;
            };

            if let Ok((_, terrain)) = terrain_query.get(entity) {
                draft.terrain.insert(id, terrain.clone(), bounds);
            }

            if let Ok((_, structure)) = structure_query.get(entity) {
                draft.structures.insert(id, structure.clone(), bounds);
            }

            if let Ok((_, actor)) = actor_query.get(entity) {
                draft.actors.insert(id, actor.clone(), bounds);
            }
        }

        run_generation_over_segregated_draft(draft);
    }
}
```

This pattern keeps the base spatial index type-agnostic while allowing generation systems to work against strongly typed spatial views. Each typed draft can be queried and mutated independently, then compacted back into the underlying draft or applied through a type-specific synchronization system.


#### 3.3.2: Persistence

1. Each live `Entity` that we wish to persist must map to a(n) `Id`. 
2. Each `Id` that we wish to persist may map to multiple types. (This induces a mapping from `Entity` to types thought such was already implied in [Materialization](#331-materialization).)
3. When persisting, we use the mapping from `Entity` to `Id` available in the game world to update and store an out-of-memory spatial index over `Id`.
4. We load portions of the out-of-memory spatial index by queries over AaBb regions. 
5. We then materialize by querying for stored types and inserting into the game world. 
6. The entities given by the game world are then inserted into the in-memory spatial index. 

```mermaid
```

### 3.4: Generation

#### 3.4.1: Hierarchical Generation

The simplest and most flexible way to perform hierarchical generation is to generate from the bottom up--checking requirements and using the spatial index for fetching results. 

The simplest pattern within this context is often to perform cellular generation. That is, to identify disjoint cells and generate values within them. We show this pattern below.


```rust
/// The base generator trait simply asks for a the implementer to provide a method for get or generating types within a requested region.
pub trait Generator<T>: SpatialIndex<T> {

    /// Generates and inserts type instances intersecting with the region.  
    fn get_or_generate(
        &mut self,
        requested_region: AaBb
    ) -> Result<impl Iter<Item = T>, GenerationError>;
}

/// A cellular generator narrows this by enforcing that each cell has only one value for the type.
pub trait CellGenerator<T>: SpatialIndex<T> {

    /// Gets all of the cells that would intersecting with the region for the type. 
    fn intersecting_cells(&self, region: AaBb) -> impl Iter<Item = Cell>;

    /// Generates one instance on a cell. 
    fn generate_cell(&mut self, cell: Cell) -> Result<T, GenerationError>; 

    /// Gets or generates one instance on a cell
    fn get_or_generate_cell(
        &mut self,
        cell: Cell
    ) -> Result<T, GenerationError> {

        fn get_or_generate_cell(
    &mut self,
    cell: Cell,
) -> Result<T, GenerationError> {
        if let Some(value) = self.read_one(cell.as_region())? {
            return Ok(value);
        }

        let value = self.generate_cell(cell)?;
        self.insert(value.clone())?;
        Ok(value)
    }
}

/// If we have a cell generator, we automatically have a generator. 
impl<Index: CellGenerator<T>> Generator<T> for Index {

    for cell in self.intersecting_cells(requested_region) {
        self.get_or_generate_cell(cell)?;
    }

}

/// Now an example hierarchy
pub struct Top;
pub struct Middle;
pub struct Bottom;

impl<Index> CellGenerator<Bottom> for Index
    // We specifically enforce tha Top and Middle are cell generators as opposed to general [Generator] types. 
    // This allows us to ensure one-to-one cell mappings. 
    where Index: CellGenerator<Top> + CellGenerator<Middle> 
{

    fn intersecting_cells(&self, region: AaBb) -> impl Iter<Item = Cell> {
        // ...
    }

    fn generate_cell(
        &mut self, 
        cell: Cell
    ) -> Result<Bottom, GenerationError> {

        // One Bottom cell should be within one Top cell.
        let top: Top = self.get_or_generate_cell(cell)?;
        // One Bottom cell should be within one Middle cell.
        let middle: Middle = self.get_or_generate_cell(cell)?;

        // Custom constructor
        Bottom::from_cell_and_parents(cell, top, middle)

    }

}

/// The middle layer might only consider top. 
impl<Index> Generator<Middle> for Index
    where Index: CellGenerator<Top> 
{

    
    fn generate_cell(
        &mut self, 
        cell: Cell
    ) -> Result<Bottom, GenerationError> {

        // One Middle cell should be within one Top cell.
        let top: Top = self.get_or_generate_cell(cell)?;

        // Custom constructor
        Middle::from_cell_and_parents(cell, top)

    }

}
```

> [!WARNING]
> Circumventing the hierarchy generator traits to insert higher order types can produce unequal paths through the hierarchy. You should fetch requirements. 

> [!WARNING]
> Generally, you should only fetch up the hierarchy. Trying to fetch siblings can create circular dependencies. 

This bottom-up approach allows systems to discover minimal generation paths, preventing overfetching. Conversely, the approach also allows systems to generate whatever they specifically need, while respecting a requirement hierarchy. 

### 3.5: In Bevy

## 4: Milestones