# RFC-n: Marazion Watersheds

## 1: Motivation

## 2: Prior Art

## 3: Design

The watershed designs proposed in this RFC are referred to as Marazion watersheds. All following the stamping framework proposed in [RFC-105](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain). 

### 3.1: Marazion Pocket Water Stamping

Marazion Pocket Waters are used to satisfy the [Jersey Pocket Waters requirement of RFC-105](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain#384-jersey-pocket-waters-small-hydrology-chains). 

Marazion pocket waters rely on three levels of cellular stamping hierarchy:

1. **[Pre-pocket Cells](#311-pre-pocket-cells):** the base parent cells representing the extents within which **Pocket Cells** are generated. For simplicity, they are a grid of fixed-size AABB cells and each fixes one cell size for all **Pocket Cells** contained within it, creating an internal grid. The role of **Pre-pocket Cell:** is to vary the extents of **Pocket Cells** over the game world, while keeping regional correlations. The noise value for the Pocket Cell size is given by the lower-left coordinate of Pre-pocket Cell, floored to a reasonable multiple.
2. **[Pocket Cells](#312-pocket-cells):** the cells within which certain simple hydrology types are selected. A **Pocket Cell** uses pseudo-random **Guillotine cuts** with bounded depth. The noise value for the Guillotine cuts is given by the lower-left coordinate of the Pocket Cell. 
3. **[Pocket Water Cells](#313-pocket-water-cells):** the cells within which independent pocket water types are generated. 

#### 3.1.1: Pre-pocket Cells

Pre-pocket cells tile the horizontal plane on a **world-anchored** axis-aligned grid, so streaming agrees across chunk boundaries.

- **Grid:** fix a **pre-pocket pitch** $W_{\text{pre}}$ (world units) and origin $(O_x, O_z)$. Any world point $(x,z)$ lies in the cell with indices
  $$
  i = \left\lfloor \frac{x - O_x}{W_{\text{pre}}} \right\rfloor,\qquad
  j = \left\lfloor \frac{z - O_z}{W_{\text{pre}}} \right\rfloor.
  $$
  The **anchor** for that pre-pocket is its **lower-left corner** $(O_x + i W_{\text{pre}},\, O_z + j W_{\text{pre}})$.
- **Pocket pitch inside the pre-pocket:** sample deterministic noise at the anchor (and a fixed salt). Map it to a **discrete** pocket pitch $W_{\text{pocket}}$ from a small allowed set (e.g. powers of two or fixed quanta). Require $W_{\text{pocket}}$ to **divide** $W_{\text{pre}}$ on both axes (or define a rule for leftover margin at the max- $x$ / max- $z$ edges). That yields an integer **$n_x \times n_z$** grid of **Pocket Cells** inside each pre-pocket.
- **Role:** $W_{\text{pocket}}$ is **constant** within one pre-pocket but **varies** between pre-pockets, so changes in pocket size over the world while staying **regionally correlated** along the pre-pocket grid.

```rust
// World xz. Pre-pocket containing (x, z):
let i = floor((x - ox) / w_pre);
let j = floor((z - oz) / w_pre);
let anchor_x = ox + i * w_pre;
let anchor_z = oz + j * w_pre;
let w_pocket = choose_pocket_pitch(anchor_x, anchor_z); // from noise, discrete set; divides w_pre
let nx = w_pre / w_pocket;
let nz = w_pre / w_pocket;

// Pocket cell indices inside this pre-pocket (0..nx, 0..nz):
let px = floor((x - anchor_x) / w_pocket).clamp(0, nx - 1);
let pz = floor((z - anchor_z) / w_pocket).clamp(0, nz - 1);
let pocket_rect = Rect::new(
    anchor_x + px * w_pocket,
    anchor_z + pz * w_pocket,
    w_pocket,
    w_pocket,
);
```

#### 3.1.2: Pocket Cells

Each **Pocket Cell** is one tile of the $n_x \times n_z$ grid inside its pre-pocket (see [3.1.1](#311-pre-pocket-cells)). Its footprint is the axis-aligned square $[x_p,\, x_p + W_{\text{pocket}}] \times [z_p,\, z_p + W_{\text{pocket}}]$.

Within that footprint, Marazion applies a **Guillotine partition** with **bounded depth** so you get **variable rectangular sub-regions** (the layout stage before **Pocket Water Cells** in [3.1.3](#313-pocket-water-cells)). Each cut is an axis-aligned line spanning the **full** width or height of the **current** piece; children tile the parent with no gaps.

- **Seed:** noise is keyed by the **lower-left** of the **current** sub-rectangle (and split depth / index), in the same deterministic style as the pre-pocket anchor.
- **Stop rule:** stop when `depth >= max_depth`, or when the next cut would leave a child smaller than a **minimum span** (world units or a fraction of $W_{\text{pocket}}$), or when the leaf is already at target granularity for hydrology typing—pick and document one scheme.
- **Split rule:** choose **vertical vs horizontal** from noise; choose **cut position** along that axis (optionally snap to a **sub-quantum** for stable BVH / hashing). Recurse on the two children.
- **Leaves:** each leaf is an axis-aligned **sub-rectangle** of the Pocket Cell; [3.1.3](#313-pocket-water-cells) treats each as a **Pocket Water Cell** for hydrology typing (lake, stream, …) and elevation stamps.

```rust
// pocket_rect from pre-pocket grid (3.1.1); lower-left (xp, zp) = (x_p, z_p).
fn guillotine_partition(rect: Rect, anchor_ll: (f64, f64), depth: u8) -> Vec<Rect> {
    if depth >= MAX_DEPTH || rect_too_small(rect, MIN_SUB_SPAN) {
        return vec![rect];
    }
    let vertical = n01(anchor_ll, depth, SPLIT_SALT) < 0.5;
    let t = choose_cut_ratio(anchor_ll, depth, vertical); // e.g. in [0.25, 0.75]
    let (a, b) = if vertical {
        guillotine_vertical(rect, t)
    } else {
        guillotine_horizontal(rect, t)
    };
    let mut out = Vec::new();
    out.extend(guillotine_partition(a, a.lower_left(), depth + 1));
    out.extend(guillotine_partition(b, b.lower_left(), depth + 1));
    out
}

// let leaves = guillotine_partition(pocket_rect, pocket_rect.lower_left(), 0);
```

#### 3.1.3: Pocket Water Cells

The following are the Pocket Water Cells which should be included in Marazion.

To ensure reliable rims, all construction rely on creating a plateau, then depressing everything within the plateau to ensure a rim around the body of water. 

##### 3.1.3.1: Lake

The **default** lake footprint is an **offset centroid** and a **noisy circular radius** inside the pocket water cell (steps 1–7). Elevation follows the usual plateau-then-depress rim recipe from the introduction to [3.1.3](#313-pocket-water-cells).

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

Stream construction is similar to the lake, but uses a **distance to path** (polyline) rather than distance to a point.

1. Pick two endpoints in the pocket-water cell from deterministic noise (optionally constrained away from very steep source terrain).
2. Choose an initial heading from start to end and a target segment length in world units.
3. Build a polyline from start to end with a **noisy hysteresis** walk: at each step, blend the previous heading and the direct heading to the endpoint, then add bounded angular jitter. This keeps coherent turns and avoids white-noise zigzags.
4. Stop when either: (a) the walk comes within a snap radius of the endpoint and connects directly, (b) max segment count is reached, or (c) the next step exits the cell and cannot be projected back safely.
5. Define stream width from noise (base half-width plus variation), then compute band masks by distance to the polyline: thalweg, wet channel, and skirt or rim.
6. Set surface grade along path arc length from upstream to downstream. The monotone drop can be made slightly noisy, as long as the max elevation added is significantly less than the depth of the stream. 
7. Raise the skirt band toward bank grade, keep channel near water surface, and depress the thalweg by depth profile.

Hysteresis path search pseudocode:

```rust
// Build a deterministic stream centerline in one cell.
fn build_stream_path(cell: Rect, anchor: Seed, start: Vec2, end: Vec2) -> Vec<Vec2> {
    let mut p = start;
    let mut dir_prev = (end - start).normalize_or_zero();
    let mut out = vec![start];

    for k in 0..MAX_SEGMENTS {
        let to_end = end - p;
        if to_end.length() <= SNAP_RADIUS {
            out.push(end);
            break;
        }

        let dir_goal = to_end.normalize_or_zero();
        let theta = angle_jitter(anchor, k) * MAX_TURN_RADIANS; // in [-max,+max]
        let blended = normalize(lerp(dir_goal, dir_prev, HYSTERESIS));
        let dir = rotate(blended, theta);

        let mut q = p + dir * STEP_LEN;
        if !cell.contains(q) {
            q = project_to_rect(q, cell);
            if distance(q, p) < MIN_PROGRESS {
                break;
            }
        }

        out.push(q);
        p = q;
        dir_prev = dir;
    }

    // Ensure endpoint closure if still reasonably close.
    if distance(*out.last().unwrap_or(&start), end) <= CONNECT_RADIUS {
        out.push(end);
    }
    out
}
```

Elevation modulation pseudocode:

```rust
// Modulate terrain at sample (x,z) from stream polyline and graded surface.
fn stamp_stream_height(base_h: f32, x: f32, z: f32, path: &[Vec2], anchor: Seed) -> f32 {
    let (d, s) = distance_and_arclen_to_polyline(vec2(x, z), path);
    // d = shortest distance to centerline, s = arc length coordinate of closest point.

    let half_w = BASE_HALF_WIDTH + width_noise(anchor, x, z);
    let thalweg_w = THALWEG_RATIO * half_w;
    let skirt_w = half_w + SKIRT_EXTRA;

    // Monotone downstream grade along path.
    let surface = surface_at_head(anchor) - GRADE_PER_METER * s;
    let depth = depth_profile(anchor, s);

    let mut h = base_h;

    // Outer skirt / bank shaping.
    if d < skirt_w {
        let t = smoothstep(skirt_w, half_w, d);
        h = h.max(lerp(base_h, surface + bank_noise(anchor, x, z), t));
    }

    // Wet channel near surface.
    if d < half_w {
        h = h.min(surface + channel_noise(anchor, x, z));
    }

    // Thalweg depression.
    if d < thalweg_w {
        let u = 1.0 - (d / thalweg_w).clamp(0.0, 1.0);
        h -= u * depth;
    }

    h
}
```

> [!NOTE]
> If stream construction raises too many unnatural ridges, constrain it by increasing skirt width, lowering bank lift, reducing max turn per step, or rejecting start and end seeds on steep terrain.

##### 3.1.3.3: Bog

Bogs are a **cluster of small lake-like basins** generated from local candidate centroids on a coarse lattice. Compared to a lake, bogs favor many shallow pockets and wetter interstitial ground.

1. Choose a bog lattice pitch $W_{\text{bog}}$ and anchor from the pocket-water cell lower-left corner.
2. For each sample $(x,z)$, map into bog-lattice coordinates and gather nearby centroid candidates from floor and ceiling lattice corners (2x2 set around the sample).
3. Jitter each candidate centroid deterministically from its lattice coordinate; evaluate an activation threshold (noise + optional slope filter) so only some centroids are active.
4. Select the nearest active centroid. If none is active, we may treat the sample as bog fringe--i.e., no deep basin carve; optional slight wetting and flattening only.
5. For the chosen centroid, build a small basin exactly like Lake with local parameters (surface, depth, radius, rim widths), but with shallower depth and tighter radius ranges.
6. Blend overlapping candidate influence softly so transitions between adjacent micro-basins do not create hard seams.
7. Apply final bog mask noise to keep edges ragged and avoid a regular checker appearance from the lattice.

Bog centroid selection and elevation pseudocode:

```rust
// Returns modified height for one sample (x, z) inside a pocket-water cell.
fn stamp_bog_height(base_h: f32, x: f32, z: f32, cell: Rect, anchor: Seed) -> f32 {
    let mut best: Option<(Vec2, f32)> = None; // (centroid, distance)

    let uv = (vec2(x, z) - cell.min()) / W_BOG;
    let i0 = uv.x.floor() as i32;
    let j0 = uv.y.floor() as i32;

    // 2x2 floor/ceil candidate set around sample.
    for di in 0..=1 {
        for dj in 0..=1 {
            let gi = i0 + di;
            let gj = j0 + dj;
            let g_anchor = lattice_anchor(cell.min(), gi, gj, W_BOG);

            if !centroid_active(g_anchor, anchor) {
                continue;
            }

            let c = g_anchor + centroid_jitter(g_anchor, anchor);
            let d = distance(vec2(x, z), c);
            if best.map(|(_, bd)| d < bd).unwrap_or(true) {
                best = Some((c, d));
            }
        }
    }

    let Some((c, d)) = best else {
        // Fringe wetting only (optional): flatten slightly toward local mean.
        return base_h + fringe_wet_lift(anchor, x, z);
    };

    // Lake-like local basin, but shallow and small.
    let surface = base_h_at(c.x, c.y) + bog_surface_noise(anchor, c);
    let depth = BOG_DEPTH_SCALE * depth_from_noise(anchor, c);
    let pre_r = dist_to_rect_boundary(c.x, c.y, cell) - BOG_MU;
    let r = (pre_r - bog_radius_jitter(anchor, x, z)).max(BOG_MIN_R);

    let inner = d < r;
    let outer = d < r + BOG_SKIRT;
    let bowl  = d < r - BOG_THALWEG_PAD;

    let mut h = base_h;
    if inner { h = h.max(surface + bog_rim_noise(anchor, x, z)); }
    if outer { h = h.max(surface + bog_skirt_noise(anchor, x, z)); }
    if bowl {
        let u = 1.0 - (d / r).clamp(0.0, 1.0);
        h -= u * depth;
    }

    // Optional final ragged mask to break lattice regularity.
    h + bog_edge_mask_noise(anchor, x, z)
}
```

> [!NOTE]
> To avoid grid artifacts, keep centroid jitter at a meaningful fraction of `W_BOG`, avoid binary activation near threshold (use a small smooth band), and randomize bog pitch between neighboring pocket-water cells only at discrete, deterministic levels.

##### 3.1.3.4: Lake into Stream

##### 3.1.3.5: Stream into Lake

### 3.2: Marazion Basin Water Stamping

### 3.3: Marazion Hydrology Complex Stamping

### 3.4: Marazion Global Ocean

## 4: Milestones