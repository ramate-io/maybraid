# Maybraid World Materials

The composed [`MaterialRef`](../../material-ref) library for Maybraid World.

Domain shaders stay in their crates until a larger grouping exists. The mythical chain lives in [`src/chain`](src/chain) because it does not yet have one. This crate is the global recipe-name table: furniture, character, urban (including `doric_column`), muzzle flame, energy (`laser_hex`, `laser_pulse`, `laser_tail`), mythical chain (`mythical_chain`), then vegetation and Standard.

```text
cargo test -p world-materials --all-targets
cargo run -p world-materials-playground -- show chain
cargo run -p world-materials-playground -- chain capture
```

Committed chain stills live in [`evidence/chain`](evidence/chain).
