# RFC-N: Chico Vegetation

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

#### 3.1.1: Stick and Stalk Components

Stick and stalk components define the structural backbone of trees. They should remain:

* deterministic from seed
* SDF-compatible for mesh and physics reuse
* composable into chains and radial projections

---

##### 3.1.1.1: Noisy Cylinder

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

##### 3.1.1.2: Crook Cylinder

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

#### 3.1.2: Ball Components

Ball components are primarily used for canopy and foliage massing. Unlike stick components, they do not generally need to be collision-supporting, though some may retain SDF backings where useful for reuse or consistency.

These components provide the visual mass of vegetation, while stick components define structure. Together, they enable a wide range of tree and plant forms through simple composition.

---

##### 3.1.2.1: Icosahedron

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

##### 3.1.2.2: Noisy Ball

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

##### 3.1.2.3: Octagonal Plane

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

##### 3.1.2.4: Triangular Plane

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

##### 3.1.2.5: Plane Splay

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

##### 3.1.2.6: Tufts

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

##### 3.1.2.7: Fronds

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

##### 3.1.2.8: Jessen's Icosahedron

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

#### 3.1.3: Ball-stick Anchors

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

#### 3.1.4: Ball-stick Chains

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

#### 3.1.5: Ball Selection

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


#### 3.1.6: Well-known Component Constructions

This section lists reusable component-level constructions. These are not complete tree recipes; they are smaller routines that named tree constructions can compose.

---

##### 3.1.6.1: Palm Crown

A palm crown is built from several radially projecting frond rings placed in quick vertical succession.

Each ring places frond anchors around a central crown point:

```rust
for ring in 0..ring_count {
    let h = ring as f32 * ring_spacing;
    let vertical_bias = base_bias + ring as f32 * bias_step;

    for i in 0..fronds_per_ring {
        let theta = TAU * i as f32 / fronds_per_ring as f32;
        let radial = Vec3::new(theta.cos(), 0.0, theta.sin());

        spawn_frond(
            anchor = crown + Vec3::Y * h,
            direction = normalize(radial + Vec3::Y * vertical_bias),
        );
    }
}
```

Higher rings should start with greater upward bias. Lower rings may droop or project closer to horizontal. This produces the layered crown silhouette common to palms.

---

##### 3.1.6.2: Palm Trunk

A palm trunk should be built without allocating a separate stalk. Instead, use a tight ball-stick chain grown upward from a ground anchor.

The chain should have:

* strong vertical bias
* low angular variance
* consistent slight directional bias for arching palms
* tight hysteresis to preserve a smooth curve

```rust
let config = HysteresisConfig {
    bias_ray: normalize(Vec3::Y + arch_bias),
    bias_strength: high,
    angle_tolerance: low,
    child_count: 1..=1,
    length_range: short..medium,
    radius_range: trunk_radius..trunk_radius,
};
```

Invert the usual tapering rule, so the bottom of each segment is slightly narrower than the top:

```rust
segment.base_radius = r * 0.92;
segment.top_radius = r;
```

Repeated over many segments, this gives the impression of stacked palm trunk bands.

---

##### 3.1.6.3: High-bushes and Shoots

High-bushes and shoots are trunkless radial constructions.

Use a ground or near-ground anchor and emit a single ring of upward-biased radial projections:

```rust
for i in 0..shoot_count {
    let theta = TAU * i as f32 / shoot_count as f32;
    let radial = Vec3::new(theta.cos(), 0.0, theta.sin());

    let dir = normalize(radial * radial_strength + Vec3::Y * vertical_bias);

    grow_chain(anchor, dir);
}
```

This construction is useful for bushes, young trees, tall grass-like woody growth, and vine-like shrubs. Leaf allocation is usually dense near terminal nodes.

---

##### 3.1.6.4: Jungle Growths

Jungle growths are secondary foliage allocations placed at selected ball points.

At a selected canopy node:

```rust
spawn_canopy_ball(node);

spawn_noisy_ball(
    position = node.position,
    radius = node.radius * jungle_growth_scale,
    material = darker_leaf_material,
);

spawn_tuft(
    position = node.position,
    direction = outward_or_upward_bias(node),
);
```

The larger, darker ball gives depth and density. The tuft adds protruding detail and a wet, overgrown silhouette.

This construction is useful for tropical trees, banyans, branch epiphytes, and dense understory vegetation.

---

##### 3.1.6.5: Banyan Trunk

A banyan trunk is a thick, noisy stalk.

Use a large radius and high surface noise:

