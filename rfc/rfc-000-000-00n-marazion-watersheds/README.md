# RFC-n: Marazion Watersheds

## 1: Motivation

## 2: Prior Art

## 3: Design

The watershed designs proposed in this RFC are referred to as Marazion watersheds. All following the stamping framework proposed in [RFC-105](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain). 

### 3.1: Marazion Pocket Water Stamping

Marazion Pocket Waters are used to satisfy the [Jersey Pocket Waters requirement of RFC-105](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain#384-jersey-pocket-waters-small-hydrology-chains). 

Marazion pocket waters rely on three levels of cellular stamping hierarchy:

1. **[Pre-pocket Cells](#311-pre-pocket-cells):** the base parent cells representing the extents within which **Pocket Cells** are generated. For simplicity, they are a grid of fixed-size AABB cells and each fixes one cell size for all **Pocket Cells** contained within it, creating an internal grid. The role of **Pre-pocket Cell:** is to vary the extents of **Pocket Cells** over the game world, while keeping regional correlations. The noise value for the Pocket Cell size is given by the lower-left coordinate of Pre-pocket Cell, floored to a reasonable multiple.
2. **[Pocket Cells](#312-pocket-cells):** the cells within which certain simple hydrology types are selected. A **Pocket Cell** use a pseudo-random Guillotine cuts, with bounded depth. The noise value for the Guillotine cuts is given by the lower-left coordinate of the Pocket Cell. 
3. **[Pocket Water Cells](#313-pocket-water-cells):** the cells within which independent pocket water types are generated. 

#### 3.1.1: Pre-pocket Cells

#### 3.1.2: Pocket Cells

#### 3.1.3: Pocket Water Cells

The following are the Pocket Water Cells which should be included in Marazion.

To ensure reliable rims, all construction rely on creating a plateau, then depressing everything within the plateau to ensure a rim around the body of water. 

##### 3.1.3.1: Lake

The **default** lake footprint is an **offset centroid** and a **noisy circular radius** inside the pocket water cell (steps 1–7). Elevation follows the usual plateau-then-depress rim recipe from the introduction to §3.1.3.

1. Sample noise to offset the lake **centroid** $(x_c, z_c)$ from the cell centroid. 
2. Compute **lake surface** elevation at $(x_c, z_c)$: add a signed noise value to the terrain height there. Derive a **depth** scale from noise (same anchor as the rest of the cell).
3. Compute **pre-radius:** distance from $(x_c, z_c)$ to the nearest point on the cell boundary, minus a margin $\mu$.
4. At each sample, **radius** $=$ pre-radius minus a noisy term (keyed, so the disc stays inside the cell). Points with horizontal distance to $(x_c, z_c)$ below that radius count as **inside the water disc** for the steps below.
5. Raise all points **inside** the radius to the lake surface elevation **plus** a noise value.
6. Raise all points inside **radius** $+\,\alpha \cdot \text{noise} \cdot \mu$ (with $\alpha,\, \text{noise} \in [0,1]$) to the surface elevation **plus or minus** noise (rim / transition band).
7. Depress all points inside **radius** $-\,\alpha \cdot \text{noise} \cdot \mu$ by subtracting $\text{dist to }(x_c,z_c) \cdot \text{noise} \cdot \text{depth}$ from the current elevation (bowl).

> [!NOTE]
> **Suggested alternative — inscribed footprint:** if the cell is long or thin, a circle around a centroid wastes extent. Use axis-aligned bounds $[x_0,x_1]\times[z_0,z_1]$, inward clearance $d_\cap(x,z)=\min(x-x_0,\,x_1-x,\,z-z_0,\,z_1-z)$, margin $m=\mu+\text{noise}(\text{anchor})$. **Water:** $d_\cap\ge m$; **shore:** $d_\cap=m$. Replace **dist to centroid** and **radius** bands in steps 5–7 with **depth from shore** monotone in $g=d_\cap-m$ (or the per-edge rectangle variant from the same section).

Rust pseudocode (default circular footprint; `g` hook matches the NOTE alternative):

```rust
// Pocket water cell AABB in xz; sample at (x, z). `base_h` is pre-stamp height.
// `n2`, `n01` = deterministic noise fns in [0,1] keyed by (anchor, salt).

let (xc, zc) = cell_centroid + noise_offset_xz(anchor);
let surface = base_h_at(xc, zc) + surface_noise(anchor);
let depth = depth_from_noise(anchor);
let pre_r = dist_to_rect_boundary(xc, zc, cell) - mu;
let r = pre_r - radius_jitter(x, z, anchor); // keep r positive inside cell

let d = hypot(x - xc, z - zc);
let inner = d < r;
let outer_band = d < r + alpha * n01(x, z, anchor) * mu;
let bowl = d < r - alpha * n01(x, z, anchor) * mu;

let mut h = base_h;
if inner {
    h = h.max(surface + n2(x, z, anchor) * rim_lift);
}
if outer_band {
    h = h.max(surface + rim_noise(x, z, anchor)); // ± noise per step 6
}
if bowl {
    h -= d * n2(x, z, anchor) * depth;
}

// Inscribed alternative: g = d_cap(x,z, cell) - m; use g instead of (r - d) for bands:
// let g = inward_clearance(x, z, cell) - m;
// same structure: inner = g > 0, outer_band = g > -..., bowl = g > +...
```

##### 3.1.3.2: Stream

##### 3.1.3.3: Bog

##### 3.1.3.4: Lake into Stream

##### 3.1.3.5: Stream into Lake

### 3.2: Marazion Basin Water Stamping

### 3.3: Marazion Hydrology Complex Stamping

### 3.4: Marazion Global Ocean

## 4: Milestones