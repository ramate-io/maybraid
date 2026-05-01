# 3.1.7.9: Date Palm

This page is subsection **3.1.7.9** of [RFC-183: Chico Vegetation](../../../README.md)


The Date Palm is a tall, vertical palm with a dense, layered crown. Compared to [Waialea Palm](../08-waialea-palm/README.md#3178-waialea-palm), it is less arched, more columnar, and has a fuller, more structured canopy.

**Shape**

* Tall, straight trunk
* Dense crown with many frond layers
* Lower fronds droop outward and downward
* Upper fronds project upward
* Strong vertical silhouette

**Trunk**

Use the [Palm Trunk](../../06-well-known-component-constructions/02-palm-trunk/README.md#3162-palm-trunk) construction without arching.

```rust
let trunk_height = 0.90 * H;
let trunk_radius = 0.025 * H;
```

Use a tight, vertical chain:

```rust
HysteresisConfig {
    bias_ray: Vec3::Y,
    bias_strength: very_high,
    angle_tolerance: radians(2.0),
    child_count: 1..=1,
    length_range: 0.05 * H..0.08 * H,
    radius_range: trunk_radius..trunk_radius,
}
```

Maintain the inverted taper per segment for banded trunk appearance:

```rust
segment.base_radius = r * 0.92;
segment.top_radius = r;
```

This produces the characteristic stacked palm trunk.

**Crown Anchors**

Place the crown at the trunk tip.

```rust
let crown = trunk_tip;
let ring_count = 6..=10;
let fronds_per_ring = 10..=16;
let ring_spacing = 0.01 * H;
```

Use the [Palm Crown](../../06-well-known-component-constructions/01-palm-crown/README.md#3161-palm-crown) construction with many tightly stacked layers.

```rust
for ring in 0..ring_count {
    let u = ring as f32 / (ring_count - 1) as f32;

    let vertical_bias = mix(
        -0.10,  // lower rings droop
        0.60,   // upper rings rise
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

Use [Fronds](../../02-ball-components/07-fronds/README.md#3127-fronds) with longer and more structured leaves than Waialea Palm.

```rust
FrondConfig {
    segments: 10..=16,
    length: 0.35 * H..0.50 * H,
    width: 0.05 * H,
    droop: medium_to_high,
    twist: mild,
    leaflet_count: 14..=24,
}
```

Lower fronds should droop noticeably; upper fronds should rise or remain near horizontal.

**Ball Selection**

Date Palm uses direct frond allocation from crown anchors rather than ball-stick node selection. Optionally, place a dense central mass to conceal the frond base:

```rust
spawn_tuft(
    position = crown,
    direction = Vec3::Y,
    scale = 0.05 * H,
);
```

**Materials**

* Stick shader: fibrous palm bark, layered or banded trunk
* Leaf shader: bright or dusty green palm leaves

**Variants**

* Increase droop for desert palms.
* Reduce ring count for younger palms.
* Add [Fruiting Bodies](../../06-well-known-component-constructions/07-fruiting-bodies/README.md#3167-fruiting-bodies) beneath the crown for date clusters.

