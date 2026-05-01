# 3.5.1: Parameterization

This page is subsection **3.5.1** of [RFC-183: Chico Vegetation](../../README.md)

Forest parameterization controls how a selected forest layering is adapted to a specific region of terrain. A forest does not rewrite the groves it selects, and it does not replace grove-authored ranges. Instead, it passes down sampled bias values that influence how each grove samples within its own ranges.

This is the layer where broad biome variation is expressed. The same grove can feel compact and sparse in one forest cell, or taller, denser, and more perturbed in another, while still preserving the grove's internal construction rules.

## 3.5.1.1: Grove Parameter Biases

Each [Cellular Grove](../../03-04-cellular-groves/README.md) defines its own parameter ranges. A forest may provide unit-interval bias values that set a new preferred mean within those ranges, but the grove still owns the minimum and maximum.

The forest-level value should be interpreted as a percentile mean, not as a new range. `0.0` means "prefer the low end", `0.5` means "prefer the center", and `1.0` means "prefer the high end".

```rust
let grove_scale = biased_sample(
    grove.scale_range,
    forest_bias.scale_mean,
    grove_scale_noise(cell),
);
```

One simple implementation is to convert the unit value into an anchor inside the range, then sample around it with saturation:

```rust
fn biased_sample(range: Range<f32>, mean_unit: f32, noise: f32) -> f32 {
    let mean = lerp(range.start, range.end, mean_unit.clamp(0.0, 1.0));
    let radius = max(mean - range.start, range.end - mean);
    saturate_to_range(mean + noise.remap(-radius, radius), range)
}
```

This is a saturating add, not a wrapping add. Wrapping makes sense for circular bucket space, but scalar ranges such as scale, density, offset, elevation, and steepness should remain inside their authored bounds.

```rust
pub struct ForestGroveBiases {
    scale_mean: Range<f32>,   // unit interval
    density_mean: Range<f32>, // unit interval
    offset_mean: Range<f32>,  // unit interval

    noise_amplitude_mean: Range<f32>, // unit interval
    noise_frequency_mean: Range<f32>, // unit interval

    elevation_mean: Range<f32>, // unit interval
    steepness_mean: Range<f32>, // unit interval

    bucket_mean_shift: Range<f32>,
    bucket_perturbation_bias: Range<f32>,
}
```

For example, a high-elevation forest may set broadleaf `scale_mean` near the low end of the unit interval, set conifer `density_mean` higher, and set ground-cover `noise_amplitude_mean` lower. A wet tropical forest may bias lower-canopy density upward, bias scale upward, and allow stronger bucket perturbation, all without changing authored ranges.

## 3.5.1.2: Spatial Coherence

Biases should be sampled with spatially coherent noise at the forest-cell scale. This keeps neighboring forest cells related without forcing them to be identical.

```rust
let biases = ForestGroveBiases {
    scale_mean: sample_range(0.35..0.62, scale_noise(cell)),
    density_mean: sample_range(0.45..0.75, density_noise(cell)),
    offset_mean: sample_range(0.35..0.58, offset_noise(cell)),

    noise_amplitude_mean: sample_range(0.35..0.70, amplitude_noise(cell)),
    noise_frequency_mean: sample_range(0.40..0.60, frequency_noise(cell)),

    elevation_mean: sample_range(0.45..0.55, elevation_noise(cell)),
    steepness_mean: sample_range(0.45..0.62, steepness_noise(cell)),

    bucket_mean_shift: sample_range(-0.25..0.25, bucket_mean_noise(cell)),
    bucket_perturbation_bias: sample_range(-0.20..0.30, bucket_perturbation_noise(cell)),
};
```

The sampled biases are then passed to each selected grove in the forest layering. The grove applies them to its own [Parameterization](../../03-04-cellular-groves/01-parameterization/README.md), then proceeds with normal grove-level selection and placement.

Bucket-related biases are passed to [Bucket Throw](../../03-04-cellular-groves/02-selection-and-placement/01-bucket-throw/README.md): `bucket_mean_shift` shifts where the throw is centered, while `bucket_perturbation_bias` controls deterministic changes to bucket sizes.

## 3.5.1.3: Invariants

Forest parameterization should respect the grove's authored identity:

* It may bias scale sampling up or down inside the grove's scale range, but should not turn an understory grove into an upper-canopy grove.
* It may bias density sampling denser or sparser inside the grove's density range, but should not erase the meaning of `None` in the forest layer distribution.
* It may bias constraints, but should not bypass per-variant placement constraints.
* It should keep deterministic sampling stable for a given world seed and forest cell.

Use forest biases for broad environmental expression. Use grove parameterization for the grove's local behavior.
