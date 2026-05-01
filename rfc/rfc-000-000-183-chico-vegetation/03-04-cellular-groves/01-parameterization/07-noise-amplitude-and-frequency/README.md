# 3.4.1.7: Noise Amplitude and Frequency

This page is subsection **3.4.1.7** of [RFC-183: Chico Vegetation](../../../README.md)


Controls spatial variation.

* Grove defines base amplitude and frequency
* Forest perturbs

```rust
let noise = fbm(world_pos * freq) * amplitude;
```

---

> [!NOTE]
> There is no palette parameterization or perturbation.
> This avoids excessive material variation and draw overhead.
> Visual diversity is instead achieved through world-space shading variation in species shaders.

---

