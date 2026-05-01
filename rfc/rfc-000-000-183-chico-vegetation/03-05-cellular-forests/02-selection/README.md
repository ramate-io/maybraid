# 3.5.2: Selection


## 3.5.2.1: Hopscotch
Cellular forest cell selection follows a similar concept to [Bucket Throw](../../03-04-cellular-groves/03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md), but over a graph instead of contiguous buckets. This algorithm is called Hopscotch. This helps to generalize the local coherence of Bucket Throw. Now instead of only have two adjacent buckets, each type can have arbitrarily many. 

- Each cell has its own bin weight. This will be used to determine the likelihood of selection as an anchor and the cost of traversing through the cell type as a node on the graph.  
- Each cell also assigns a set of weighted edges to other variants. 
- At the start, we use a separately sampled noise values to select the anchor and a hop budget.
- We then noisily select a traversal path over the graph, respecting the edge weights as likelihoods for each step. Each time we move, we spend the bin weight of the node we are leaving from our hop budget. If we cannot leave a node, we are done. 

The description of a hopscotch distribution looks something like this:

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
            (A, 1.0)
        ],
        item: A
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
    })
}
```

> [!NOTE]
> The graph is directed to allow for flexibility. However, in most cases, the programmer will want to reverse link any forward links.

> [!NOTE]
> Sometimes a loop-back link is desirable when we want to express that the type tends to stay self-same. 

In Chico vegetation, we use Hopscotch Selection to choose between different well-known layerings of groves. See [Well-known Layerings](../04-well-known-layerings/README.md).