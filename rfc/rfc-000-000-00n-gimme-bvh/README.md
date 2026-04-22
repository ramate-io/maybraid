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
struct SpatialIndex<Entity> { 
    cells: HashMap<(D, Cell), HashSet<Entity>>, 
    values: HashMap<Entity, AaBb>
}
```

> [!NOTE]
> We use `Entity` above in reference to Bevy's entities. As described in [3.1.3](#313-typing), the primary use of the Gimme spatial index is over entities in the Bevy ECS. 

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
impl SpatialIndex<Entity> {
    fn query(
        &self,
        region: AaBb,
        levels: impl Iterator<Item = D>,
    ) -> HashSet<Entity> {
        let mut result = HashSet::new();

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
                                if !result.contains(id) {
                                    if let Some(aabb) = self.values.get(&id) {
                                        if aabb.intersects(&region) {
                                            result.add(id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        result
    }
}
```

#### 3.1.3: Typing

The Gimme spatial index is designed to provide a broadphase, type-agnostic index on game entities. In particular, we want:

1. The ability to ergonomically identify intersections across different types. 
2. The ability narrow types for particular spatial queries. 

This implies using an identifier such as Bevy's `Entity` as the spatially indexed object. We can then reify queries into types against a Bevy `Query` object, as is elaborated up in [3.2: Concurrency](#32-concurrency) and [3.3](#33-generation). Where entities may be multiple types, we can use `Option` query fields, `Or` queries, or simply multiple `Query` objects.  

```rust
```

Such a type-agnostic approach will naturally cause over-fetching. For the most part, this should be reasonable. However, re-use of [3.2.1.2: Optimistic Drafts](#3211-optimistic-drafts) is advised.

### 3.2: Concurrency

For basic usages the user may be able to rely on synchronization primitives, such as Bevy's resource and query APIs or standard library locks. However, Gimme's spatial index will often be used heavily for both reads and writes. To account for this contention, we propose two distinct write APIs: exclusive writes and optimistic drafts. Additionally, we suggest a two-layer spatial indexing model.

#### 3.2.1: Write APIs

We identify two common cases for writing to a spatial index:

1. **Mutual exclusion:** when writing, all other readers and writers must be excluded for the update to be correct. This is most common in stateful or non-streamable systems. 
2. **Optimistic independence** when writing, most updates do not touch the same entities. And, if they do, they either write the same value or can be sequenced for correctness by information passed down from the caller. 

Importantly, a spatial index cannot operate in both modes at once. These modes must be mutually exclusive. 

To accomplish this, we compose the following type from the spatial index:

```rust
pub struct SequenceNumber {
    /// Writes regardless of stored sequence number.
    Agnostic,
    /// Writes only if value is greater than stored sequence number.
    /// This helps avoid stale writes for asynchronous processes. 
    Number(u64)
};

pub struct BimodalSpatialIndex<Entity> {
    /// The core underlying spatial index.
    spatial_index: SpatialIndex<Entity>,
    /// The sequence numbers for entities in the spatial index.
    sequence_numbers: HashMap<Entity, SequenceNumber>,
    /// The queue of oneshot receivers of drafts. 
    draft_queue: VecDeque<OneshotReceiver<DraftSpatialIndex>>
}

pub struct ExclusiveSpatialIndex<Entity> {
    /// The underlying spatial index. 
    index: &mut BimodalSpatialIndex<Entity>,
    /// The underlying spatial index queried to a given region. 
    /// Mutation methods are not publicly available on the spatial index. 
    sub_index: SpatialIndex<Entity>,
}

pub struct DraftSpatialIndex<Entity> {
    /// The underlying spacial index queried to a region (this is a copy).
    sub_index: SpatialIndex<Entity>,
    /// When we update a spatial inde via a draft,
    /// sequence numbers are respected. 
    /// Effectively, compaction occurs respecting "latest sequence number writes" semantics. 
    draft_sequence_numbers: HashMap<Entity, SequenceNumber>,
    // fix: this needs to be non-circular
    self_sender: OneshotSender<Self>
}

impl BimmodalSpatialIndex<Entity> {
    
    /// Constructs the exclusive spatial index 
    /// Clears all active drafts. 
    /// 
    /// Note: we could explore some fairness designs. 
    /// We could make the force optional and place something like a
    /// "no new drafts" lock on this. But, maybe that overcomplicates. 
    /// Often where we have contention is going to be a loading screen anyways and the data we need to materialize will often be stored. 
    fn exclusive(
        &mut self, 
        region: AaBb, 
        levels: impl Iter<Item = D>
    ) -> ExclusiveSpatialIndex<'_, Entity>;

    /// Constructs the draft spatial index
    fn draft(
        &mut self,
        region: AaBb, 
        level: impl Iter<Item = D>
    ) -> DraftSpatialIndex<Entity>;

}
```

##### 3.2.1.1: Exclusive Writes

##### 3.2.1.1: Optimistic Drafts

#### 3.2.2: Ground and State Indexes


### 3.3: Generation

### 3.4: In Bevy

## 4: Milestones