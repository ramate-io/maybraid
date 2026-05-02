# 3.5.2: Selection and Construction

Cellular forest selection chooses which forest layering controls each forest cell, then constructs the selected layers by instantiating grove grids inside that forest cell. It operates one level above [Cellular Groves](../../03-04-cellular-groves/README.md): forests choose coherent stacks of grove layers, while groves choose individual grove cells and vegetation variants.

Every forest cell is active. There is no forest-level activation test. Each forest cell selects a forest layering through Hopscotch, then evaluates the layer distributions inside that layering.

Every grove cell is also active. There is no grove-level activation test. Each grove cell chooses a grove variant through Bucket Throw; `None` is represented explicitly as a possible selected item in the layer or grove distribution when emptiness is desired.

## 3.5.2.1: Forest Cells

The world is divided into a grid of forest cells. Each forest cell owns:

* A selected forest layering.
* A set of sampled [forest parameter biases](../01-parameterization/README.md).
* One grove grid per selected layer.

The forest cell is the coherence unit for broad biome identity. Neighboring forest cells may select related layerings through Hopscotch adjacency, but each forest cell still has a single resolved layering.

## 3.5.2.2: Hopscotch

Hopscotch is the forest-level counterpart to [Bucket Throw](../../03-04-cellular-groves/02-selection-and-placement/01-bucket-throw/README.md). Bucket Throw preserves local coherence by moving through adjacent buckets in a one-dimensional distribution. Hopscotch generalizes that idea to a directed graph, so each forest type can choose its own compatible neighbors instead of only having a left and right neighbor.

A Hopscotch distribution is made from nodes. Each node represents a candidate forest layering and provides:

* `weight`: the node's anchor likelihood and traversal cost.
* `adjacencies`: weighted outgoing edges to compatible neighboring layerings.
* `item`: the forest layering selected if traversal ends on this node.

Selection uses two independent deterministic noise samples:

* `anchor_noise`: chooses the starting node from the node weights.
* `hop_noise`: chooses a hop budget and drives edge choices during traversal.

The algorithm is:

1. Select an anchor node by weighted throw over all node `weight` values.
2. Sample a hop budget from the forest's configured hop range.
3. While the current node has outgoing edges and enough budget remains to leave it:
   1. Spend the current node's `weight` from the hop budget.
   2. Select one outgoing edge using its adjacency weights.
   3. Move to the selected neighbor.
4. Return the `item` on the final node.

This gives high-weight nodes two effects: they are more likely to be selected as anchors, and they are harder to traverse away from. Low-weight nodes are less likely to be initial anchors, but once reached, they are easier to cross through.

The description of a Hopscotch distribution looks like this:

```rust
pub enum MyHopscotch {
    A(Bucket {
        weight: 4.0,
        adjacencies: [
            (B, 1.0),
            (C, 2.0),
            (D, 0.5)
        ],
        item: A
    }),
    B(Bucket {
        weight: 1.0,
        adjacencies: [
            (A, 1.0),
            (B, 0.5),
        ],
        item: B
    }),
    C(Bucket {
        weight: 1.0,
        adjacencies: [
            (B, 1.0),
            (D, 1.0)
        ],
        item: C
    }),
    D(Bucket {
        weight: 2.0,
        adjacencies: [
            (A, 1.0),
            (C, 1.0)
        ],
        item: D
    }),
}
```

> [!NOTE]
> The graph is directed for flexibility. In most distributions, forward links should have matching reverse links unless there is a deliberate one-way transition.

> [!NOTE]
> A loop-back edge is useful when a type should tend to remain self-same after traversal reaches it. This is separate from node weight: `weight` controls anchor likelihood and traversal cost, while a loop-back controls edge choice from that node.

## 3.5.2.3: Layer Selection

After Hopscotch selects the forest layering, each layer in that layering selects one grove or `None` with [Bucket Throw](../../03-04-cellular-groves/02-selection-and-placement/01-bucket-throw/README.md).

```rust
let layering = hopscotch_select(forest_distribution, forest_cell);
let biases = sample_forest_biases(forest_cell);

let ground_cover = bucket_throw(layering.ground_cover, forest_cell);
let tufts = bucket_throw(layering.tufts, forest_cell);
let understory = bucket_throw(layering.understory, forest_cell);
let lower_canopy = bucket_throw(layering.lower_canopy, forest_cell);
let upper_canopy = bucket_throw(layering.upper_canopy, forest_cell);
```

The forest layer distribution is the place where emptiness is authored. If the upper canopy should be absent, the upper-canopy layer selects `None`. The forest system should not skip cells through a separate activation rule.

## 3.5.2.4: Grove Grid Construction

Each selected grove creates a grid of grove cells inside the forest cell. The grove grid uses the selected grove's own cell size, offset, density, noise, and placement rules, after applying any forest-level parameter biases.

```rust
for selected_grove in selected_layer_groves {
    let grove_parameters = selected_grove
        .parameters()
        .with_forest_biases(biases);

    for grove_cell in grid_inside(forest_cell, grove_parameters.cell_size) {
        let grove_variant = bucket_throw(selected_grove.distribution, grove_cell);
        construct_grove_variant(grove_variant, grove_cell, grove_parameters);
    }
}
```

All grove cells are active. If a grove wants empty outcomes, `None` must be present in the grove distribution. If a grove variant cannot be placed at a sampled point because of per-variant constraints, the grove-level first-fit behavior chooses another valid variant as described in [Variant Selection](../../03-04-cellular-groves/02-selection-and-placement/05-variant-selection/README.md).

## 3.5.2.5: Determinism

Selection and construction must be deterministic for a given world seed, forest cell, grove cell, and distribution. Different noise salts should be used for forest anchor selection, forest hop budget selection, layer selection, grove parameter sampling, grove variant selection, and placement perturbation.

```rust
let anchor = weighted_throw(distribution.nodes, anchor_noise(cell, seed));
let mut current = anchor;
let mut budget = sample_hop_budget(hop_budget_noise(cell, seed));

while budget >= current.weight && !current.adjacencies.is_empty() {
    budget -= current.weight;

    let edge_noise = hop_edge_noise(cell, seed, step);
    current = weighted_throw(current.adjacencies, edge_noise);
    step += 1;
}

return current.item;
```

The result should be stable under camera movement and frame timing. It may change when the forest distribution, world seed, forest-cell coordinate, or grove-cell coordinate changes.

## 3.5.2.6: Authoring Guidance

Use Hopscotch when forest layerings should vary coherently across space but should not be arranged on a simple linear spectrum.

Good Hopscotch distributions tend to:

* Put high weights on common, stable, biome-defining layerings.
* Put low weights on transition, accent, or edge-case layerings.
* Link each layering to plausible neighbors, not to every other type.
* Use loop-back edges where a type should resist noisy drift.
* Keep directed one-way edges rare and intentional.

In Chico vegetation, Hopscotch selection chooses between different well-known layerings of groves. See [Well-known Layerings](../04-well-known-layerings/README.md).
