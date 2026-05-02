# 3.4.2.5: Variant Selection

This page is subsection **3.4.2.5** of [RFC-183: Chico Vegetation](../../../README.md)


Selection uses a first-fit approach over the ordered distribution:

```rust
let start = bucket_throw(noise);

for variant in distribution.first_fit_from(start) {
    if variant.valid_at(position, terrain) {
        return variant;
    }
}
```

* start from a noise-derived index
* evaluate that variant's placement constraints
* if placement fails, move to an adjacent bucket in distribution order
* select the first valid variant
* preserve distribution while respecting constraints

---

This structure intentionally mirrors terrain detail systems so that:

* placement behavior is predictable
* systems compose cleanly
* spatial artifacts (flicker, migration) are avoided

...while still allowing rich biome-level variation.

