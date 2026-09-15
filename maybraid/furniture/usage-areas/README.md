# Furniture usage areas

Expand Richmond [`FurnitureUsageNode`](../../richmond/building-components/src/furniture/usage.rs) regions into ensembles of [`FurnitureNode`](../../richmond/building-components/src/furniture/node.rs) slots.

Richmond packers keep labels such as `BitesCounter` and `BitesKitchen` on the packed AABB. They do **not** remap a whole band to one stretched `Counter`. This crate fills those boxes:

- [`BitesCounterUsage`](src/bites_counter.rs) — station-width counters (about 1.2–1.8 m) plus leftover sit-on display / fruit / bread on the 1 m slab
- [`BitesKitchenUsage`](src/bites_kitchen.rs) — wall-run counters with a reserved range whenever the run fits, shelves above the run, fridge on leftover wall (or the run end), basin + faucet, cookware

Depends on [`richmond-building-components`](../../richmond/building-components/) only. [`furniture-assemblies`](../assemblies/) `collect_furniture_slots` / the 50 m cell stream call [`expand_usages`](src/expand.rs) after collecting High usage nodes.
