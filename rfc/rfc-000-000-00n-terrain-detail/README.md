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

Apply parameters to decide whether cell should have a crag complex. Use a hysteresis pathfinding method similar to the [Stream](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#3132-stream) construction in Marazion watersheds to build a polyline or graph along which boulders will be placed within a cell.

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