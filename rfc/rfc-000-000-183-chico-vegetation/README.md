# RFC-183: Chico Vegetation

## Table of Contents

## 1: Summary

We propose the Chico vegetation system in response to [#61](https://github.com/ramate-io/maybraid/issues/61).

## 2: Prior Art

## 3: Design

### 3.1: Stalk and Ball-stick Trees

Stalk and ball-stick trees form the core geometric system for Chico vegetation. The design refines the current system which uses [an ad hoc stalk with radial projection](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/vegetation/src/tree.rs#L171), a [`BallStick`](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/comproc/src/complex/chain/ball_stick/builder.rs) complex for the canopy, a [noisy cylinder](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/vegetation/src/tree/meshes/trunk/segment.rs) for trunk and branch segments, and a [planar canopy](https://github.com/ramate-io/maybraid/blob/9c38f45cfd697a392e6114bbc6e67b50005b7f65/procedures/vegetation/src/tree/meshes/canopy/ball.rs).

The goal is to formalize this into a composable, parameterized system:

* **Stalks** define primary vertical structure
* **Sticks** define trunk and branch segments
* **Balls and planes** define canopy and foliage
* **Ball-stick chains** unify tree construction

Tree types are then expressed as parameterized constructions over these primitives rather than bespoke implementations.

---

### 3.1.1: Stick and Stalk Components

Stick and stalk components define the structural backbone of trees. They should remain:

* deterministic from seed
* SDF-compatible for mesh and physics reuse
* composable into chains and radial projections

---

### 3.1.1.1: Noisy Cylinder

The noisy cylinder is the default segment primitive and corresponds directly to the existing [noisy cylinder implementation](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/vegetation/src/tree/meshes/trunk/segment.rs).

It defines a tapered cylinder along the $y$ axis with noise applied to its surface.

```rust
pub struct SegmentConfig {
    pub seed: u32,
    pub base_radius: f32,
    pub top_radius: f32,
    pub noise_amplitude: f32,
    pub noise_frequency: f32,
}
```

SDF sketch:

```rust
fn distance(&self, p: Vec3) -> f32 {
    let y = p.y;
    let t = y.clamp(0.0, 1.0);

    let radius = mix(
        self.base_radius,
        self.top_radius,
        t,
    );

    let radial = Vec2::new(p.x, p.z).length();
    let mut d = radial - radius;

    let n = perlin(
        p * self.noise_frequency,
        self.seed,
    );

    d += n * self.noise_amplitude;

    if y < 0.0 {
        d = d.max(-y);
    } else if y > 1.0 {
        d = d.max(y - 1.0);
    }

    d
}
```

This component is suitable for:

* trunks
* straight or mildly irregular branches
* most general-purpose segment usage

---

### 3.1.1.2: Crook Cylinder

The crook cylinder extends the noisy cylinder by introducing continuous curvature along the segment while preserving an SDF formulation.

Instead of a straight axis, the cylinder is defined around a smooth centerline:

$$
\gamma(t) =
\begin{bmatrix}
a_x \sin(\pi t + \phi_x) \
t \
a_z \sin(\pi t + \phi_z)
\end{bmatrix}
$$

...where $t \in [0,1]$ and $a_x, a_z$ control bend magnitude.

```rust
pub struct CrookConfig {
    pub segment: SegmentConfig,
    pub bend_x: f32,
    pub bend_z: f32,
    pub phase_x: f32,
    pub phase_z: f32,
}
```

SDF sketch:

```rust
fn centerline(&self, t: f32) -> Vec3 {
    Vec3::new(
        self.bend_x * (PI * t + self.phase_x).sin(),
        t,
        self.bend_z * (PI * t + self.phase_z).sin(),
    )
}

fn distance(&self, p: Vec3) -> f32 {
    let t = p.y.clamp(0.0, 1.0);

    let c = self.centerline(t);
    let q = p - c;

    let radius = mix(
        self.segment.base_radius,
        self.segment.top_radius,
        t,
    );

    let radial = Vec2::new(q.x, q.z).length();

    let n = perlin(
        p * self.segment.noise_frequency,
        self.segment.seed,
    );

    let d = radial - radius + n * self.segment.noise_amplitude;

    if p.y < 0.0 {
        d.max(-p.y)
    } else if p.y > 1.0 {
        d.max(p.y - 1.0)
    } else {
        d
    }
}
```

This produces smoothly bent trunks and branches without introducing discontinuities.

**Usage**

* stylized or expressive trunks
* bent or wind-shaped branches
* palms, banyans, and irregular growth patterns

Crook cylinders should be used deliberately, as they strongly influence silhouette and perceived species.

### 3.1.2: Ball Components

Ball components are primarily used for canopy and foliage massing. Unlike stick components, they do not generally need to be collision-supporting, though some may retain SDF backings where useful for reuse or consistency.

These components provide the visual mass of vegetation, while stick components define structure. Together, they enable a wide range of tree and plant forms through simple composition.

---

### 3.1.2.1: Icosahedron

A low-poly convex canopy primitive used primarily at far range.

**Construction**

* Static indexed mesh: 12 vertices, 20 faces
* Can be precomputed or reused via asset handle

In Bevy:

```rust
let mesh = Mesh::from(shape::Icosahedron {
    radius,
    subdivisions: 0,
});
```

**Usage**

* far LOD canopy fill
* silhouette preservation
* cheap instancing across large forests

**Notes**

* One-sided opaque shading is sufficient at distance
* Icospheres (`subdivisions > 0`) may be used for moderate LOD
* Can replace [Noisy Balls](#3122-noisy-ball) in [Plane Splays](#3125-plane-splay)

---

### 3.1.2.2: Noisy Ball

An SDF-backed spherical canopy element with surface perturbation.

**Construction**

$$
d(\mathbf{p}) = |\mathbf{p}| - r + \text{noise}(\mathbf{p})
$$

```rust
fn distance(p: Vec3) -> f32 {
    let n = perlin(p * freq + seed) * amp;
    p.length() - radius + n
}
```

Mesh generation proceeds via marching cubes or dual contouring.

**Usage**

* mid-range canopy fill
* base layer for higher-detail canopy, e.g. [Plane Splay](#3125-plane-splay)

**Notes**

* One-sided shading at range
* Two-sided shading up close
* Can be replaced by icosahedra at low LOD

---

### 3.1.2.3: Octagonal Plane

A low triangle-count planar element used within splays.

**Construction**

* 8-sided polygon in local plane
* UVs centered for radial leaf textures

```rust
let positions = regular_ngon(8, radius);
```

**Usage**

* canopy layering in [Plane Splay](#3125-plane-splay)
* mid-detail foliage clusters

**Notes**

* Billboarded or slightly tilted
* Double-sided material recommended

---

### 3.1.2.4: Triangular Plane

Minimal planar primitive used for fine foliage and fronds.

**Construction**

```rust
let positions = [
    Vec3::new(0.0, 0.0, 0.0),
    Vec3::new(w, 0.0, 0.0),
    Vec3::new(0.0, h, 0.0),
];
```

**Usage**

* fronds
* fine canopy breakup
* edge detailing in splays

**Notes**

* Very low cost
* Best used in groups, chains, or splayed clusters
* Usually double-sided

---

### 3.1.2.5: Plane Splay

A high-detail canopy construction derived from the original [`NoisyBall`](https://github.com/ramate-io/maybraid/blob/9c38f45cfd697a392e6114bbc6e67b50005b7f65/procedures/vegetation/src/tree/meshes/canopy/ball.rs#L102-L231).

Plane Splay combines:

* a central noisy ball or implicit volume
* multiple outward-facing planes
* octagonal or triangular planar elements
* radial or hemispherical distribution

**Construction**

```rust
for i in 0..N {
    let dir = sample_sphere(seed, i);
    let pos = center + dir * radius;

    spawn_plane(
        position = pos,
        normal = dir,
        scale = plane_scale(seed, i),
    );
}
```

Planes may be emitted as independent meshes for instancing or merged into a single mesh for fewer draw calls.

**Usage**

* high LOD canopy
* outer canopy layers
* silhouette refinement
* leaf clusters around ball-stick nodes

**Notes**

* Prefer placing planes near canopy surface
* Avoid dense interior placement
* Combine with noisy ball or icosphere for volume

---

### 3.1.2.6: Tufts

A jagged, outward-projecting canopy component with an SDF backing, based on the existing [tuft implementation](https://github.com/ramate-io/maybraid/blob/9c38f45cfd697a392e6114bbc6e67b50005b7f65/procedures/terrain/src/detail/meshes/tuft.rs#L27).

**Construction**

Tufts are composed as a cluster of projecting elements from a shared origin. They are SDF-generated rather than purely planar.

```rust
fn distance(p: Vec3) -> f32 {
    let d = base_shape(p);
    let spikes = directional_noise(p, seed) * amplitude;

    d - spikes
}
```

Mesh generation proceeds via standard SDF meshing.

**Usage**

* sprouting trees
* jungle growths on branches
* canopy detail layers
* ground cover

**Notes**

* Can be used at all LOD when visible
* Cull when occluded by larger canopy elements
* Useful as both vegetation detail and terrain detail

---

### 3.1.2.7: Fronds

Fronds are mesh-based arching chains of triangular or narrow quad planes. They are used for palms, bushes, and jungle growth. They should not be SDF-backed unless collision is later required.

**Construction**

A frond is defined by a curved spine and a sequence of planar leaflets attached along it.

```rust
pub struct FrondConfig {
    pub segments: usize,
    pub length: f32,
    pub width: f32,
    pub droop: f32,
    pub twist: f32,
    pub leaflet_count: usize,
}
```

A simple spine:

```rust
fn spine(t: f32, config: &FrondConfig) -> Vec3 {
    let x = t * config.length;
    let y = -config.droop * t * t;

    Vec3::new(x, y, 0.0)
}
```

Leaflets are placed along the spine:

```rust
for i in 0..config.leaflet_count {
    let t = i as f32 / (config.leaflet_count - 1) as f32;

    let p = spine(t, config);
    let tangent = normalize(
        spine(t + EPS, config) - spine(t - EPS, config)
    );

    let side = if i % 2 == 0 { 1.0 } else { -1.0 };
    let width = config.width * (1.0 - t);
    let lateral = side * width;

    emit_triangle_or_quad(
        root = p,
        tangent = tangent,
        lateral = lateral,
        twist = config.twist * t,
    );
}
```

**Mesh strategy**

Fronds should usually be emitted as one combined mesh per frond. For palm crowns, multiple fronds may be merged into one mesh per crown ring or one mesh per tree.

**Usage**

* palm crowns
* fern-like bushes
* jungle branch growth
* sparse tropical canopy detail

**Notes**

* Use double-sided foliage materials
* Taper leaflet size toward the tip
* Add mild noise to spine droop and leaflet angle
* Prefer mesh construction over SDF unless collision is needed

---

### 3.1.2.8: Jessen's Icosahedron

[Jessen's icosahedron](https://en.wikipedia.org/wiki/Jessen%27s_icosahedron) is a non-convex variation of the icosahedron used for additional visual variety.

**Usage**

* replace standard icosahedra at far LOD
* introduce irregular silhouettes
* mix with icospheres for variation
* reduce repetition in distant forests

**Notes**

* Especially useful when many distant trees would otherwise share the same silhouette
* Selection can be randomized per instance
* Covered more completely in [Tree LOD Tricks](#318-tree-lod-tricks)

Here’s a refined replacement for **3.1.3–3.1.5** incorporating those points.

---

### 3.1.3: Ball-stick Anchors

Ball-stick anchors define where a branch, canopy chain, or foliage complex begins. The current system does this implicitly through [radial projection from an ad hoc stalk](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/vegetation/src/tree.rs#L171), then passes the resulting start point and ray into a [`BallStick`](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/comproc/src/complex/chain/ball_stick/builder.rs) construction.

The refinement is to treat anchoring as a named step. An anchor records the start position, initial growth direction, bias direction, and local scale for a branch or canopy chain.

```rust
pub struct BallStickAnchor {
    pub position: Vec3,
    pub initial_ray: Vec3,
    pub bias_ray: Vec3,
    pub radius: f32,
}
```

Anchors are usually placed at or near the **stalk radial centroid** rather than directly on the visible stalk surface. This reduces the chance that branches appear detached or improperly projected. The branch mesh can still emerge visually from the stalk surface because the first stick segment projects outward from the centroid.

Common anchor routines include:

* stalk-height sampling for ordinary branches
* radial rings around the stalk for canopies
* crown-only rings for palms
* node-derived anchors for secondary growth
* downward-biased anchors for banyan descenders
* ground anchors for bushes, shoots, and trunkless palms

Tree recipes should compose these routines rather than hard-code anchor placement. For example:

```rust
let anchors = stalk_rings(...)
    .with_height_bias(...)
    .with_radial_count(...)
    .with_direction_rule(...);
```

This makes constructions such as conifers, palms, banyans, and vase trees variations over anchor selection rather than separate systems.

---

### 3.1.4: Ball-stick Chains

Ball-stick chains describe the branching skeleton grown from an anchor. The existing [`BallStickBuilder`](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/comproc/src/complex/chain/ball_stick/builder.rs) already captures the core idea: extend from parent nodes, choose child count, sample direction, assign radii, and emit connected nodes.

At a high level, a chain is a directed graph:

```rust
pub struct BallStickChain {
    pub nodes: Vec<BallStickNode>,
    pub segments: Vec<BallStickSegment>,
}
```

Each segment becomes a stick component, usually a [noisy cylinder](#3111-noisy-cylinder) or [crook cylinder](#3112-crook-cylinder). Each node becomes a possible site for foliage, descenders, joint-concealing balls, or additional child chains.

The key behavior to preserve is **directional hysteresis**. A segment should blend prior direction, bias direction, and bounded noise rather than sampling a fresh direction independently.

```rust
let mean = blend(previous_ray, bias_ray, bias_strength);
let ray = perturb(mean, angle_tolerance, seed);
let child = parent.position + ray * segment_length;
```

However, several intended tree constructions require the hysteresis behavior to vary across the graph. Banyan descenders need every nth segment to bias downward. Vase and torch trees need the bias to change with height. Conifers need radial projections to shorten and shift angle with vertical position.

Accordingly, the refined chain model should allow a rule mapping node context to a hysteresis configuration:

```rust
pub struct ChainContext {
    pub depth: usize,
    pub branch_order: usize,
    pub height_fraction: f32,
    pub segment_index: usize,
    pub parent_ray: Vec3,
    pub anchor: BallStickAnchor,
}

pub struct HysteresisConfig {
    pub bias_ray: Vec3,
    pub bias_strength: f32,
    pub angle_tolerance: f32,
    pub length_range: Range<f32>,
    pub radius_range: Range<f32>,
    pub child_count: Range<usize>,
}
```

Conceptually:

```rust
fn hysteresis_for(ctx: ChainContext) -> HysteresisConfig {
    if ctx.segment_index % 4 == 0 {
        downward_descender_config()
    } else {
        ordinary_canopy_config(ctx.height_fraction)
    }
}
```

This can be implemented as either:

- (1) a vector of hysteresis configs indexed by depth or segment index, or
- (2) a rule object that maps `ChainContext` to `HysteresisConfig`.

The rule form is more expressive and better matches the proposed named trees. It allows the construction to say “bias upward near the top,” “bias downward every fourth segment,” or “reduce radial length logarithmically with height” without creating a new chain builder.

When a chain is processed as a branch, nodes should optionally receive small noisy balls using the same material as the stick or stalk. These joint balls conceal intersections between cylinders and make the branch graph read as continuous organic growth rather than assembled tubes.

```rust
if render_joint_balls {
    spawn_noisy_ball(
        position = node.position,
        radius = node.radius,
        material = stick_material,
    );
}
```

---

### 3.1.5: Ball Selection

Ball selection means deciding **which nodes in the ball-stick graph receive foliage or canopy mass**, not choosing the concrete ball mesh type. The current system effectively allocates canopy at branch nodes through the existing tree spawning flow and [planar canopy construction](https://github.com/ramate-io/maybraid/blob/9c38f45cfd697a392e6114bbc6e67b50005b7f65/procedures/vegetation/src/tree/meshes/canopy/ball.rs). The refinement is to make node selection an explicit policy of each tree construction.

This policy is mostly recipe-specific. A tree recipe traverses the graph and marks nodes for canopy allocation according to its intended silhouette.

Typical rules include:

* allocate only at terminal nodes
* allocate on outer canopy layers
* skip hidden interior nodes
* allocate more densely near the crown
* allocate along every node for dense jungle growth
* allocate sparse tufts on selected branch joints
* allocate fronds only on crown-ring anchors

A selector should use graph and anchor context:

```rust
pub struct BallSelectionContext {
    pub depth: usize,
    pub branch_order: usize,
    pub height_fraction: f32,
    pub distance_from_anchor: f32,
    pub is_terminal: bool,
    pub child_count: usize,
}
```

Conceptually:

```rust
fn should_allocate_ball(ctx: BallSelectionContext) -> bool {
    ctx.is_terminal || ctx.height_fraction > 0.65
}
```

This supports the named constructions later in the proposal:

* storybook trees favor outer and terminal canopy nodes
* conifers favor small allocations along many short radial projections
* banyans allocate broadly across upper canopy and descender nodes
* palms allocate primarily from crown-ring anchors
* jungle growths add secondary allocations at selected canopy nodes

The concrete component used at a selected node, such as noisy ball, plane splay, tuft, or frond, is a separate decision handled by the tree recipe and eventual LOD strategy.


### 3.1.6: Well-known Component Constructions

This section lists reusable component-level constructions. These are not complete tree recipes; they are smaller routines that named tree constructions can compose.

---

##### [3.1.6.1: Palm Crown](./well-knowns/component-construction/palm-crown/README.md)

##### [3.1.6.2: Palm Trunk](./well-knowns/component-construction/palm-trunk/README.md)

##### [3.1.6.3: High-bushes and Shoots](./well-knowns/component-construction/high-bushes-and-shoots/README.md)

##### [3.1.6.4: Jungle Growths](./well-knowns/component-construction/jungle-growths/README.md)

##### [3.1.6.5: Banyan Trunk](./well-knowns/component-construction/banyan-trunk/README.md)

##### [3.1.6.6: Banyan Descenders](./well-knowns/component-construction/banyan-descenders/README.md)

##### [3.1.6.7: Fruiting Bodies](./well-knowns/component-construction/fruiting-bodies/README.md)

### 3.1.7: Well-known Tree Constructions

We provided the intended tree shapes for Chico vegetation. Note that many of these shapes can be used with a variety of textures and scales to produce the impressions of different species. 

##### [3.1.7.1: Storybook Tree](./well-knowns/tree-construction/storybook-tree/README.md)

##### [3.1.7.2: Liam's Conifer](./well-knowns/tree-construction/liams-conifer/README.md)

##### [3.1.7.3: Vase Tree](./well-knowns/tree-construction/vase-tree/README.md)

##### [3.1.7.4: Penmarch Torch](./well-knowns/tree-construction/penmarch-torch/README.md)

##### [3.1.7.5: Honu Banyan](./well-knowns/tree-construction/honu-banyan/README.md)

##### [3.1.7.6: Sope's Banyan](./well-knowns/tree-construction/sopes-banyan/README.md)

##### [3.1.7.7: Rory's Head-trained](./well-knowns/tree-construction/rorys-head-trained/README.md)

##### [3.1.7.8: Waialea Palm](./well-knowns/tree-construction/waialea-palm/README.md)

##### [3.1.7.9: Date Palm](./well-knowns/tree-construction/date-palm/README.md)

##### [3.1.7.10: Palm Bush](./well-knowns/tree-construction/palm-bush/README.md)

##### [3.1.7.11: Northern Conifer](./well-knowns/tree-construction/northern-conifer/README.md)

##### [3.1.7.12: Common High Bush](./well-knowns/tree-construction/common-high-bush/README.md)

##### [3.1.7.13: Jungle Storybook Tree](./well-knowns/tree-construction/jungle-storybook-tree/README.md)

##### [3.1.7.13: Braid Oak](./well-knowns/tree-construction/braid-oak/README.md)

##### [3.1.7.14: Friend's Conifer](./well-knowns/tree-construction/friends-conifer/README.md)

##### [3.1.7.15: Temperate Conifer](./well-knowns/tree-construction/temperate-conifer/README.md)

##### [3.1.7.16: Simpleman's Hedge](./well-knowns/tree-construction/simplemans-hedge/README.md)

##### [3.1.7.17: Simpleman's Tuft](./well-knowns/tree-construction/simplemans-tuft/README.md)

### 3.1.8: Tree LOD Tricks

This section outlines simple, high-impact techniques for reducing geometry and draw cost while preserving silhouette and visual variety across distance. The general principle is:

* preserve **silhouette first**
* preserve **mass second**
* drop **structure (branches) early**

Where possible, prefer **fewer meshes, fewer draw calls, and simpler topology** over geometric fidelity.

---

### 3.1.8.1: Performant Very Low-LOD Canopy

Use a single primitive to approximate canopy mass:

* upside-down square pyramid
* squashed tetrahedron

These shapes:

* approximate canopy taper
* are extremely cheap (4–5 faces)
* read well at distance when shaded correctly

```rust
spawn_mesh(upside_down_pyramid(scale));
```

Use slight vertical squash for broader canopies.

---

### 3.1.8.2: Performant Very Low-LOD Trunks

Use a stretched tetrahedron or square pyramid.

```rust
spawn_mesh(stretched_pyramid(height, radius));
```

These give:

* strong vertical read
* minimal geometry
* acceptable silhouette at long range

---

### 3.1.8.3: Performant Very Low-LOD Branches

Do not allocate any branches.

Branches do not contribute meaningfully at this distance and only add draw cost.

---

### 3.1.8.4: Performant Low-LOD Canopy

Use stretched icosahedra to approximate canopy shape:

* one vertical icosahedron for tall forms
* one horizontal (squashed) icosahedron for wide forms
* combine two for vase-like or complex shapes

```rust
spawn_mesh(icosahedron(scale));
```

This preserves:

* rounded silhouette
* better shading than pyramids
* low triangle count

---

### 3.1.8.5: Performant Low-LOD Trunks

Use a hexagonal prism.

```rust
spawn_mesh(hex_prism(height, radius));
```

This gives:

* cylindrical impression
* low polygon count
* good normal interpolation

---

### 3.1.8.6: Performant Low-LOD Branches

Do not allocate branch meshes.

Branch silhouettes are implied by canopy shape at this level.

---

### 3.1.8.7: Performant Moderate-LOD Canopy

Use:

* icosahedra
* icospheres (low subdivision)

Mixing both helps preserve organic variation while keeping geometry simple.

```rust
spawn_mesh(icosphere(subdivisions = 1..2));
```

---

### 3.1.8.8: Performant Moderate-LOD Trunks

Use lower sample-rate [Noisy Cylinder](#3111-noisy-cylinder).

```rust
NoisyCylinder {
    noise_frequency: lower,
    mesh_resolution: reduced,
}
```

This preserves trunk character while reducing vertex count.

---

### 3.1.8.9: Performant Moderate-LOD Branches

Use low-resolution noisy cylinders for major branches only:

* skip smaller branches
* merge segments where possible

```rust
segments: 1..=2
```

---

### 3.1.8.10: Varied Low-LOD Canopy

Use noise to select between primitive types:

* icosahedron
* tetrahedron

```rust
if noise(seed) < 0.5 {
    use_icosahedron();
} else {
    use_tetrahedron();
}
```

This reduces repetition across distant forests.

---

### 3.1.8.11: Varied Moderate-LOD Canopy

Use noise to vary between:

* standard icosahedron
* [Jessen's Icosahedron](https://en.wikipedia.org/wiki/Jessen%27s_icosahedron)

```rust
if noise(seed) < 0.5 {
    use_icosahedron();
} else {
    use_jessen();
}
```

This subtly breaks silhouette uniformity without increasing cost.

---

### 3.1.8.12: Silhouette-Preserving Scaling

At lower LODs, slightly exaggerate large-scale proportions to preserve readability:

* widen canopy by a small factor
* slightly shorten trunk
* reduce taper

```rust
let canopy_scale = 1.05..1.15;
let trunk_scale = 0.9..0.95;
```

This compensates for the loss of fine structure and prevents trees from appearing thin or brittle at distance.

---

### 3.1.8.13: Random Rotation and Skew

Introduce small deterministic variation in orientation and scale to break repetition across instances.

**Rotation**

Rotate around the vertical axis:

```rust
let yaw = TAU * noise(seed, ROT_SALT);
transform.rotate_y(yaw);
```

This prevents aligned silhouettes across large forests.

**Non-uniform scale (skew-like effect)**

Apply slight variation in horizontal axes:

```rust
let sx = 0.9 + 0.2 * noise(seed, SCALE_X);
let sz = 0.9 + 0.2 * noise(seed, SCALE_Z);

transform.scale *= Vec3::new(sx, 1.0, sz);
```

This produces:

* slight elongation or compression
* variation in canopy footprint
* reduced tiling artifacts

**Optional lean (very subtle)**

```rust
let lean = 0.05 * noise(seed, LEAN_SALT);
transform.rotate_axis(Vec3::Z, lean);
```

Use sparingly; excessive lean breaks vertical readability.

These small variations are critical for avoiding visual repetition when using low-LOD primitives.

---

### 3.1.8.14: Vertical Color Gradient

Apply a simple vertical gradient in the shader:

* darker near trunk base
* lighter near canopy top

This simulates:

* ambient occlusion
* light falloff
* canopy density

...without adding geometry.

---

### 3.1.8.15: Material Simplification

Reduce shader complexity at lower LODs:

* remove normal maps
* reduce texture lookups
* flatten roughness variation

This reduces GPU cost and improves batching while maintaining overall color and silhouette.

---

These techniques combine to produce large, varied forests at low cost while preserving convincing silhouettes and biome identity.

### 3.1.9: Stick Shading

Stick shading covers trunks, branches, descenders, and exposed woody structures. The goal is to avoid flat bark color while allowing each tree species to define its own palette.

This follows the same basic idea as [world-space ground color noise](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-170-terrain-detail#33-world-space-ground-color-noise): color variation is sampled in world space, so nearby points are visually related and variation remains stable across generated chunks.

#### 3.1.9.1: Species Palette

Each tree species provides a small ordered palette of stick colors.

```rust
pub struct StickPalette {
    pub colors: [Vec3; 4],
    pub regional_scale: f32,
    pub detail_scale: f32,
    pub value_strength: f32,
}
```

Example palette:

```rust
let bark_palette = [
    vec3(0.22, 0.15, 0.10), // dark brown
    vec3(0.36, 0.26, 0.18), // warm bark
    vec3(0.42, 0.36, 0.28), // gray bark
    vec3(0.16, 0.12, 0.09), // dark crevice
];
```

Species can bias toward gray, red, yellow, black, or pale bark tones.

#### 3.1.9.2: World-space Variation

Use low-frequency noise to choose a palette region and higher-frequency noise to modulate bark detail.

```wgsl
let regional = fbm(world_position.xz * stick.regional_scale, stick.seed);
let detail = fbm(world_position.xyz * stick.detail_scale, stick.seed + 101u);
```

The regional sample drives broad color variation. The detail sample adds local bark irregularity.

#### 3.1.9.3: WGSL Sketch

```wgsl
struct StickShaderParams {
    seed: u32,
    regional_scale: f32,
    detail_scale: f32,
    value_strength: f32,
    color0: vec3<f32>,
    _pad0: f32,
    color1: vec3<f32>,
    _pad1: f32,
    color2: vec3<f32>,
    _pad2: f32,
    color3: vec3<f32>,
    _pad3: f32,
};

@group(1) @binding(0)
var<uniform> stick: StickShaderParams;

fn fbm(p: vec3<f32>, seed: u32) -> f32 {
    // Placeholder: use standard value noise, Perlin, or existing project fbm.
    return fract(sin(dot(p, vec3<f32>(12.9898, 78.233, 37.719)) + f32(seed)) * 43758.5453);
}

fn palette_sample(t: f32) -> vec3<f32> {
    let t = clamp(t, 0.0, 1.0);
    let x = t * 3.0;
    let i = u32(floor(x));
    let f = fract(x);

    if (i == 0u) {
        return mix(stick.color0, stick.color1, f);
    }
    if (i == 1u) {
        return mix(stick.color1, stick.color2, f);
    }

    return mix(stick.color2, stick.color3, f);
}

fn stick_color(world_position: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let regional = fbm(world_position * stick.regional_scale, stick.seed);
    let detail = fbm(world_position * stick.detail_scale, stick.seed + 101u);

    let base = palette_sample(regional);

    let value = mix(
        1.0 - stick.value_strength,
        1.0 + stick.value_strength,
        detail,
    );

    // Optional: darken upward-facing creases less than side-facing bark.
    let side = 1.0 - abs(normal.y);
    let side_shade = mix(0.9, 1.05, side);

    return base * value * side_shade;
}
```

#### 3.1.9.4: Notes

* Use world-space coordinates, not UVs, so bark color stays stable across generated meshes.
* Use species palettes for broad identity and noise for local variation.
* Low-frequency noise should shift between bark tones; high-frequency noise should only modulate value.
* This can be shared by trunks, branches, descenders, and joint-concealing bark balls.

### 3.1.10: Leaf Shading

Leaf shading follows the same world-space palette approach as [Stick Shading](#319-stick-shading), but adds time, longitude, and altitude controls. The goal is to support stable foliage variation, seasonal color changes, snow accumulation, spring buds, and localized flecking without changing the underlying vegetation geometry.

#### 3.1.10.1: Base Leaf Palette

Each species provides a base palette for foliage color.

```rust
pub struct LeafPalette {
    pub colors: Vec<Vec3>,
    pub regional_scale: f32,
    pub detail_scale: f32,
    pub value_strength: f32,
}
```

World-space noise selects and modulates the base color:

```wgsl
let regional = fbm(world_position.xyz * leaf.regional_scale, leaf.seed);
let detail = fbm(world_position.xyz * leaf.detail_scale, leaf.seed + 101u);

let base = palette_sample(regional);
let value = mix(1.0 - leaf.value_strength, 1.0 + leaf.value_strength, detail);
```

#### 3.1.10.2: Flecks

A fleck is an additional color contribution applied over the base leaf color. Flecks are used for snow, buds, flowers, disease, dryness, or other localized seasonal effects.

```rust
pub struct LeafFleck {
    pub color: Vec3,
    pub strength: f32,

    pub season_center: f32,
    pub season_width: f32,
    pub season_cutoff: f32,

    pub longitude_divisor: f32,
    pub altitude_divisor: f32,

    pub season_weight: f32,
    pub longitude_weight: f32,
    pub altitude_weight: f32,

    pub noise_scale: f32,
    pub noise_cutoff: f32,
}
```

Each fleck computes a likelihood or strength from:

* season
* longitude
* altitude
* local world-space noise

The hard cutoff ensures the fleck can fully disappear rather than merely fade.

#### 3.1.10.3: Season, Longitude, and Altitude Terms

Season is cyclic over a normalized year:

```rust
let season: f32; // 0..1
```

A simple cyclic season response:

```wgsl
fn cyclic_window(t: f32, center: f32, width: f32) -> f32 {
    let d = abs(fract(t - center + 0.5) - 0.5);
    return smoothstep(width, 0.0, d);
}
```

Longitude and altitude can be normalized into coarse environmental masks:

```wgsl
let lon_term = fbm(vec3<f32>(world_position.x / fleck.longitude_divisor, 0.0, 0.0), seed);
let alt_term = smoothstep(alt_min, alt_max, world_position.y);
```

The combined fleck strength is:

```wgsl
let env =
    fleck.season_weight * season_term +
    fleck.longitude_weight * lon_term +
    fleck.altitude_weight * alt_term;

let env = env / max(
    0.0001,
    fleck.season_weight + fleck.longitude_weight + fleck.altitude_weight,
);
```

Then apply local fleck noise:

```wgsl
let local = fbm(world_position.xyz * fleck.noise_scale, seed);
let mask = env * local;
```

If `mask < fleck.noise_cutoff`, the fleck is absent.

#### 3.1.10.4: WGSL Sketch

```wgsl
const MAX_LEAF_COLORS: u32 = 4u;
const MAX_FLECKS: u32 = 4u;

struct LeafFleck {
    color: vec3<f32>,
    strength: f32,

    season_center: f32,
    season_width: f32,
    season_cutoff: f32,
    longitude_divisor: f32,

    altitude_start: f32,
    altitude_end: f32,
    altitude_divisor: f32,
    noise_scale: f32,

    season_weight: f32,
    longitude_weight: f32,
    altitude_weight: f32,
    noise_cutoff: f32,
};

struct LeafShaderParams {
    seed: u32,
    color_count: u32,
    fleck_count: u32,
    _pad0: u32,

    regional_scale: f32,
    detail_scale: f32,
    value_strength: f32,
    _pad1: f32,

    colors: array<vec4<f32>, 4>,
    flecks: array<LeafFleck, 4>,
};

@group(1) @binding(0)
var<uniform> leaf: LeafShaderParams;

@group(1) @binding(1)
var<uniform> season_time: f32;

fn fbm(p: vec3<f32>, seed: u32) -> f32 {
    // Placeholder: use standard value noise, Perlin, or project fbm.
    return fract(sin(dot(p, vec3<f32>(12.9898, 78.233, 37.719)) + f32(seed)) * 43758.5453);
}

fn cyclic_window(t: f32, center: f32, width: f32) -> f32 {
    let d = abs(fract(t - center + 0.5) - 0.5);
    return smoothstep(width, 0.0, d);
}

fn palette_sample(t: f32) -> vec3<f32> {
    let count = max(leaf.color_count, 1u);
    let max_i = count - 1u;

    let x = clamp(t, 0.0, 1.0) * f32(max_i);
    let i = min(u32(floor(x)), max_i);
    let j = min(i + 1u, max_i);
    let f = fract(x);

    return mix(
        leaf.colors[i].rgb,
        leaf.colors[j].rgb,
        f,
    );
}

fn fleck_mask(
    fleck: LeafFleck,
    world_position: vec3<f32>,
    seed: u32,
) -> f32 {
    let season_term = cyclic_window(
        season_time,
        fleck.season_center,
        fleck.season_width,
    );

    if (season_term < fleck.season_cutoff) {
        return 0.0;
    }

    let lon_term = fbm(
        vec3<f32>(
            world_position.x / max(fleck.longitude_divisor, 0.0001),
            0.0,
            0.0,
        ),
        seed + 17u,
    );

    let alt_base = smoothstep(
        fleck.altitude_start,
        fleck.altitude_end,
        world_position.y,
    );

    let alt_noise = fbm(
        vec3<f32>(
            0.0,
            world_position.y / max(fleck.altitude_divisor, 0.0001),
            0.0,
        ),
        seed + 31u,
    );

    let altitude_term = alt_base * alt_noise;

    let denom = max(
        0.0001,
        fleck.season_weight
            + fleck.longitude_weight
            + fleck.altitude_weight,
    );

    let env = (
        season_term * fleck.season_weight
        + lon_term * fleck.longitude_weight
        + altitude_term * fleck.altitude_weight
    ) / denom;

    let local = fbm(world_position * fleck.noise_scale, seed + 47u);
    let mask = env * local;

    if (mask < fleck.noise_cutoff) {
        return 0.0;
    }

    return mask;
}

fn leaf_color(world_position: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let regional = fbm(world_position * leaf.regional_scale, leaf.seed);
    let detail = fbm(world_position * leaf.detail_scale, leaf.seed + 101u);

    var color = palette_sample(regional);

    let value = mix(
        1.0 - leaf.value_strength,
        1.0 + leaf.value_strength,
        detail,
    );

    color = color * value;

    for (var i = 0u; i < MAX_FLECKS; i = i + 1u) {
        if (i >= leaf.fleck_count) {
            break;
        }

        let fleck = leaf.flecks[i];
        let mask = fleck_mask(fleck, world_position, leaf.seed + i * 131u);
        let amount = clamp(mask * fleck.strength, 0.0, 1.0);

        color = mix(color, fleck.color, amount);
    }

    return color;
}
```

#### 3.1.10.5: Usage

**Snow**

Use white flecks with strong season, latitude or longitude, and altitude weighting.

```rust
LeafFleck {
    color: Vec3::splat(0.95),
    strength: 0.8,
    season_center: winter,
    season_width: winter_width,
    season_cutoff: 0.2,
    longitude_weight: 0.4,
    altitude_weight: 0.6,
    season_weight: 1.0,
    noise_cutoff: 0.45,
}
```

Trees in the same region should generally share similar snow fleck parameters regardless of species, unless understory shielding or grove-specific effects apply.

**Spring buds**

Use bright green, yellow, pink, or white flecks early in the season. Bias primarily by season with slight longitude variation.

```rust
LeafFleck {
    color: bud_color,
    strength: 0.5,
    season_center: early_spring,
    season_width: short,
    season_weight: 1.0,
    longitude_weight: 0.2,
    altitude_weight: 0.1,
    noise_cutoff: 0.55,
}
```

**Overlapping flecks**

Multiple flecks may overlap. Their `strength` controls how aggressively each fleck blends over the current color. This lets snow, buds, flowers, and leaf-color variation coexist without requiring separate materials.

### 3.2: L-system Trees

L-systems are a well-established method for generating botanical structures and offer a natural way to express recursive growth, branching grammars, and species variation. They are a strong candidate for future expansion of the vegetation system.

However, we avoid adopting L-systems at this stage for a few practical reasons.

Primarily, L-systems do not provide direct spatial implications. While they are excellent for describing *connectivity* and *growth rules*, they do not inherently encode:

* world-space positioning
* spatial ownership or containment
* chunk alignment or LOD boundaries

As a result, additional interpretation layers are required to map symbolic structures into spatial ones. This introduces a tradeoff:

* **Composability**: combining multiple growth rules and structures cleanly
* **Connectivity**: maintaining coherent, continuous geometry in world space

Naive approaches tend to struggle to satisfy both simultaneously. Either the system becomes difficult to compose across species and features, or spatial coherence becomes fragile and expensive to maintain.

In contrast, the ball-stick and radial projection constructions used here:

* operate directly in world space
* align naturally with chunking and LOD systems
* provide predictable spatial ownership
* remain easy to compose and parameterize

While less expressive than full L-systems, they are significantly more practical for initial terrain-scale vegetation.

L-systems remain an important area of future work. In particular, hybrid approaches that:

* retain spatial grounding
* incorporate limited grammar-based growth
* or use L-systems as local refinements within existing structures

...may offer a path to richer vegetation without sacrificing system ergonomics or performance.

### 3.3: Ground Cover

Ground cover provides the lowest layer of vegetation detail and is responsible for visually filling terrain with grasses, moss, scrub, and low-lying plant matter. It should be:

* dense but inexpensive
* spatially stable
* driven primarily by terrain properties (elevation, slope, biome)
* composable with higher-level vegetation systems

The approach combines **continuous surface modification (bump outs)** with **discrete volumetric detail (tufts)**.

---

### 3.3.1: Bump Outs

Ground cover primarily relies on a similar [bump out](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-170-terrain-detail#34-bump-outs) method to RFC-170. These modify the underlying terrain SDF to introduce small-scale height variation representing grass beds, moss, soft soil, or low vegetation mats.

Construction follows:

* define a cell or region
* sample noise to determine coverage
* apply a bounded vertical displacement to the terrain SDF

```rust
let mask = noise(world_position * scale);

if mask > threshold {
    let height = amplitude * smooth(mask);
    sdf += height;
}
```

Key characteristics:

* **continuous**: no discrete meshes required
* **cheap**: operates in terrain generation phase
* **stable**: tied to world-space coordinates
* **biome-driven**: parameters vary with terrain conditions

Detail is primarily expressed through [Leaf Shaders](#3110-leaf-shading):

* color variation (greens, yellows, browns)
* seasonal effects (drying, snow cover)
* flecking (flowers, debris, moss variation)

Bump outs provide the **base visual mass** of ground vegetation.

---

### 3.3.2: Tufts

Extra volumetric detail is provided by [Tufts](#332-tufts). These add discrete geometry to break up the flatness of bump-out-only surfaces.

Tufts are:

* SDF-based or mesh-based clumps
* sparsely distributed over bump-out regions
* oriented by terrain normal
* scaled and rotated deterministically

```rust
if noise(seed) > placement_threshold {
    spawn_tuft(
        position = terrain_position,
        direction = terrain_normal,
        scale = tuft_scale,
    );
}
```

As detailed in [Tufts](#3522-tufts-layer), tufts should be handled as a **separate layer** from bump outs:

* bump outs define coverage and base density
* tufts provide localized vertical structure

This separation allows:

* independent LOD control
* independent density tuning
* better performance scaling

**Placement considerations**

* bias placement toward flatter regions or slight slopes
* avoid excessive clustering unless biome requires it
* reduce density near large vegetation or obstacles

**Usage**

* grasses and scrub
* jungle undergrowth
* dry brush
* moss clumps
* small flowering plants

Together, bump outs and tufts provide a scalable and performant ground cover system that integrates cleanly with terrain and vegetation layers.

Good call — that linkage matters for consistency across systems. Here is the corrected and tightened version with proper references to RFC-170 where concepts are reused.

---

### 3.4: Cellular Groves

Cellular Groves are the primary allocation unit for vegetation types. A grove defines a **locally coherent planting context**: it determines *what* can be planted, *how often*, and *under what constraints*. Groves unify a set of compatible vegetation types and expose parameter ranges that are instantiated by the parent [Forest](#35-cellular-forests).

At a high level:

* **Parameterization** defines the statistical and environmental character of the grove
* **Selection and Placement** determines where and what is actually planted

---

### 3.4.1: Parameterization

Each grove receives a set of parameters. The grove defines ranges; the [Forest](#35-cellular-forests) resolves them via spatially coherent noise.

### 3.4.1.1: Scale

Controls overall tree size.

* Grove defines `[min, max]`
* Forest samples via FBM

```rust
let scale = fbm(world_pos * scale_freq).remap(min_scale, max_scale);
```

Nearby groves will have similar scales.

---

### 3.4.1.2: Density

Controls planting frequency.

* Grove defines `[min, max]`
* Forest samples via FBM

```rust
let density = fbm(world_pos * density_freq).remap(min_density, max_density);
```

Used as the activation threshold for cells.

---

### 3.4.1.3: Distribution

Defines the relative likelihood of selecting variants within the grove.

* Grove defines base weights and ordering
* Forest perturbs weights via noise

Selection is performed via [Bucket Throw](#3421-bucket-throw), allowing smooth spatial shifts in composition.

---

### 3.4.1.4: Offsets

Controls intra-cell placement.

* Grove defines min and max offset ranges
* Forest selects values within that range

As discussed in [Cell Selection and Planting Constraints](#342-selection-and-placement), this follows the same approach as
[RFC-170: Terrain Detail – Position Selection and Validation](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-170-terrain-detail#313-position-selection-and-validation).

```rust
let offset = noise_vec2(seed).remap(offset_min, offset_max);
let position = cell_origin + offset;
```

Offsets may exceed sub-cell bounds, but ownership and stability are always derived from the parent cell.

---

### 3.4.1.5: Elevation Constraints

Defines allowable elevation ranges per variant.

* Base constraints live on tree variants
* Grove may narrow these
* Forest perturbs minimally

These are evaluated exactly as in terrain detail placement in
[RFC-170](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-170-terrain-detail#313-position-selection-and-validation).

Only extreme noise values should substantially affect these constraints.

---

### 3.4.1.6: Steepness Constraints

Similar to elevation, but based on terrain slope.

```rust
let steepness = laplacian(terrain, position);
```

* Grove defines acceptable range
* Forest perturbs slightly

As with elevation, this mirrors the validation step in RFC-170 terrain detail placement.

---

### 3.4.1.7: Noise Amplitude and Frequency

Controls spatial variation.

* Grove defines base amplitude and frequency
* Forest perturbs

```rust
let noise = fbm(world_pos * freq) * amplitude;
```

---

> [!NOTE]
> There is no palette parameterization or perturbation.
> This avoids excessive material variation and draw overhead.
> Visual diversity is instead achieved through world-space shading variation in species shaders.

---

### 3.4.2: Selection and Placement

This stage determines where trees are placed and which variant is selected. It closely mirrors the sampling and validation flow used in
[RFC-170: Terrain Detail](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-170-terrain-detail#313-position-selection-and-validation).

---

### 3.4.2.1: Bucket Throw

The bucket throw algorithm maps variants to contiguous weighted regions.

* Each variant has a **weight** and **position**
* Weights define region size
* Positions define ordering

Selection:

$$
variant = bucket(mean + s([-T, T]))
$$

where:

* $T$ is total ordering span
* $s$ is a centrally-biased noise sample

```rust
let shift = noise(seed).remap(-total_order, total_order);
let idx = wrap(mean + shift, total_order);
let variant = bucket_lookup(idx);
```

This produces:

* locally coherent variation
* gradual composition shifts
* non-uniform but stable distributions

> [!NOTE]
> Canonically, the default mean is at 0.0 in bucket space.

---

### 3.4.2.2: Cell Activation

Cells are selected based on density and noise.

```rust
if fbm(cell_pos * density_freq) > density_threshold {
    continue;
}
```

---

### 3.4.2.3: Position Selection

Their exact point is determined by an offset on the grid, following
[RFC-170: Terrain Detail – Position Selection and Validation](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-170-terrain-detail#313-position-selection-and-validation).

```rust
let p = cell_origin + offset;
```

---

### 3.4.2.4: Constraint Evaluation

Evaluate terrain at the selected point.

```rust
let elevation = terrain_height(p);
let steepness = laplacian(terrain, p);
```

Reject placements that violate constraints.

```rust
if !within(elevation, elevation_range) { continue; }
if !within(steepness, steepness_range) { continue; }
```

This is directly analogous to the validation phase in RFC-170 terrain detail.

---

### 3.4.2.5: Variant Selection

Selection uses a next-fit approach over the ordered distribution:

```rust
selection(elevation, steepness, noise)
```

* start from a noise-derived index
* select nearest valid variant
* preserve distribution while respecting constraints

---

This structure intentionally mirrors terrain detail systems so that:

* placement behavior is predictable
* systems compose cleanly
* spatial artifacts (flicker, migration) are avoided

...while still allowing rich biome-level variation.

### 3.4.3: Well-known Ground Cover Groves

> [!NOTE]
> Assume an empty grove variant exists. 

> [!NOTE]
> For bump outs, the internal cells tend to be a bit larger than other layers.

### 3.4.3.1: Huelgoat Pitch

Huelgoat Pitch is a low, smooth ground-cover grove based on shallow [bump outs](#331-bump-outs). It should read as mossy, soft, and continuous, closely following the underlying terrain with only slight vertical lift.

Good for damp forests, riparian shade, temperate groves, old stone regions, and sparse woodland understory.

```rust
pub enum HuelgoatPitchCell {
    BumpOut(Bucket {
        weight: 1.0,
        item: BumpOut {
            noise: NoiseProfile::LowSmooth,
            height: 0.05..0.10,
            collide: true,
            palette_mix: [
                dark_green..light_green,
            ],
            flecking_mix: [
                Flecking {
                    kind: FleckingKind::Snowfall,
                    strength: Minimal,
                    ..Snowfall::common_flecking(world_size)
                },
            ],
        },
    }),
}

impl CellGrove for HuelgoatPitch {
    type Cell = HuelgoatPitchCell;

    const CELL_SIZE_RANGE: Range<f32> = 50.0..100.0;
    const DENSITY_RANGE: Range<f32> = 0.60..0.80;

    const ELEVATION_RANGE: Range<f32> = 0.0..0.75; // elevation range as fraction of max world height or other normalized elevation
    const STEEPNESS_RANGE: Range<f32> = 0.0..0.45;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.25;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.005..0.020;
}
```

**Construction**

* Use a low, smooth bump out with height around `5cm–10cm`.
* Closely follow the underlying terrain normal and terrain SDF.
* Player collision should use the bumped surface, so the player stands on the pitch rather than visually sinking into it.
* Use moderate to high density: roughly `60%–80%` cell activation.
* Use internal cells around `50m–100m`, preferably chosen as even subdivisions of the parent grove cell.
* At low LOD, collapse the internal cell size to the full grove cell.
* Pair with sparse [Tufts](#332-tufts) for additional volume detail.
* Flecking should be minimal and generally limited to snowfall.


### 3.4.3.2: Flecking Bed

Flecking Bed is a soft, non-colliding ground-cover grove based on moderate [bump outs](#331-bump-outs). It should read as a visual vegetation layer rather than physical terrain, allowing the player to sink through it.

Good for wildflower fields, meadow floors, heath, moss beds, flowering understory, and seasonal ground-cover blooms.

```rust
pub enum FleckingBedCell {
    BumpOut(Bucket {
        weight: 1.0,
        item: BumpOut {
            noise: NoiseProfile::Moderate,
            height: 0.10..0.25,
            collide: false,
            palette_mix: [
                dark_green..light_green,
                yellow_green..dry_green,
            ],
            flecking_mix: [
                Flecking {
                    kind: FleckingKind::Bloom,
                    strength: ModerateToHigh,
                    season_weight: High,
                    longitude_weight: Low,
                    altitude_weight: LowToModerate,
                    ..Default::default()
                },
            ],
        },
    }),
}

impl CellGrove for FleckingBed {
    type Cell = FleckingBedCell;

    const CELL_SIZE_RANGE: Range<f32> = 50.0..100.0;
    const DENSITY_RANGE: Range<f32> = 0.60..0.85;

    // Normalized fraction of max world height.
    const ELEVATION_RANGE: Range<f32> = 0.0..0.80;
    const STEEPNESS_RANGE: Range<f32> = 0.0..0.35;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.25..0.55;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.010..0.040;
}
```

**Construction**

* Use a moderate bump out with height around `10cm–25cm`.
* Apply moderate noise, so the surface reads as uneven vegetation rather than smooth terrain.
* Do not enable collision; the player should visually sink through this layer.
* Use moderate to high density, roughly `60%–85%` cell activation.
* Use internal cells around `50m–100m`, preferably even subdivisions of the parent grove cell.
* At low LOD, collapse the internal cell size to the full grove cell.
* Pair well with any tufting pattern, especially sparse flowering tufts or dry brush.

**Flecking**

* Strong seasonal flecking is encouraged.
* Bloom colors may include white, yellow, pink, purple, orange, or pale blue.
* Flecking strength should vary by season and optionally by longitude or altitude.
* Snow flecking may be layered separately, but bloom flecking is the defining feature.

### 3.4.3.3: Jim's Collage

Jim's Collage is a mixed ground-cover grove that evenly blends [Huelgoat Pitch](#3431-huelgoat-pitch) and [Flecking Bed](#3432-flecking-bed). It provides both a grounded mossy layer and a more decorative, seasonal visual layer.

Good for mixed woodland floors, meadow-forest transitions, garden-like groves, riparian clearings, and areas where ground cover should feel varied without introducing many distinct systems.

```rust
pub enum JimsCollageCell {
    HuelgoatPitch(Bucket {
        weight: 1.0,
        item: HuelgoatPitchCell,
    }),
    FleckingBed(Bucket {
        weight: 1.0,
        item: FleckingBedCell,
    }),
}

impl CellGrove for JimsCollage {
    type Cell = JimsCollageCell;

    const CELL_SIZE_RANGE: Range<f32> = 50.0..100.0;
    const DENSITY_RANGE: Range<f32> = 0.60..0.85;

    const ELEVATION_RANGE: Range<f32> = 0.0..0.80;
    const STEEPNESS_RANGE: Range<f32> = 0.0..0.40;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.15..0.45;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.008..0.035;
}
```

**Construction**

* Use an even split between `HuelgoatPitch` and `FleckingBed`.
* Maintain moderate to high density, roughly `60%–85%`.
* Allow Huelgoat cells to provide low colliding ground softness.
* Allow Flecking Bed cells to provide seasonal bloom and visual softness.
* Use the same internal cell sizing strategy as both parent types: typically `50m–100m`, fit to even subdivisions.
* At low LOD, collapse internal cell size to the full grove cell.


### 3.4.3.4: Floor Scrub

Floor Scrub is a low-density variant of [Jim's Collage](#3433-jims-collage). It uses the same basic split between [Huelgoat Pitch](#3431-huelgoat-pitch) and [Flecking Bed](#3432-flecking-bed), but reduces coverage and uses smaller internal cells for a patchier, more exposed ground layer.

Good for arid regions, sparse woodland, stripped-back understory, chaparral edges, dry groves, and disturbed terrain.

```rust
pub enum FloorScrubCell {
    HuelgoatPitch(Bucket {
        weight: 1.0,
        item: HuelgoatPitchCell,
    }),
    FleckingBed(Bucket {
        weight: 1.0,
        item: FleckingBedCell,
    }),
}

impl CellGrove for FloorScrub {
    type Cell = FloorScrubCell;

    const CELL_SIZE_RANGE: Range<f32> = 15.0..20.0;
    const DENSITY_RANGE: Range<f32> = 0.20..0.45;

    const ELEVATION_RANGE: Range<f32> = 0.0..0.85;
    const STEEPNESS_RANGE: Range<f32> = 0.0..0.45;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.35;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.015..0.050;
}
```

**Construction**

* Use a low-density split between `HuelgoatPitch` and `FleckingBed`.
* Keep density around `20%–45%`.
* Use internal cells around `15m`, fit to even subdivisions of the parent grove cell.
* At low LOD, collapse internal cell size to the full grove cell.
* Prefer weaker bump-out heights and lighter flecking than Jim's Collage.
* Pair well with sparse tufts, dry brush, or exposed terrain detail.

### 3.4.3.5: Grassy Mounds

Grassy Mounds are discrete rounded ground-cover features based on the [Sparse Boulder](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-170-terrain-detail#31-sparse-boulders) placement pattern, but shaded and embedded as vegetation rather than exposed rock.

Good for meadow irregularity, mossy hummocks, pasture texture, wetland edges, and soft terrain breakup.

```rust
pub enum GrassyMoundsCell {
    Mound(Bucket {
        weight: 1.0,
        item: Mound {
            placement: SparseBoulderLike {
                cell_size: 5.0,
                object_scale: 0.60,
                embed_depth: Deep,
            },
            shader: GroundCoverShader,
            palette_mix: [
                dark_green..light_green,
                yellow_green..dry_green,
            ],
        },
    }),
}

impl CellGrove for GrassyMounds {
    type Cell = GrassyMoundsCell;

    const CELL_SIZE_RANGE: Range<f32> = 5.0..6.0;
    const DENSITY_RANGE: Range<f32> = 0.25..0.55;

    const ELEVATION_RANGE: Range<f32> = 0.0..0.85;
    const STEEPNESS_RANGE: Range<f32> = 0.0..0.35;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.15..0.40;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.02..0.06;
}
```

**Construction**

* Use Sparse Boulder-style cell placement.
* Use internal cells around `5m`.
* Set mound size to roughly `60%` of the cell.
* Use rounded SDF forms rather than angular rock forms.
* Embed more deeply than sparse boulders, so the mound reads as terrain growth, not an object sitting on top.
* Use ground-cover or leaf shaders rather than stone shaders.
* Collision may be enabled when mound height materially affects traversal.

**Placement**

```rust
let cell_size = 5.0;
let mound_radius = 0.60 * cell_size;
let position = sparse_boulder_position(cell, seed);

if !contains(parent_cell, position) {
    return None;
}

spawn_mound(
    position,
    radius = mound_radius,
    embed_depth = 0.25 * mound_radius,
    shader = GroundCoverShader,
);
```

### 3.4.3.6: Allbed

Allbed is a broad, mixed ground-cover grove combining flecking and non-flecking bump outs, colliding and non-colliding surface layers, and [Grassy Mounds](#3435-grassy-mounds). It is the most general ground-cover bed and is useful when a region should feel lush, varied, and continuous without committing to a single ground-cover pattern.

Good for rich forest floors, riparian understory, old gardens, meadow edges, fantasy groves, and high-detail mixed biomes.

```rust
pub enum AllbedCell {
    HuelgoatPitch(Bucket {
        weight: 2.0,
        item: HuelgoatPitchCell,
    }),
    FleckingBed(Bucket {
        weight: 2.0,
        item: FleckingBedCell,
    }),
    GrassyMound(Bucket {
        weight: 1.0,
        item: GrassyMoundsCell,
    }),
    LowNonCollidingBumpOut(Bucket {
        weight: 1.0,
        item: BumpOut {
            noise: NoiseProfile::LowSmooth,
            height: 0.05..0.12,
            collide: false,
            palette_mix: [
                dark_green..light_green,
                yellow_green..dry_green,
            ],
            flecking_mix: [],
        },
    }),
    CollidingFleckingBumpOut(Bucket {
        weight: 1.0,
        item: BumpOut {
            noise: NoiseProfile::Moderate,
            height: 0.08..0.18,
            collide: true,
            palette_mix: [
                dark_green..light_green,
                yellow_green..dry_green,
            ],
            flecking_mix: [
                Flecking {
                    kind: FleckingKind::Bloom,
                    strength: LowToModerate,
                    ..Default::default()
                },
                Flecking {
                    kind: FleckingKind::Snowfall,
                    strength: Minimal,
                    ..Snowfall::common_flecking(world_size)
                },
            ],
        },
    }),
}

impl CellGrove for Allbed {
    type Cell = AllbedCell;

    const CELL_SIZE_RANGE: Range<f32> = 15.0..100.0;
    const DENSITY_RANGE: Range<f32> = 0.10..0.90;

    const ELEVATION_RANGE: Range<f32> = 0.0..0.85;
    const STEEPNESS_RANGE: Range<f32> = 0.0..0.40;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.15..0.55;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.008..0.060;
}
```

**Construction**

* Mix multiple bump-out forms rather than enforcing a single bed type.
* Include both colliding and non-colliding bump outs.
* Include both flecking and non-flecking variants.
* Add occasional grassy mounds for rounded volumetric breakup.
* Use moderate to mixed density, roughly `10%–90%`.
* Use larger cells for broad beds and smaller cells where more local variation is desired.
* At low LOD, collapse internal cell size to the full grove cell.

**Behavior**

* Colliding bump outs provide physical surface variation.
* Non-colliding bump outs provide visual softness without affecting traversal.
* Flecking variants provide seasonal blooms or snow.
* Grassy mounds add discrete rounded relief and break up continuous mats.

**Use**

Allbed is best treated as a high-variety default ground-cover grove. It should be used where the designer wants a rich floor texture but does not need a strong specific identity like pitch, scrub, or flowering bed.

### 3.4.4: Well-known Tufts Groves

> [!NOTE]
> Assume an empty grove variant exists. 

### 3.4.5: Well-known Understory Groves

> [!NOTE]
> Assume an empty grove variant exists. 

### 3.4.6: Well-known Lower Canopy Groves

> [!NOTE]
> Assume an empty grove variant exists. 

### 3.4.7: Well-known Upper Canopy Groves

> [!NOTE]
> Assume an empty grove variant exists. 


### 3.4.7: Grove LOD Tricks

### 3.5: Cellular Forests

General name for top-level grove allocation system. Split into several layers of groves. 

### 3.5.1: Parameterization

### 3.5.2: Forest Layers

### 3.5.2.1: Ground Cover Layer

The ground cover layer, unlike other layers is composed of two sublayers to enable a simple model of overlapping ground cover. The sublayers are referred to as Flip and Flop. 

### 3.5.2.2: Tufts Layer

### 3.5.2.3: Understory Layer 

### 3.5.2.4: Tree Layer

### 3.5.3: Well-known Ground Cover Forest Layers

### 3.5.4: Well-known Tuft Forest

### 3.5.4: Well-known

### 3.5.3: Well-known Forests

```rust
pub enum ForestCell {
    Riparian,
    Chaparral,
    Alpine,
    TemperateConiferous,
    Orchard,
    Coniferous,
    Jungle,
    TropicalJungle,

}
```

### 3.5.3.1: Riparian 

### 3.5.3.2: Chaparral

### 3.5.3.3: Alpine

### 3.5.3.4: Temperate Coniferous

### 3.5.3.5: Orchard

### 3.5.3.6: Coniferous

### 

### 3.5.4: Forest LOD Tricks

### 3.6: Elder Trees

## 4: Milestone

