# 3.4.2.1: Bucket Throw

This page is subsection **3.4.2.1** of [RFC-183: Chico Vegetation](../../../README.md)


The bucket throw algorithm maps variants to contiguous weighted regions. It is used anywhere Chico vegetation needs locally coherent selection from an ordered weighted distribution: grove variants inside a grove, and groves inside a forest layer.

* Each variant has a **weight** and **position**
* Weights define region size
* Positions define ordering

Selection:

$$
variant = bucket(mean + shift + s([-T, T]))
$$

where:

* `mean` is the anchor point in bucket space.
* `shift` is a parent- or region-level signed offset in bucket space.
* `s([-T, T])` is an independent centrally-biased selection sample.
* `T` is the total ordering span.
* `total_order` is the full wrapped span of the distribution.

```rust
let selection = centered_selection_noise(seed, cell)
    .remap(-total_order, total_order);

let idx = wrap(mean_anchor + shift + selection, total_order);
let variant = bucket_lookup(idx);
```

This produces:

* locally coherent variation
* gradual composition shifts
* non-uniform but stable distributions

> [!NOTE]
> Canonically, the default mean anchor is at `0.0` in bucket space.

## 3.4.2.1.1: Mean Anchor and Shift

The mean anchor is the center of the throw. By default, it is `0.0`. A parent system may provide a `shift` to move that center toward another region of the wrapped bucket space.

```rust
let shift = shift_noise.remap(-shift_radius, shift_radius);
let shifted_mean = wrap(default_mean_anchor + shift, total_order);
```

Use shift noise when a parent system wants to move the center of selection without changing the distribution's weights. For example, a forest cell may pass a `bucket_mean_shift` down to a grove so that one part of the forest favors wetter variants while another favors drier variants.

Shift should be slower and more coherent than the independent selection sample. It expresses regional bias.

## 3.4.2.1.2: Perturbation

Perturbation changes the sizes of the buckets before lookup. It is not the local throw; the local throw remains the independent `s([-T, T])` selection sample.

Perturbation should preserve bucket order and avoid producing negative weights. A common implementation is to multiply each bucket's base weight by a small deterministic factor, then renormalize the distribution.

```rust
let perturbed_weight = max(
    MIN_BUCKET_WEIGHT,
    base_weight * (1.0 + perturbation_noise(bucket, cell) * perturbation_strength),
);

let perturbed_distribution = normalize_preserving_order(perturbed_weights);
```

Use perturbation when nearby cells should keep the same ordering but slightly change composition. A low perturbation strength keeps bucket sizes close to their authored weights. A high perturbation strength allows stronger local changes, but should still preserve the identity of the distribution.

## 3.4.2.1.3: Parent Biases

Parent systems may provide two bucket-throw biases:

* `bucket_mean_shift`: shifts the mean anchor before the independent selection sample.
* `bucket_perturbation_bias`: changes bucket sizes before lookup.

These values should not reorder buckets. Shift changes where sampling is centered in the existing bucket space. Perturbation changes bucket sizes, but only through bounded deterministic variation.

```rust
let shift = total_order * forest_bias.bucket_mean_shift;

let perturbation_strength = distribution.base_bucket_perturbation_strength()
    * remap_bias_to_factor(forest_bias.bucket_perturbation_bias);

let perturbed_distribution = perturb_bucket_sizes(
    distribution,
    perturbation_strength,
    bucket_size_noise(seed, cell),
);

let selection = centered_selection_noise(seed, cell)
    .remap(-perturbed_distribution.total_order, perturbed_distribution.total_order);

let idx = wrap(
    perturbed_distribution.default_mean_anchor + shift + selection,
    perturbed_distribution.total_order,
);

let variant = perturbed_distribution.bucket_lookup(idx);
```

This common scheme lets forest cells pass broad biome pressure into grove selection without overriding the grove's distribution.

---

