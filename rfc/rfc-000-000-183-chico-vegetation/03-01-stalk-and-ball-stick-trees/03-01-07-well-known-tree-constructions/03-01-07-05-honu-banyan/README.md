# 3.1.7.5: Honu Banyan

This page is subsection **3.1.7.5** of [RFC-183: Chico Vegetation](../../../README.md)


The Honu Banyan is a wide, spreading banyan-like tree with a heavy trunk, broad upper canopy, and occasional downward-descending branches. It is useful for jungle, riparian, and mystical forest regions.

**Shape**

* Thick, irregular central trunk
* Canopy begins high on the tree
* Broad, near-horizontal radial spread
* Periodic descenders fall from canopy branches
* Leaf mass is distributed throughout the upper canopy

**Stalk**

Use the [Banyan Trunk](../../03-01-06-well-known-component-constructions/03-01-06-05-banyan-trunk/README.md#3165-banyan-trunk) construction.

```rust
let stalk_height = 0.80 * H;
let stalk_radius = 0.08 * H;
```

Use a high-noise [Noisy Cylinder](../../03-01-01-stick-and-stalk-components/03-01-01-01-noisy-cylinder/README.md#3111-noisy-cylinder), optionally with [Crook Cylinder](../../03-01-01-stick-and-stalk-components/03-01-01-02-crook-cylinder/README.md#3112-crook-cylinder) variants for secondary trunks.

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

Then apply [Banyan Descenders](../../03-01-06-well-known-component-constructions/03-01-06-06-banyan-descenders/README.md#3166-banyan-descenders) every third to fourth segment.

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

Use [Noisy Ball](../../03-01-02-ball-components/03-01-02-02-noisy-ball/README.md#3122-noisy-ball) or [Plane Splay](../../03-01-02-ball-components/03-01-02-05-plane-splay/README.md#3125-plane-splay) depending on detail level. For jungle variants, combine with [Jungle Growths](../../03-01-06-well-known-component-constructions/03-01-06-04-jungle-growths/README.md#3164-jungle-growths).

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

