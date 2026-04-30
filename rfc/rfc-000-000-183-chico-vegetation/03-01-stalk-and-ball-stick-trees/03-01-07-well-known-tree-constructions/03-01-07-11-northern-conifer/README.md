# 3.1.7.11: Northern Conifer

This page is subsection **3.1.7.11** of [RFC-183: Chico Vegetation](../../../README.md)


The Northern Conifer is a fuller, colder-climate variant of [Liam's Conifer](../03-01-07-02-liam-s-conifer/README.md#3172-liams-conifer). It preserves the narrow stalk, dense vertical ringing, and short radial projections, but replaces tuft foliage with [Plane Splays](../../03-01-02-ball-components/03-01-02-05-plane-splay/README.md#3125-plane-splay) for broader, denser needle mass.

**Shape**

* Tall, narrow central stalk
* Short radial projections
* Dense layered conifer profile
* Fuller canopy than Liam's Conifer
* Needle-like or clustered planar foliage

**Stalk**

Use the [Liam's Conifer](../03-01-07-02-liam-s-conifer/README.md#3172-liams-conifer) stalk.

```rust
let stalk_height = H;
let stalk_radius = 0.025 * H;
```

**Anchor Rings**

Use the same ring structure as Liam's Conifer.

```rust
let z_min = 0.10 * H;
let z_max = 0.98 * H;
let ring_spacing = 0.04 * H;
let anchors_per_ring = 4;
```

**Projection Length**

Use the same linear upper shortening profile.

```rust
let max_projection_length = 0.05 * H;
let length = max(
    0.20 * max_projection_length,
    max_projection_length * (1.0 - u),
);
```

**Chain Growth**

Use the same sparse radial chain shape, but allow slightly more foliage density at each node.

```rust
BallStickChain {
    segments: 3,
    segment_lengths: [
        0.70 * projection_length,
        0.15 * projection_length,
        0.15 * projection_length,
    ],
    child_count: 1..=2,
    angle_tolerance: radians(8.0),
}
```

Bias projections slightly downward:

```rust
let bias_ray = rotate_down(radial, radians(2.0));
```

**Ball Selection**

Allocate foliage at all ball-stick joints, as in Liam's Conifer, but use [Plane Splay](../../03-01-02-ball-components/03-01-02-05-plane-splay/README.md#3125-plane-splay) instead of [Tufts](../../03-01-02-ball-components/03-01-02-06-tufts/README.md#3126-tufts).

```rust
fn should_allocate_ball(_ctx: BallSelectionContext) -> bool {
    true
}
```

Use small, narrow splays to imply needle clusters.

```rust
let splay_radius = 0.018 * H;
let splay_count = 2..=4;
```

Plane splays should align broadly with the branch direction and slightly downward or outward.

**Materials**

* Stick shader: darker or colder conifer bark
* Leaf shader: dark green, blue-green, or snow-tinted needle material

**Variants**

* Increase splay density for spruce-like forms.
* Use paler shaders for dry or alpine forms.
* Add snow bump-out integration for winter biomes.

