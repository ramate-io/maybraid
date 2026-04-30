# 3.1.4: Ball-stick Chains

This page is subsection **3.1.4** of [RFC-183: Chico Vegetation](../../README.md)


Ball-stick chains describe the branching skeleton grown from an anchor. The existing [`BallStickBuilder`](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/comproc/src/complex/chain/ball_stick/builder.rs) already captures the core idea: extend from parent nodes, choose child count, sample direction, assign radii, and emit connected nodes.

At a high level, a chain is a directed graph:

```rust
pub struct BallStickChain {
    pub nodes: Vec<BallStickNode>,
    pub segments: Vec<BallStickSegment>,
}
```

Each segment becomes a stick component, usually a [noisy cylinder](../03-01-01-stick-and-stalk-components/03-01-01-01-noisy-cylinder/README.md#3111-noisy-cylinder) or [crook cylinder](../03-01-01-stick-and-stalk-components/03-01-01-02-crook-cylinder/README.md#3112-crook-cylinder). Each node becomes a possible site for foliage, descenders, joint-concealing balls, or additional child chains.

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

