# 3.5.1: Parameterization

This page is subsection **3.5.1** of [RFC-183: Chico Vegetation](../../README.md)

Forest parameterization controls how a selected forest layering is adapted to a specific region of terrain. A forest does not rewrite the groves it selects; instead, it passes down sampled modifiers that bias grove scale, density, offset, noise, palette, and placement behavior.

This is the layer where broad biome variation is expressed. The same grove can feel compact and sparse in one forest cell, or taller, denser, and more perturbed in another, while still preserving the grove's internal construction rules.

## 3.5.1.1: Grove Parameter Overrides

Each [Cellular Grove](../../03-04-cellular-groves/README.md) defines its own parameter ranges. A forest may provide override factors that multiply, narrow, widen, or offset those ranges before the grove samples its final parameters.

```rust
pub struct ForestGroveModifiers {
    scale_factor: Range<f32>,
    density_factor: Range<f32>,
    offset_factor: Range<f32>,

    noise_amplitude_factor: Range<f32>,
    noise_frequency_factor: Range<f32>,

    elevation_bias: Range<f32>,
    steepness_bias: Range<f32>,
}
```

For example, a high-elevation forest may shrink broadleaf groves, increase conifer density, and reduce ground-cover noise. A wet tropical forest may increase lower-canopy density, widen scale ranges, and allow stronger offset perturbation.

## 3.5.1.2: Spatial Coherence

Modifiers should be sampled with spatially coherent noise at the forest-cell scale. This keeps neighboring forest cells related without forcing them to be identical.

```rust
let modifiers = ForestGroveModifiers {
    scale_factor: sample_range(0.85..1.20, scale_noise(cell)),
    density_factor: sample_range(0.75..1.35, density_noise(cell)),
    offset_factor: sample_range(0.80..1.10, offset_noise(cell)),

    noise_amplitude_factor: sample_range(0.75..1.40, amplitude_noise(cell)),
    noise_frequency_factor: sample_range(0.80..1.25, frequency_noise(cell)),

    elevation_bias: sample_range(-0.05..0.05, elevation_noise(cell)),
    steepness_bias: sample_range(-0.04..0.08, steepness_noise(cell)),
};
```

The sampled modifiers are then passed to each selected grove in the forest layering. The grove applies them to its own [Parameterization](../../03-04-cellular-groves/01-parameterization/README.md), then proceeds with normal grove-level selection and placement.

## 3.5.1.3: Invariants

Forest parameterization should respect the grove's authored identity:

* It may scale a grove up or down, but should not turn an understory grove into an upper-canopy grove.
* It may make a grove denser or sparser, but should not erase the meaning of `None` in the forest layer distribution.
* It may bias constraints, but should not bypass per-variant placement constraints.
* It should keep deterministic sampling stable for a given world seed and forest cell.

Use forest modifiers for broad environmental expression. Use grove parameterization for the grove's local behavior.
