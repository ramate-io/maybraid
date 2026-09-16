# Furniture usage areas

Expand Richmond [`FurnitureUsageNode`](../../richmond/building-components/src/furniture/usage.rs) regions into ensembles of [`FurnitureNode`](../../richmond/building-components/src/furniture/node.rs) slots.

Richmond packers keep labels such as `BitesCounter` and `BitesKitchen` on the packed AABB. They do **not** remap a whole band to one stretched `Counter`. This crate fills those boxes:

- [`BitesCounterUsage`](src/bites_counter.rs) — ~0.85 m passage-face stations (not a room-deep slab), sit-on display facing the customer, leftover stock / lounge in deep boxes
- [`BitesKitchenUsage`](src/bites_kitchen.rs) — wall-run counters plus a peninsula, sit-on cooktop + pans, basin, full-height fridge, leftover shelves and lounge
- [`BitesSeatingUsage`](src/bites_seating.rs) — cafe tables and chairs fitted into the packed seating box

Depends on [`richmond-building-components`](../../richmond/building-components/) only. [`furniture-assemblies`](../assemblies/) `collect_furniture_slots` / the 50 m cell stream call [`expand_usages`](src/expand.rs) after collecting High usage nodes.
