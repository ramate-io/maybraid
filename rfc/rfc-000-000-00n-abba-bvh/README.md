# RFC-N: ABBA BVH

## Table of Contents

## 1: Motivation

ABBA BVH is a simple AaBb-based BVH system proposed for spatial storage, querying, and retrieval system for Maybraid. It seeks to address the needs to efficiently determine spatial entities available within an AaBb region, as well as to handle generation. The API is roughly intended to handle the following operations:

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

### 3.1: Spatial Storage

ABBA BVH uses a simple hierarchical spatial index for storing BVH objects. The structure is an implicit, multi-resolution grid where each level defines a uniform subdivision of space, and objects are indexed by the cells they intersect at an appropriate scale.

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
struct SpatialIndex<ObjectId> { 
    cells: HashMap<(D, Cell), HashSet<ObjectId>>, 
    values: HashMap<ObjectId, AaBb>
}
```

---

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
impl SpatialIndex<ObjectId> {
    fn query(
        &self,
        region: AaBb,
        levels: impl Iterator<Item = D>,
    ) -> Vec<ObjectId> {
        let mut result = Vec::new();
        let mut seen = HashSet::new();

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
                                    // Optional exact check (recommended)
                                    if let Some(aabb) = self.values.get(&id) {
                                        if aabb.intersects(&region) {
                                            result.push(id);
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


## 4: Milestones