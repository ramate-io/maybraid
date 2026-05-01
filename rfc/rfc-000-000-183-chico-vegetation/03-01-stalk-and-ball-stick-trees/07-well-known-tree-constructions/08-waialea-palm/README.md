# 3.1.7.8: Waialea Palm

This page is subsection **3.1.7.8** of [RFC-183: Chico Vegetation](../../../README.md)


Waialea Palm is a gently arched palm with a light, layered crown. It is useful for tropical coastlines, riparian edges, resorts, gardens, and sparse warm-region groves.

**Shape**

* Slender arched trunk
* Crown concentrated at the top
* Two to three frond rings
* Lower fronds droop or project outward
* Upper fronds rise more vertically

**Trunk**

Use the [Palm Trunk](../../06-well-known-component-constructions/02-palm-trunk/README.md#3162-palm-trunk) construction with a gentle arch.

```rust
let trunk_height = 0.85 * H;
let trunk_radius = 0.025 * H;
```

Use a tight upward chain with slight persistent lateral bias:

```rust
let arch_bias = Vec3::new(0.12, 1.0, 0.0).normalize();

HysteresisConfig {
    bias_ray: arch_bias,
    bias_strength: high,
    angle_tolerance: radians(4.0),
    child_count: 1..=1,
    length_range: 0.05 * H..0.08 * H,
    radius_range: trunk_radius..trunk_radius,
}
```

Invert the usual tapering per segment, so each segment’s top is slightly wider than its base:

```rust
segment.base_radius = r * 0.92;
segment.top_radius = r;
```

This gives a stacked palm-trunk impression.

**Crown Anchors**

Place the crown at the trunk tip.

```rust
let crown = trunk_tip;
let ring_count = 2..=3;
let fronds_per_ring = 8..=12;
let ring_spacing = 0.015 * H;
```

Use the [Palm Crown](../../06-well-known-component-constructions/01-palm-crown/README.md#3161-palm-crown) construction.

```rust
for ring in 0..ring_count {
    let vertical_bias = base_bias + ring as f32 * bias_step;

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

Use [Fronds](../../02-ball-components/07-fronds/README.md#3127-fronds) as mesh-based arching chains.

```rust
FrondConfig {
    segments: 8..=14,
    length: 0.28 * H..0.42 * H,
    width: 0.045 * H,
    droop: medium,
    twist: mild,
    leaflet_count: 10..=18,
}
```

Lower rings should have less vertical bias and more droop. Higher rings should start more upright.

```rust
let base_bias = 0.10;
let bias_step = 0.18;
```

**Ball Selection**

Waialea Palm does not use ordinary ball selection over a branch graph. The crown directly allocates fronds from crown anchors. Optional small [Tufts](../../02-ball-components/06-tufts/README.md#3126-tufts) may be placed at the crown center to conceal the frond origins.

```rust
spawn_tuft(
    position = crown,
    direction = Vec3::Y,
    scale = 0.04 * H,
);
```

**Materials**

* Stick shader: palm bark, dry fibrous bark, or banded trunk material
* Leaf shader: tropical palm green, coastal green, or dry palm tones

**Variants**

* Increase arch bias for windswept coastal palms.
* Reduce ring count for sparse decorative palms.
* Add [Fruiting Bodies](../../06-well-known-component-constructions/07-fruiting-bodies/README.md#3167-fruiting-bodies) near the crown for coconut-like variants.

