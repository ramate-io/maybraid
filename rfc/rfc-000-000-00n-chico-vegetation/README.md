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

We list some means of achieving less obvious constructions. 

##### 3.1.6.1: Palm Crown

Anchor a series of radially projecting frond rings in quick vertical succession. Give fronds at higher rings greater vertical bias, hence starting upwards at a slightly greater angle above lower fronds. 

##### 3.1.6.2: Palm Trunk

Do not allocate a stalk. Use canopy ball stick to build up from anchor, biasing roughly vertically. For consistent curve give slight angle bias and low variance. This will cause hysteresis to remain tight. 

Additionally, invert the typical stick dimensions s.t. the radius of the bottom of each segment is slightly less than the top. This will give the palm segment impression. 

##### 3.1.6.3: High-bushes and Shoots

Do not allocate a stalk. Bias a single ring of radial projection roughly vertically. 

##### 3.1.6.4: Jungle Growths

At ball points, in addition to canopy, allocate a larger and darker ball and a tuft.

##### 3.1.6.5: Banyan Trunk

Use large radius and high noise value for stalk. 

##### 3.1.6.6: Banyan Descenders 

Use a high radial projection segment count. Give a strong bias to grow vertically downward in excess of the height of the Banyan on every nth segment. 

#### 3.1.7: Well-known Tree Constructions

We provided the intended tree shapes for Chico vegetation. Note that many of these shapes can be used with a variety of textures and scales to produce the impressions of different species. 

##### 3.1.7.1: Storybook Tree

A general impression of a tree. 

- Use fairly narrow stalk reaching far upwards to about 80% of the total height of the tree including the canopy. Use multiple rings of moderately-dense radial projection to construct the canopy. 
- Start radial projections at roughly 15% of the height of the tree. 
- Bias upper radial projections to be slightly shorter than lower ones. This tail-off like the logarithm approaching the axis from the positive side. (You should use this or something similar mathematically.)
- Place radial projections roughly every 8% of the height of the tree and roughly every 60 degrees, i.e., 6 per ring. 
- Allow total length of radial projections to range up to 60% of the height of the tree. 
- Upper radial projections should be allowed moderate angular variance about a straight horizontal projection, about 15 degrees. 
- Use 3-5 ball-stick segments per projection. Allowing branching between 1 and 3 with mean roughly 2. 
- Allocate [Plane Splay](#3125-plane-splay) for leaves at highest LOD, programming strong preference for only allocating at the outer layers of the canopy. Use a radius of roughly 9% the height of the tree. 

Good for many kinds of forests, particularly deciduous ones. Accommodates all kinds of shaders for both sticks and leaves.

##### 3.1.7.2: Liam's Conifer

A sparse conifer. 

- Use a narrow stalk to reach to about 100% of the total height of the tree. 
- Bias upper radial projections to reduce length linearly w.r.t. to lower ones. 
- Start radial projections roughly 10% of the height of the tree. 
- Place radial projections roughly every 4% of the height of the tree and roughly every 90 degrees, i.e., 4 per ring. 
- Bias radial projections to angle slightly downward, -2 degrees from horizontal. Allow variance within 8 degrees. 
- Use long first ball-stick segment, followed by two very short segments. Allow branching between 1 and 2 with mean closer to 1. 
- Max length of radial projections should be about 5% the height of the tree.
- Allocate [Tufts](#3126-tufts) for canopy at all ball joints. Use 2 to 3 tufts per joint. Allocate with scale proportional to roughly 2% the height of the tree.

Good for drier deciduous forests. Better with lighter shaders for both sticks and leaves. 

##### 3.1.7.3: Vase Tree

A tree that gets wider towards the top, giving a unique head-trained appearance. 

- Take the basic construction from [Storybook Tree](#3171-storybook-tree) but invert the radial projection length s.t. the width increases as you move up the tree. 
- Give the radial projections a vertical bias of 45 degrees from horizontal, decrease the bias as the height increases. 

Good for variety and mystical elements in deciduous forests. Can also be used for bushes and in urban settings. 

##### 3.1.7.4: Penmarch Torch

A tree that projects upwards like a torch. 

- Take the basic construction from [Vase Tree](#3173-vase-tree) but increase the vertical bias as the height increases. 

Good for chaparral, shorter conifers in arid regions, and urbanized settings. 

##### 3.1.7.5: Honu Banyan

A banyan-like tree, spreading its canopy far and wide, and descending some branches down. 

- Use the [Banyan Trunk](#3165-banyan-trunk) construction.
- Start the radial projections at roughly 80% of the total height of the tree. Do not use too many rings, 2 to 3. 
- Use the [Banyan Descenders](#3166-banyan-descenders) construction, typically biasing the canopy towards near horizontal except for every third to fourth segment. 
- Allocate leaf balls throughout the canopy. 

Good for jungle and riparian regions.  

##### 3.1.7.6: Sope's Banyan

A banyan-like tree with a vase-like crown. 

- Begin with the [Honu Banyan](#3175-honu-banyan) construction. 
- Adjust the canopy to project up vertically a la [Penmarch Torch](#3174-penmarch-torch). Radial projections should now begin at something like 40% of the total height. Descenders will still occur every third to fourth segment. 

Good for jungle and riparian regions. Adds a sense of mysticism. Often good as particularly tall, almost as if an [Elder Tree](#36-elder-trees). 

##### 3.1.7.7: Rory's Head-trained 

A stalk with a thin horizontal canopy at the top.

- Use a standard stock. 
- Begin radial projections at 90% or more of total height.
- Bias radial projections to be nearly horizontal. 
- Use moderate branching and segment length values, similar to [Storybook Tree](#3171-storybook-tree).

Good as a tree in arid regions. Good as a bush, e.g., grape vine. Can be used in almost all groves, except for coniferous ones. 

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