```rust
let trunk = NoisyCylinder {
    base_radius: large,
    top_radius: large * taper,
    noise_amplitude: high,
    noise_frequency: medium,
};
```

Banyan trunks should appear irregular and rooted rather than smooth. Crook cylinders may be used for secondary trunk forms, but the primary impression should come from radius, noise, and mass.

Joint-concealing balls using bark material may be allocated near major trunk or branch intersections.

---

##### 3.1.6.6: Banyan Descenders

Banyan descenders are downward-growing branch chains emitted from the upper canopy.

Use high radial projection segment count and a chain rule that periodically switches to a strong downward bias:

```rust
fn hysteresis_for(ctx: ChainContext) -> HysteresisConfig {
    if ctx.segment_index % descender_period == 0 {
        HysteresisConfig {
            bias_ray: -Vec3::Y,
            bias_strength: very_high,
            angle_tolerance: low,
            child_count: 1..=1,
            length_range: long..very_long,
            radius_range: thin..medium,
        }
    } else {
        ordinary_canopy_config(ctx)
    }
}
```

Descenders should often extend below the canopy height and may approach or intersect the ground. When they reach the ground, they can be thickened or treated as secondary stalks.

Use sparse foliage on descenders themselves; most foliage should remain attached to the upper canopy.

##### 3.1.6.7: Fruiting Bodies

Fruiting bodies are optional canopy details: small, brightly colored ellipsoidal volumes placed on or near the radius of canopy components. They are useful for fruit trees, jungle growths, magical trees, and seasonal variation.

A fruiting body should usually be attached to a selected canopy node or canopy ball, not to the trunk. The placement rule samples points near the canopy surface:

```rust
let dir = sample_sphere(seed, i);
let p = canopy_center + dir * canopy_radius;
```

Then apply a small inward or outward offset, so fruit appears embedded in the foliage rather than floating:

```rust
let p = p - dir * embed_depth;
```

The fruit itself can be a scaled sphere or ellipsoid:

```rust
pub struct FruitingBodyConfig {
    pub count: usize,
    pub radius: Vec3,
    pub color: Color,
    pub embed_depth: f32,
    pub surface_bias: f32,
}
```

For SDF construction:

```rust
fn ellipsoid_sdf(p: Vec3, r: Vec3) -> f32 {
    (p / r).length() - 1.0
}
```

For mesh construction, use a low-subdivision UV sphere or icosphere and apply non-uniform scale:

```rust
spawn_ellipsoid(
    position = p,
    scale = config.radius,
    material = fruit_material,
);
```

Fruit allocation should be sparse and deterministic:

```rust
for i in 0..config.count {
    if noise(seed, i) < fruit_probability {
        continue;
    }

    let dir = sample_canopy_surface(seed, i);
    let p = canopy_center + dir * canopy_radius;
    spawn_fruit(p);
}
```

A more advanced form may include seasonality. A time or season parameter can modulate both visibility and size:

```rust
let maturity = seasonal_curve(time, fruit_phase, fruit_duration);

let scale = base_scale * maturity;
let visible = maturity > visibility_threshold;
```

This allows fruit to emerge, grow, ripen, and disappear over time without changing the underlying tree construction. Color may also vary with maturity, for example green to yellow to red.

#### 3.1.7: Well-known Tree Constructions

We provided the intended tree shapes for Chico vegetation. Note that many of these shapes can be used with a variety of textures and scales to produce the impressions of different species. 

##### 3.1.7.1: Storybook Tree

The Storybook Tree is the default broadleaf silhouette: a narrow central stalk with a rounded canopy assembled from moderately dense radial ball-stick projections. It is useful for deciduous forests, orchards, parks, and general-purpose background trees.

**Shape**

* Tall, fairly narrow stalk
* Rounded canopy beginning low on the upper trunk
* Lower branches longer than upper branches
* Moderate branching and soft radial spread

**Stalk**

Let $H$ be total tree height including canopy.

```rust
let stalk_height = 0.80 * H;
let stalk_radius = 0.035 * H;
```

