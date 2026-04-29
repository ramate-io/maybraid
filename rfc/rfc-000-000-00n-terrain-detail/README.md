# RFC-N: Terrain Detail

## Table of Contents

## 1: Summary

In response to [#57](https://github.com/ramate-io/maybraid/issues/57), we propose a several simple cellular terrain detail generation systems under the name Durham Terrain Detail. 

## 2: Prior Art

## 3: Design

### 3.1: Sparse Boulders

Sparse boulders are generated as low-density, spatially stable features using a two-tier grid and deterministic noise. Ownership and LOD are governed strictly by the first-order grid.

**High-level stages**

1. Partition space into first-order cells for ownership and LOD.
2. Subdivide each cell into second-order sampling regions.
3. Activate candidate regions via deterministic noise.
4. Select and validate positions against bounds and terrain steepness.
5. Generate boulder geometry via SDF parameterization.
6. Place and embed boulders into terrain.

---

#### 3.1.1: First-order grid (LOD ownership)

Let $\mathcal{G}_1$ be a grid over $\mathbb{R}^3$ with cell size $L$. Each cell $C \in \mathcal{G}_1$ defines a **boulder ownership region**.

All generated boulders must satisfy:

$$
\mathbf{x}_{\text{boulder}} \in C
$$

...and are spawned and managed exclusively by the chunk corresponding to $C$.

---

#### 3.1.2: Second-order grid (sampling)

Within each $C \in \mathcal{G}_1$, define a finer grid $\mathcal{G}_2(C)$ with cell size $l \approx$ minimum boulder separation.

Each $c \in \mathcal{G}_2(C)$ is a candidate sampling region.

Parameters:

* $s_{\min}, s_{\max}$: allowable terrain steepness

Activation:

```rust
let seed = hash(c);
if !noise_bool(seed) {
    continue;
}
```

---

#### 3.1.3: Position selection and validation

```rust
let o = noise_vec3(seed); // ~ [-1, 1]^3
let p = origin(c) + o * l;
```

Allow $p$ to exceed bounds of $c$.

Validation:

```rust
if !contains(C, p) {
    return None;
}

let k = laplacian(terrain, p);

if k < s_min || k > s_max {
    return None;
}
```

> [!WARNING]
> Second-order cells are **sampling-only** and must not define LOD.
> Offsets may escape their cell.
>
> **All ownership, culling, and stability must derive from the first-order grid.**
>
> A common fallback is to align second-order bounds with their parent first-order cell.

---

#### 3.1.4: Shape and scale (SDF)

```rust
let params = noise_params(seed);
let sdf = boulder_sdf(params); // unit
let scale = noise_scale(seed);

spawn_mesh(mesh_from_sdf(sdf), scale);
```

Mesh generation uses a unit SDF. Scaling is applied at spawn. Physics should use the scaled SDF.

---

#### 3.1.5: Placement

```rust
let z_offset = -embed_depth(seed);
let position = Vec3::new(p.x, terrain_height(p), p.z)
    + Vec3::Y * z_offset;
```

Embed slightly into terrain for grounding.

---

**Properties**

* deterministic placement
* LOD-safe ownership
* controlled separation
* independence from chunk boundaries beyond $\mathcal{G}_1$

### 3.2: Crag Complexes

Crag complexes are clustered rock features generated along deterministic paths or small graphs inside a first-order ownership cell. Compared to sparse boulders, crag complexes intentionally create local structure: ridgelines, broken shelves, talus chains, and clustered outcrops.

The construction borrows the hysteresis path idea from [Marazion Stream](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#3132-stream) construction: a polyline is built by blending the previous heading with a goal heading, then adding bounded angular jitter. This creates coherent paths without white-noise zigzags.

**High-level stages**

1. Decide whether the first-order cell contains a crag complex.
2. Select one or more endpoints or anchors inside the cell.
3. Build a bounded hysteresis polyline or small graph.
4. Treat bounded paths as valid LOD subunits if desired.
5. Place boulder candidates along the path.
6. Validate candidates against terrain steepness.
7. Generate SDF boulders and compose them into a clustered crag.

---

#### 3.2.1: Cell activation

Let $C \in \mathcal{G}_1$ be a first-order ownership cell. Sample deterministic control noise at the cell anchor:

```rust
let seed = hash(C);
if noise(seed, CRAG_COMPLEX_SALT) > p_crag {
    return None;
}
```

Optional terrain filters may reject cells whose terrain is too flat or too steep overall:

```rust
let k = mean_laplacian(terrain, C);
if k < crag_min_steepness || k > crag_max_steepness {
    return None;
}
```

---

#### 3.2.2: Path or graph construction

Choose endpoints from noise, biased toward plausible rocky terrain:

```rust
let start = sample_point_in_cell(seed, START_SALT);
let end = sample_point_in_cell(seed, END_SALT);
```

Build a crag spine with a hysteresis walk:

```rust
fn build_crag_path(cell: AaBb, seed: Seed, start: Vec2, end: Vec2) -> Vec<Vec2> {
    let mut p = start;
    let mut dir_prev = (end - start).normalize_or_zero();
    let mut path = vec![start];

    for k in 0..MAX_SEGMENTS {
        let to_end = end - p;

        if to_end.length() <= SNAP_RADIUS {
            path.push(end);
            break;
        }

        let dir_goal = to_end.normalize_or_zero();
        let theta = angle_jitter(seed, k) * MAX_TURN_RADIANS;
        let blended = normalize(lerp(dir_goal, dir_prev, HYSTERESIS));
        let dir = rotate(blended, theta);

        let q = p + dir * SEGMENT_LENGTH;

        if !contains_xz(cell, q) {
            break;
        }

        path.push(q);
        p = q;
        dir_prev = dir;
    }

    path
}
```

For larger complexes, add one or two bounded branches from intermediate points on the main spine:

```rust
let branches = noise_count(seed, BRANCH_SALT, 0..=MAX_BRANCHES);
```

---

#### 3.2.3: LOD units

Unlike sparse boulder second-order cells, crag paths can serve as LOD units because they are explicitly bounded by the first-order cell.

A crag unit may be represented as:

```rust
struct CragUnit {
    bounds: AaBb,
    path: Vec<Vec2>,
}
```

The unit bounds should be derived from the path plus a maximum crag width:

```rust
let bounds = path_bounds(path).inflate(CRAG_WIDTH).clamp(C);
```

This allows the system to cull, stream, or refine individual crag paths while preserving first-order cell ownership.

---

#### 3.2.4: Candidate placement along path

Sample candidate boulders along the polyline at approximately separation distance $l$:

```rust
for s in arclength_samples(path, l) {
    let center = point_at_arclength(path, s);
    let normal = path_normal(path, s);

    let lateral = noise(seed, s) * CRAG_WIDTH;
    let p = center + normal * lateral;

    if !contains_xz(C, p) {
        continue;
    }

    candidates.push(p);
}
```

The lateral offset lets the crag occupy a band around the spine rather than a single line.

---

#### 3.2.5: Terrain validation

For each candidate point $p$, evaluate local steepness or curvature:

```rust
let k = laplacian(terrain, p);

if k < boulder_min_steepness || k > boulder_max_steepness {
    continue;
}
```

Additional checks may reject candidates too close to water, roads, settlement footprints, or already accepted boulders.

---

#### 3.2.6: Shape and composition

Each accepted candidate receives a deterministic SDF variant:

```rust
let params = crag_boulder_params(seed, p);
let sdf = boulder_sdf(params); // unit
let scale = crag_scale(seed, p);
let rotation = align_to_slope(terrain_normal(terrain, p));
```

Crag complexes should bias shapes toward angular, elongated, or fractured forms. Neighboring boulders may share a complex-level style seed, so the whole formation reads as one geological feature.

---

#### 3.2.7: Placement

```rust
let h = terrain_height(p);
let embed = crag_embed_depth(seed, p);

let position = Vec3::new(p.x, h, p.y) - terrain_normal(terrain, p) * embed;
```

Embedding should be stronger than sparse boulders so clustered rocks appear grounded and partially fused into the terrain.


### 3.3: World Unit Varying Shader for Ground Color

### 3.4: Bump Outs

"Bump Outs" refer to structures placed above the terrain which follow its contours. In Durham terrain detail, we build bump-outs simply by cloning the underlying terrain SDF and adding to its Z extents noisily within some boundary determined via a noisy radius. We provide the general cell and boundary generation description in [3.4.1](#341-cell-and-boundary-generation) and specify particular bump outs in the sections which follow.

#### 3.4.1: Cell and Boundary Generation

#### 3.4.2: Snow Bump Out

1. Parameterize whether cell is snowy by underlying elevation and fractal noise sampling for local consistency.
2. Standard bump out. 
3. Use snow shader. 
4. Don't worry about seasonality yet. 

#### 3.4.3: Sand and Dunes Bump Out

1. Parameterize by whether cell has sand dunes by steepness sampled at a few points and fractal noise sampling for local consistency. 
2. Use inner grid to generate points at which elliptical dunes will exist. 
3. Apply standard bump out noise plus dune "dome" noise around selected elliptical points. 
4. Use sand shader. 

## 4: Milestones