# 3.1.8.11: Varied Moderate-LOD Canopy

This page is subsection **3.1.8.11** of [RFC-183: Chico Vegetation](../../../README.md)


Use noise to vary between:

* standard icosahedron
* [Jessen's Icosahedron](https://en.wikipedia.org/wiki/Jessen%27s_icosahedron)

```rust
if noise(seed) < 0.5 {
    use_icosahedron();
} else {
    use_jessen();
}
```

This subtly breaks silhouette uniformity without increasing cost.

---

