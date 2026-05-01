# 3.1.7.4: Penmarch Torch

This page is subsection **3.1.7.4** of [RFC-183: Chico Vegetation](../../../README.md)


The Penmarch Torch is an upward-projecting variant of the [Vase Tree](../03-vase-tree/README.md#3173-vase-tree). Instead of relaxing toward horizontal near the top, its branches become increasingly vertical, producing a flame-like or torch-like silhouette.

**Shape**

* Narrow to moderate stalk
* Canopy projects upward
* Lower branches open outward
* Upper branches tighten toward vertical
* Overall silhouette resembles a torch or flame

**Stalk**

Use the [Vase Tree](../03-vase-tree/README.md#3173-vase-tree) stalk, usually slightly shorter and more compact.

```rust
let stalk_height = 0.70 * H;
let stalk_radius = 0.03 * H;
```

A [Crook Cylinder](../../01-stick-and-stalk-components/02-crook-cylinder/README.md#3112-crook-cylinder) may be used for stylized urban or chaparral variants.

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

Use the same upper-widening profile as the [Vase Tree](../03-vase-tree/README.md#3173-vase-tree), but generally with a smaller maximum spread:

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

Use compact [Plane Splay](../../02-ball-components/05-plane-splay/README.md#3125-plane-splay), [Tufts](../../02-ball-components/06-tufts/README.md#3126-tufts), or [Noisy Ball](../../02-ball-components/02-noisy-ball/README.md#3122-noisy-ball) depending on desired density.

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

