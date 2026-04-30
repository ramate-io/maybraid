# 3.4.2.1: Bucket Throw

This page is subsection **3.4.2.1** of [RFC-183: Chico Vegetation](../../../README.md)


The bucket throw algorithm maps variants to contiguous weighted regions.

* Each variant has a **weight** and **position**
* Weights define region size
* Positions define ordering

Selection:

$$
variant = bucket(mean + s([-T, T]))
$$

where:

* $T$ is total ordering span
* $s$ is a centrally-biased noise sample

```rust
let shift = noise(seed).remap(-total_order, total_order);
let idx = wrap(mean + shift, total_order);
let variant = bucket_lookup(idx);
```

This produces:

* locally coherent variation
* gradual composition shifts
* non-uniform but stable distributions

> [!NOTE]
> Canonically, the default mean is at 0.0 in bucket space.

---

