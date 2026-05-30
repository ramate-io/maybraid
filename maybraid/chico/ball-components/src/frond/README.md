# Frond crown

Mesh-based arching frond chains for palm crowns, fern bushes, and tropical canopy detail ([#218](https://github.com/ramate-io/maybraid/issues/218), [RFC-183 §3.1.2.7](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/07-fronds/README.md)).

## Role

Each frond is a curved spine with alternating tapered leaflets. Crowns merge many fronds into one mesh at a shared anchor. Not SDF-backed.

## Modules

| File | Purpose |
|------|---------|
| [`config.rs`](config.rs) | [`FrondConfig`] — length, width, droop, twist, leaflet count |
| [`spine.rs`](spine.rs) | Quadratic droop spine + tangent frame |
| [`leaflet.rs`](leaflet.rs) | Tapered quad leaflets along the spine |
| [`construction.rs`](construction.rs) | [`FrondElement`], [`FrondCluster`] mesh merge |
| [`crown.rs`](crown.rs) | Palm-like downward/outward direction cap |

## See also

- [Tufts](../tuft.rs) — compact prisms and grass ribbons
- [Buddha's-hand tuft](../tuft/buddha_hand/README.md) — palm-bush massing before dedicated fronds
