# 3.5.2: Selection

Cellular forest selection chooses which forest layering should control a forest cell. It operates one level above [Cellular Groves](../../03-04-cellular-groves/README.md): groves decide what to place inside a layer, while forest selection decides which coherent stack of grove layers belongs at a point.

## 3.5.2.1: Hopscotch

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

## 3.5.2.2: Determinism

Hopscotch selection must be deterministic for a given world seed, forest cell, and distribution. Different noise salts should be used for anchor selection, hop budget selection, and each hop step, so changing the number of hops does not also change the initial anchor.

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

The result should be stable under camera movement and frame timing. It may change when the forest distribution, world seed, or forest-cell coordinate changes.

## 3.5.2.3: Authoring Guidance

Use Hopscotch when forest layerings should vary coherently across space but should not be arranged on a simple linear spectrum.

Good Hopscotch distributions tend to:

* Put high weights on common, stable, biome-defining layerings.
* Put low weights on transition, accent, or edge-case layerings.
* Link each layering to plausible neighbors, not to every other type.
* Use loop-back edges where a type should resist noisy drift.
* Keep directed one-way edges rare and intentional.

In Chico vegetation, Hopscotch selection chooses between different well-known layerings of groves. See [Well-known Layerings](../04-well-known-layerings/README.md).