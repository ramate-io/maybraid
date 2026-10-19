# Maybraid World Materials

The composed [`MaterialRef`](../../material-ref) library for Maybraid World.

Domain shaders stay in their crates until a larger grouping exists. This crate is the global recipe-name table: furniture, character, urban (including `doric_column`), muzzle flame, energy (`laser_hex`, `laser_pulse`, `laser_tail`), mythical chain (`mythical_chain`), then vegetation and Standard.

```text
cargo test -p world-materials
```
