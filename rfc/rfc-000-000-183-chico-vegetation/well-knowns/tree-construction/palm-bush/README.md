# Palm Bush

This file is part of [RFC-183: Chico Vegetation](../../../README.md).

**Construction type:** tree construction (see section 3.1.7 in the main RFC).


The Palm Bush is a trunkless palm form: a dense, ground-anchored cluster of fronds radiating outward. It is useful for understory tropical vegetation, coastal growth, decorative landscaping, and dense jungle edges.

**Shape**

* No visible trunk
* Dense radial frond cluster from ground
* Multi-layered crown
* Lower fronds droop outward
* Upper fronds rise slightly

**Anchor**

Place the crown directly at or slightly above ground level.

```rust
let crown = ground_position + Vec3::Y * (0.02 * H);
```

**Crown Construction**

Use the [Palm Crown](../../component-construction/palm-crown/README.md) construction with more layers to achieve density.

```rust
let ring_count = 6..=10;
let fronds_per_ring = 10..=16;
let ring_spacing = 0.01 * H;
```

```rust
for ring in 0..ring_count {
    let u = ring as f32 / (ring_count - 1) as f32;

    let vertical_bias = mix(
        -0.20, // lower rings droop strongly
        0.35,  // upper rings slightly upward
        u,
    );

    for i in 0..fronds_per_ring {
        let theta = TAU * i as f32 / fronds_per_ring as f32;
        let radial = Vec3::new(theta.cos(), 0.0, theta.sin());

        spawn_frond(
            anchor = crown + Vec3::Y * ring as f32 * ring_spacing,
            direction = normalize(radial + Vec3::Y * vertical_bias),
        );
    }
}
```

**Fronds**

Use [Fronds](../../../README.md#3127-fronds) with moderate length and strong droop for lower layers.

```rust
FrondConfig {
    segments: 8..=14,
    length: 0.25 * H..0.40 * H,
    width: 0.05 * H,
    droop: medium_to_high,
    twist: mild,
    leaflet_count: 12..=20,
}
```

Lower fronds should sweep outward and downward, forming a skirt. Upper fronds should provide some upward lift to avoid a flattened silhouette.

**Ball Selection**

Not applicable. Fronds are directly allocated from the crown anchor. Optionally, use a small central [Tuft](../../../README.md#3126-tufts) to conceal the origin point.

```rust
spawn_tuft(
    position = crown,
    direction = Vec3::Y,
    scale = 0.04 * H,
);
```

**Materials**

* Leaf shader: tropical greens, dusty greens, or dry palm tones
* Optional variation in leaf color across rings for natural variation

**Variants**

* Reduce height and increase frond count for dense ground cover.
* Increase droop for desert or coastal scrub variants.
* Add [Fruiting Bodies](../../component-construction/fruiting-bodies/README.md) near the base for decorative or exotic forms.
