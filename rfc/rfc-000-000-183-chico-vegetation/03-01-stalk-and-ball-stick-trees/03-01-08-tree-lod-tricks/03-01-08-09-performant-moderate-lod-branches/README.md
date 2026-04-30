# 3.1.8.9: Performant Moderate-LOD Branches

This page is subsection **3.1.8.9** of [RFC-183: Chico Vegetation](../../../README.md)


Use low-resolution noisy cylinders for major branches only:

* skip smaller branches
* merge segments where possible

```rust
segments: 1..=2
```

---

