# 3.4.2.5: Variant Selection

This page is subsection **3.4.2.5** of [RFC-183: Chico Vegetation](../../../README.md)


Selection uses a next-fit approach over the ordered distribution:

```rust
selection(elevation, steepness, noise)
```

* start from a noise-derived index
* select nearest valid variant
* preserve distribution while respecting constraints

---

This structure intentionally mirrors terrain detail systems so that:

* placement behavior is predictable
* systems compose cleanly
* spatial artifacts (flicker, migration) are avoided

...while still allowing rich biome-level variation.