Use a [Noisy Cylinder](#3111-noisy-cylinder) for the stalk. Noise should be visible but not dominant.

```rust
NoisyCylinder {
    base_radius: stalk_radius,
    top_radius: stalk_radius * 0.55,
    noise_amplitude: 0.08 * stalk_radius,
    noise_frequency: medium,
}
```

**Anchor Rings**

Radial projections begin at roughly $15%$ of total height and continue toward the top of the stalk.

```rust
let z_min = 0.15 * H;
let z_max = stalk_height;
let ring_spacing = 0.08 * H;
let anchors_per_ring = 6;
```

Each ring places anchors roughly every $60^\circ$:

```rust
for z in steps(z_min, z_max, ring_spacing) {
    for i in 0..anchors_per_ring {
        let theta = TAU * i as f32 / anchors_per_ring as f32;
        let radial = Vec3::new(theta.cos(), 0.0, theta.sin());

        anchor(position = stalk_centroid(z), initial_ray = radial);
    }
}
```

Anchors should originate near the stalk radial centroid to avoid detached-looking branches.

**Projection Length**

Lower branches should be longer than upper branches. Let:

$$
u = \frac{z - z_{\min}}{z_{\max} - z_{\min}}
$$

Use a logarithmic or similar falloff:

$$
\ell(u) = \ell_{\max}(1 - \log(1 + \alpha u) / \log(1 + \alpha))
$$

with:

```rust
let max_projection_length = 0.60 * H;
let alpha = 4.0;
```

This produces a round canopy that gently approaches the vertical axis near the top.

**Chain Growth**

Each radial projection grows as a short ball-stick chain:

```rust
BallStickChain {
    segments: 3..=5,
    child_count: 1..=3, // mean near 2
    angle_tolerance: radians(15.0),
    bias_ray: radial,
    bias_strength: moderate,
}
```

The bias should be mostly horizontal, with slight upward variance for higher branches and slight downward variance for lower branches if a fuller canopy is desired.

**Ball Selection**

At high detail, allocate [Plane Splay](#3125-plane-splay) primarily on the outer canopy:

```rust
fn should_allocate_ball(ctx: BallSelectionContext) -> bool {
    ctx.is_terminal || ctx.distance_from_anchor > 0.65 * ctx.max_projection_length
}
```

Use a splay radius of roughly:

```rust
let leaf_radius = 0.09 * H;
```

Interior nodes should usually avoid foliage allocation unless a dense canopy is desired.

**Materials**

* Stick shader: bark or stylized trunk material
* Leaf shader: broadleaf, deciduous, orchard, or fantasy foliage
* Optional [Fruiting Bodies](#3167-fruiting-bodies) for orchard or magical variants

**Variants**

* Denser rings and larger leaf splays produce orchard trees.
* Smaller projection length and darker materials produce compact forest trees.
* Higher angular variance and [Crook Cylinder](#3112-crook-cylinder) segments produce older or more whimsical silhouettes.

##### 3.1.7.2: Liam's Conifer

Liam's Conifer is a sparse, dry conifer silhouette: a narrow vertical stalk with many short, lightly downward-biased radial projections. It is useful for drier conifer stands, semi-arid forests, and lighter woodland edges.

**Shape**

* Tall, narrow central stalk
* Short radial projections
* Sparse branching
* Tuft-based canopy at most ball-stick joints
* Slight downward branch bias

**Stalk**

Let $H$ be total tree height.

```rust
let stalk_height = H;
let stalk_radius = 0.025 * H;
```

Use a [Noisy Cylinder](#3111-noisy-cylinder) with modest taper and low to medium noise.

```rust
NoisyCylinder {
    base_radius: stalk_radius,
    top_radius: stalk_radius * 0.35,
    noise_amplitude: 0.06 * stalk_radius,
    noise_frequency: medium,
}
```

**Anchor Rings**

Radial projections begin at roughly $10%$ of height and continue nearly to the top.

```rust
let z_min = 0.10 * H;
let z_max = 0.98 * H;
let ring_spacing = 0.04 * H;
let anchors_per_ring = 4;
```

Each ring places anchors roughly every $90^\circ$:

```rust
for z in steps(z_min, z_max, ring_spacing) {
    for i in 0..anchors_per_ring {
        let theta = TAU * i as f32 / anchors_per_ring as f32;
        let radial = Vec3::new(theta.cos(), 0.0, theta.sin());

        anchor(position = stalk_centroid(z), initial_ray = radial);
    }
}
```

**Projection Length**

Upper projections shrink linearly relative to lower projections.

Let:

$$
u = \frac{z - z_{\min}}{z_{\max} - z_{\min}}
$$

Then:

$$
\ell(u) = \ell_{\max}(1 - u)
$$

with:

```rust
let max_projection_length = 0.05 * H;
```

Optionally clamp to preserve a small top silhouette:

```rust
let length = max(0.20 * max_projection_length, max_projection_length * (1.0 - u));
```

**Chain Growth**

Each projection uses a long first segment followed by two short segments.

```rust
BallStickChain {
    segments: 3,
    segment_lengths: [
        0.70 * projection_length,
        0.15 * projection_length,
        0.15 * projection_length,
    ],
    child_count: 1..=2, // mean close to 1
    angle_tolerance: radians(8.0),
}
```

Bias the projection slightly downward:

```rust
let downward_bias = rotate_down(radial, radians(2.0));
```

Use tight hysteresis, so branches remain sparse and readable.

```rust
HysteresisConfig {
    bias_ray: downward_bias,
    bias_strength: high,
    angle_tolerance: radians(8.0),
    child_count: 1..=2,
}
```

**Ball Selection**

Allocate [Tufts](#3126-tufts) at all ball-stick joints.

```rust
fn should_allocate_ball(_ctx: BallSelectionContext) -> bool {
    true
}
```

Use two to three tufts per joint:

```rust
let tuft_count = 2..=3;
let tuft_scale = 0.02 * H;
```

Tufts should follow the branch direction with mild upward spread to avoid a purely flat silhouette.

**Materials**

* Stick shader: lighter bark or dry trunk tones
* Leaf shader: pale green, dusty green, or dry conifer tones

**Variants**

* Increasing ring density produces fuller conifers.
* Replacing tufts with [Plane Splay](#3125-plane-splay) produces a northern conifer variant.
* Increasing downward bias gives a drooping alpine silhouette.

##### 3.1.7.3: Vase Tree

The Vase Tree is a broad, upward-opening tree form. It starts from the [Storybook Tree](#3171-storybook-tree) construction but inverts the canopy profile, so radial projections grow wider toward the top. This gives a head-trained, vase-like silhouette useful for ornamental trees, mystical forests, bushes, and urban plantings.

**Shape**

* Narrow to moderate stalk
* Canopy opens upward and outward
* Upper branches are longer than lower branches
* Lower branches are strongly upward-biased
* Bias relaxes closer to horizontal near the top

**Stalk**

Use the same stalk construction as [Storybook Tree](#3171-storybook-tree), optionally shortened slightly for bush or ornamental variants.

```rust
let stalk_height = 0.75 * H;
let stalk_radius = 0.035 * H;
```

Use a [Noisy Cylinder](#3111-noisy-cylinder) or [Crook Cylinder](#3112-crook-cylinder) depending on desired stylization.

**Anchor Rings**

Use Storybook-style radial rings, but favor upper canopy density.

```rust
let z_min = 0.20 * H;
let z_max = stalk_height;
let ring_spacing = 0.08 * H;
let anchors_per_ring = 6;
```

Anchors should originate near the stalk radial centroid.

**Projection Length**

Invert the Storybook falloff so branch length increases with height.

Let:

$$
u = \frac{z - z_{\min}}{z_{\max} - z_{\min}}
$$

Use a softened increasing curve:

$$
\ell(u) = \ell_{\min} + (\ell_{\max} - \ell_{\min}) \frac{\log(1 + \alpha u)}{\log(1 + \alpha)}
$$

with:

```rust
let min_projection_length = 0.15 * H;
let max_projection_length = 0.60 * H;
let alpha = 4.0;
```

This creates short lower branches and broader upper spread.

**Chain Growth**

Use a Storybook-like chain with moderate branching.

```rust
BallStickChain {
    segments: 3..=5,
    child_count: 1..=3,
    angle_tolerance: radians(15.0),
}
```

Bias should start strongly upward and approach horizontal as height increases.

```rust
let vertical_angle = mix(
    radians(45.0),
    radians(5.0),
    u,
);

let bias_ray = rotate_up(radial, vertical_angle);
```

This opens the canopy like a vase: lower branches climb sharply, while upper branches spread outward.

**Ball Selection**

Allocate foliage mostly on upper and outer nodes.

```rust
fn should_allocate_ball(ctx: BallSelectionContext) -> bool {
    ctx.is_terminal
        || ctx.height_fraction > 0.60
        || ctx.distance_from_anchor > 0.60 * ctx.max_projection_length
}
```

Use [Plane Splay](#3125-plane-splay) at high detail and [Noisy Ball](#3122-noisy-ball) or icospheres at lower detail.

```rust
let leaf_radius = 0.08 * H;
```

**Materials**

* Stick shader: deciduous bark, ornamental bark, or stylized dark bark
* Leaf shader: broadleaf, flowering, magical, or urban ornamental foliage
* Optional [Fruiting Bodies](#3167-fruiting-bodies) for orchard-like variants

**Variants**

* Shorter stalk and denser upper branches produce a bush form.
* Higher upward bias produces a flame-like ornamental tree.
* Crook cylinders add a trained or sculpted garden appearance.


##### 3.1.7.4: Penmarch Torch

The Penmarch Torch is an upward-projecting variant of the [Vase Tree](#3173-vase-tree). Instead of relaxing toward horizontal near the top, its branches become increasingly vertical, producing a flame-like or torch-like silhouette.

**Shape**

* Narrow to moderate stalk
* Canopy projects upward
* Lower branches open outward
* Upper branches tighten toward vertical
* Overall silhouette resembles a torch or flame

**Stalk**

Use the [Vase Tree](#3173-vase-tree) stalk, usually slightly shorter and more compact.

```rust
let stalk_height = 0.70 * H;
let stalk_radius = 0.03 * H;
```

A [Crook Cylinder](#3112-crook-cylinder) may be used for stylized urban or chaparral variants.

**Anchor Rings**

Use Vase-style radial rings:

```rust
let z_min = 0.20 * H;
let z_max = stalk_height;
let ring_spacing = 0.08 * H;
let anchors_per_ring = 6;
```

Anchors should originate near the stalk radial centroid.

**Projection Length**

Use the same upper-widening profile as the [Vase Tree](#3173-vase-tree), but generally with a smaller maximum spread:

```rust
let min_projection_length = 0.10 * H;
let max_projection_length = 0.45 * H;
```

This preserves the torch shape without becoming too broad.

**Chain Growth**

Use moderate branching, similar to Vase Tree:

```rust
BallStickChain {
    segments: 3..=5,
    child_count: 1..=3,
    angle_tolerance: radians(12.0),
}
```

The key difference is the vertical bias. Let:

$$
u = \frac{z - z_{\min}}{z_{\max} - z_{\min}}
$$

Instead of decreasing vertical bias with height, increase it:

```rust
let vertical_angle = mix(
    radians(25.0),
    radians(70.0),
    u,
);

let bias_ray = rotate_up(radial, vertical_angle);
```

Lower branches still flare outward, while upper branches climb sharply.

**Ball Selection**

Allocate foliage mainly along upper and terminal nodes to preserve the torch silhouette.

```rust
fn should_allocate_ball(ctx: BallSelectionContext) -> bool {
    ctx.is_terminal
        || ctx.height_fraction > 0.55
        || ctx.distance_from_anchor > 0.70 * ctx.max_projection_length
}
```

Use compact [Plane Splay](#3125-plane-splay), [Tufts](#3126-tufts), or [Noisy Ball](#3122-noisy-ball) depending on desired density.

```rust
let leaf_radius = 0.06 * H;
```

**Materials**

* Stick shader: dry bark, pale bark, or ornamental trunk material
* Leaf shader: chaparral green, dusty green, conifer-like, or urban ornamental foliage

**Variants**

* Reduce height and increase density for chaparral shrubs.
* Use tufts instead of broadleaf splays for short dry conifers.
* Increase vertical bias and use saturated leaves for stylized urban trees.

##### 3.1.7.5: Honu Banyan

The Honu Banyan is a wide, spreading banyan-like tree with a heavy trunk, broad upper canopy, and occasional downward-descending branches. It is useful for jungle, riparian, and mystical forest regions.

**Shape**

* Thick, irregular central trunk
* Canopy begins high on the tree
* Broad, near-horizontal radial spread
* Periodic descenders fall from canopy branches
* Leaf mass is distributed throughout the upper canopy

**Stalk**

Use the [Banyan Trunk](#3165-banyan-trunk) construction.

```rust
let stalk_height = 0.80 * H;
let stalk_radius = 0.08 * H;
```

Use a high-noise [Noisy Cylinder](#3111-noisy-cylinder), optionally with [Crook Cylinder](#3112-crook-cylinder) variants for secondary trunks.

```rust
NoisyCylinder {
    base_radius: stalk_radius,
    top_radius: stalk_radius * 0.75,
    noise_amplitude: 0.18 * stalk_radius,
    noise_frequency: medium,
}
```

**Anchor Rings**

Radial projections begin high, around $80%$ of total height.

```rust
let z_min = 0.80 * H;
let z_max = 0.95 * H;
let ring_spacing = 0.06 * H;
let anchors_per_ring = 6..=8;
```

Use only two to three rings.

```rust
let ring_count = 2..=3;
```

Anchors should originate near the stalk radial centroid to keep major limbs visually embedded in the trunk mass.

**Projection Length**

The canopy should spread far and wide.

```rust
let max_projection_length = 0.75 * H;
let min_projection_length = 0.35 * H;
```

Projection length can remain mostly stable across the few upper rings, with slight shortening near the highest ring.

```rust
let length = mix(max_projection_length, min_projection_length, u * 0.35);
```

**Chain Growth**

Use long, mostly horizontal ball-stick chains.

```rust
BallStickChain {
    segments: 5..=8,
    child_count: 1..=3,
    angle_tolerance: radians(12.0),
}
```

The ordinary canopy bias should be nearly horizontal:

```rust
let canopy_bias = normalize(radial + Vec3::Y * 0.05);
```

Then apply [Banyan Descenders](#3166-banyan-descenders) every third to fourth segment.

```rust
fn hysteresis_for(ctx: ChainContext) -> HysteresisConfig {
    if ctx.segment_index % 4 == 0 {
        descender_config()
    } else {
        HysteresisConfig {
            bias_ray: canopy_bias,
            bias_strength: high,
            angle_tolerance: radians(12.0),
            child_count: 1..=3,
            length_range: medium..long,
            radius_range: medium..thin,
        }
    }
}
```

Descenders should bias strongly downward and may extend below the canopy height.

```rust
fn descender_config() -> HysteresisConfig {
    HysteresisConfig {
        bias_ray: -Vec3::Y,
        bias_strength: very_high,
        angle_tolerance: radians(6.0),
        child_count: 1..=1,
        length_range: long..very_long,
        radius_range: thin..medium,
    }
}
```

**Ball Selection**

Allocate leaf balls broadly throughout the canopy, not only at terminal nodes.

```rust
fn should_allocate_ball(ctx: BallSelectionContext) -> bool {
    ctx.height_fraction > 0.70
        || ctx.is_terminal
        || ctx.branch_order > 1
}
```

Use [Noisy Ball](#3122-noisy-ball) or [Plane Splay](#3125-plane-splay) depending on detail level. For jungle variants, combine with [Jungle Growths](#3164-jungle-growths).

```rust
let leaf_radius = 0.10 * H;
```

Descenders should usually receive sparse foliage or none, unless the goal is a very dense jungle silhouette.

**Materials**

* Stick shader: dark, high-variation bark
* Leaf shader: dense tropical green, riparian green, or darker jungle foliage
* Optional jungle growth layer for wet or overgrown variants

**Variants**

* Increase descender frequency for older banyans.
* Allow descenders to become secondary trunks when they reach the ground.
* Add darker interior balls and tufts for dense jungle banyans.

##### 3.1.7.6: Sope's Banyan

![Sope's Banyan](./assets/sopes-banyan.png)

Sope's Banyan is a banyan variant with a tall, vase-like crown. It begins from the [Honu Banyan](#3175-honu-banyan) construction, but moves the canopy lower and biases branch growth upward, closer to the [Penmarch Torch](#3174-penmarch-torch). The result is a mystical, vertically rising banyan form suited to jungle, riparian, and elder-tree contexts.

**Shape**

* Thick banyan trunk
* Canopy begins around mid-height
* Wide but upward-projecting branch structure
* Periodic downward descenders
* Tall, vase-like silhouette

**Stalk**

Use the [Banyan Trunk](#3165-banyan-trunk) construction.

```rust
let stalk_height = 0.75 * H;
let stalk_radius = 0.075 * H;
```

Use high-noise bark and strong trunk mass, as in [Honu Banyan](#3175-honu-banyan).

**Anchor Rings**

Radial projections begin much lower than Honu Banyan, around $40%$ of total height.

```rust
let z_min = 0.40 * H;
let z_max = 0.90 * H;
let ring_spacing = 0.08 * H;
let anchors_per_ring = 6..=8;
```

Use several rings to build the rising crown:

```rust
let ring_count = 5..=7;
```

Anchors should originate near the stalk radial centroid, so the large upward limbs read as emerging from the trunk mass.

**Projection Length**

Use a vase-like widening profile. Let:

$$
u = \frac{z - z_{\min}}{z_{\max} - z_{\min}}
$$

Projection length increases with height:

```rust
let min_projection_length = 0.25 * H;
let max_projection_length = 0.70 * H;
let length = mix(min_projection_length, max_projection_length, sqrt(u));
```

This keeps the lower crown compact while allowing the upper canopy to spread dramatically.

**Chain Growth**

Use long banyan-like chains with upward torch bias.

```rust
BallStickChain {
    segments: 5..=8,
    child_count: 1..=3,
    angle_tolerance: radians(12.0),
}
```

Bias rises with height, as in [Penmarch Torch](#3174-penmarch-torch):

```rust
let vertical_angle = mix(
    radians(25.0),
    radians(70.0),
    u,
);

let canopy_bias = rotate_up(radial, vertical_angle);
```

Descenders still occur every third to fourth segment:

```rust
fn hysteresis_for(ctx: ChainContext) -> HysteresisConfig {
    if ctx.segment_index % 4 == 0 {
        descender_config()
    } else {
        HysteresisConfig {
            bias_ray: canopy_bias,
            bias_strength: high,
            angle_tolerance: radians(12.0),
            child_count: 1..=3,
            length_range: medium..long,
            radius_range: medium..thin,
        }
    }
}
```

Descenders should remain strongly downward-biased, but may be slightly less frequent than in Honu Banyan if the crown should read more vertical than tangled.

**Ball Selection**

Allocate foliage broadly throughout the rising crown.

```rust
fn should_allocate_ball(ctx: BallSelectionContext) -> bool {
    ctx.height_fraction > 0.45
        || ctx.is_terminal
        || ctx.branch_order > 1
}
```

Use [Noisy Ball](#3122-noisy-ball), [Plane Splay](#3125-plane-splay), and optional [Jungle Growths](#3164-jungle-growths) for dense variants.

```rust
let leaf_radius = 0.09 * H;
```

Descenders should receive sparse foliage, except where a denser mystical canopy is desired.

**Materials**

* Stick shader: dark banyan bark, wet bark, or high-contrast fantasy bark
* Leaf shader: dense jungle green, deep riparian green, or saturated mystical foliage
* Optional darker inner canopy balls for depth

**Variants**

* Increase total height and reduce descender frequency for an elder-tree silhouette.
* Add [Fruiting Bodies](#3167-fruiting-bodies) for mystical or ancient variants.
* Use [Crook Cylinder](#3112-crook-cylinder) on major limbs for a more twisted appearance.


##### 3.1.7.7: Rory's Head-trained

Rory's Head-trained is a top-heavy, trained tree form: a simple stalk with a thin, mostly horizontal canopy near the top. It is useful for arid trees, grape-vine-like bushes, ornamental plantings, and non-coniferous groves.

**Shape**

* Standard vertical stalk
* Single high canopy ring
* Thin horizontal spread
* Moderate branching
* Minimal lower foliage

**Stalk**

Use a standard [Noisy Cylinder](#3111-noisy-cylinder).

```rust
let stalk_height = 0.90 * H;
let stalk_radius = 0.025 * H;
```

Keep the trunk relatively clean and readable.

```rust
NoisyCylinder {
    base_radius: stalk_radius,
    top_radius: stalk_radius * 0.55,
    noise_amplitude: 0.06 * stalk_radius,
    noise_frequency: medium,
}
```

**Anchor Ring**

Begin radial projections at $90%$ or more of total height. Use one ring layer.

```rust
let z = 0.90 * H;
let anchors_per_ring = 6..=8;
```

Anchors should originate near the stalk radial centroid.

```rust
for i in 0..anchors_per_ring {
    let theta = TAU * i as f32 / anchors_per_ring as f32;
    let radial = Vec3::new(theta.cos(), 0.0, theta.sin());

    anchor(
        position = stalk_centroid(z),
        initial_ray = radial,
        bias_ray = radial,
    );
}
```

**Projection Length**

Use moderate projection lengths similar to [Storybook Tree](#3171-storybook-tree), but keep the canopy flatter.

```rust
let projection_length = 0.35 * H..0.55 * H;
```

For bush or grape-vine variants, reduce height and keep spread relatively wide:

```rust
let projection_length = 0.60 * H;
```

**Chain Growth**

Use moderate branching and segment length values.

```rust
BallStickChain {
    segments: 3..=5,
    child_count: 1..=3,
    angle_tolerance: radians(10.0),
}
```

Bias projections nearly horizontal:

```rust
let bias_ray = normalize(radial + Vec3::Y * 0.02);
```

Keep vertical variance small to maintain the trained canopy plane.

```rust
HysteresisConfig {
    bias_ray,
    bias_strength: high,
    angle_tolerance: radians(10.0),
    child_count: 1..=3,
}
```

**Ball Selection**

Allocate canopy primarily on terminal and outer nodes, preserving the thin horizontal profile.

```rust
fn should_allocate_ball(ctx: BallSelectionContext) -> bool {
    ctx.is_terminal
        || ctx.distance_from_anchor > 0.65 * ctx.max_projection_length
}
```

Use compact [Plane Splay](#3125-plane-splay), [Noisy Ball](#3122-noisy-ball), or [Tufts](#3126-tufts) depending on species.

```rust
let leaf_radius = 0.06 * H;
```

Avoid dense interior allocation, since this tree should read as a trained crown rather than a full rounded canopy.

**Materials**

* Stick shader: dry bark, vineyard wood, or ornamental bark
* Leaf shader: broadleaf, vine, arid green, or cultivated foliage
* Optional [Fruiting Bodies](#3167-fruiting-bodies) for orchard or grape-like variants

**Variants**

* Shorten stalk and increase spread for bush or grape-vine forms.
* Add fruiting bodies for cultivated groves.
* Use sparse tufts for arid scrub variants.


##### 3.1.7.8: Waialea Palm 

- Use the [Palm Trunk](#3162-palm-trunk) construction. Arch it gently. 
- Use the [Palm Crown](#3161-palm-crown) construction with two to three layers. 

##### 3.1.7.9: Date Palm

- Use the [Pam Trunk](#3162-palm-trunk) construction without arching. 
- Use the [Palm Crown](#3161-palm-crown) construction with 6 to 10 layers. 

##### 3.1.7.9: Palm Bush

- Use the [Palm Crown](#3161-palm-crown) construction with 6 to 10 layers. 
- Do not add a trunk. 

##### 3.1.7.10: Northern Conifer

- Use [Liam's Conifer](#3172-liams-conifer) construction, but allocate [Plane Splays](#3125-plane-splay) for the leaf balls of the canopy.

##### 3.1.7.11: Common High Bush

- Use the [High-bush and Shoots](#3163-high-bushes-and-shoots) construction.
- Send up 7 to 10 vertically-biased radial projections. 

Can be used as a bush or small tree in non-arid biomes. Takes any shader combination well.

##### 3.1.7.12: Jungle Storybook Tree

- Add [Jungle Growths](#3164-jungle-growths) to the [Storybook Tree](#3171-storybook-tree)

##### 3.1.7.13: Braid Oak

- Start with something similar to the [Storybook Tree](#3171-storybook-tree).
- Make biasing vary along the height, beginning with downward bias at low Z-value and upward bias at higher Z-values.
- Use [Crook Cylinder](#3112-crook-cylinder) for segments. 

##### 3.1.7.14: Friend's Conifer

- Start with the [Northern Conifer](#31710-northern-conifer) construction. 
- Make radial projection segment length vary with log, keeping an almost consistent length for much of the length and rounding towards the top. 

##### 3.1.7.15: Temperate Conifer

- Use the [Friend's Conifer](#31714-friends-conifer) construction, but replace the leaf canopy balls with [Fronds](#3127-fronds).

Good for strange bushes when scaled down. Otherwise, works well in semi-arid tropical regions where foliage is somewhat sparse. 

##### 3.1.7.15: Simpleman's Hedge

- No need for any ball stick here just use [Plane Splay](#3125-plane-splay) and place on the ground.

##### 3.1.7.15: Simpleman's Tuft

- Just the basic [Tuft](#3126-tufts).

#### 3.1.8: Tree LOD Tricks

#### 3.1.9: Stick Shading

#### 3.1.10: Leaf Shading

### 3.2: L-system Trees

### 3.3: Ground Cover

### 3.4: Cellular Groves

General name for vegetation type allocation system. Unify exclusive types you want to plant in a grove. Groves are the level at which planting constraints are painted in.

#### 3.4.1: Parameterization

#### 3.4.2: Cell Selection and Planting Constraints

#### 3.4.3: Well-known Ground Cover Groves

#### 3.4.4: Well-known Tufts Groves

#### 3.4.5: Well-known Bush Groves

#### 3.4.6: Well-known Tree Groves

#### 3.4.7: Grove LOD Tricks

### 3.5: Cellular Forests

General name for top-level grove allocation system. Split into several layers of groves. 

### 3.5.1: Parameterization

#### 3.5.2: Forest Layers

##### 3.5.2.1: Ground Cover Layer

##### 3.5.2.2: Tufts Layer

##### 3.5.2.3: Bush Layer

##### 3.5.2.4: Tree Layer

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

#### 3.5.3.1: Riparian 

#### 3.5.3.2: Chaparral

#### 3.5.3.3: Alpine

#### 3.5.3.4: Temperate Coniferous

#### 3.5.3.5: Orchard

#### 3.5.3.6: Coniferous

#### 

### 3.5.4: Forest LOD Tricks

### 3.6: Elder Trees

## 4: Milestone

