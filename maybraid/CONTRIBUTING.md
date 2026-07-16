# Contributing

## Organization and Naming

There are a few key organization and naming rules that will help to track Maybraid development.

### Proper Noun Implementations 

Early implementations, particularly those following an RFC, will often have a name that is a proper noun. For example, [Crozon](./crozon/) and [Durham](./durham/) which are early implementations of characters and terrain types and systems respectively. 

While the assets associated with these layers should continue to sit behind this proper name, more general logic should increasingly be moved into more generally named crates and directories as the implementation matures.

### `-models` Crates

The term "model" and suffix `-models` is used to refer to a layer that defining the base behavior of a game object. Typically, this means taking a lower-order asset, such as tree from [`chico-sbs-trees`](./chico/sbs-trees/), and integrating it with standard game systems such as LOD, generation, and physics. Accordingly, models should typically define plugins that idempotently make available the needed systems for base behaviors. 

Particularly bespoke systems, like player damage, movement, and inventory, are not necessarily considered parts of models until the underlying API is generalized. Before that point, they are expected to be implemented as separate systems acting on the types that the models define. 

At the time of writing, building models refers to defining behavior with respect to...

- [`lod`](./lod/lib)
- [`generation`](./lod/lib/src/gen.rs)
- [Avian Physics](https://github.com/avianphysics/avian)

...and mostly consists of implementing the traits from [`generation`](./lod/lib/src/gen.rs) with the added colliders. 

Sometimes, particularly during early development of a model, the game object may only be defined within the `*-models` crate. However, generally, things like the composition of a game object will be defined in another crate and then extended with the model. This pattern keeps the `*-models` crate focused on the behavior of the game object rather than its internal structure. For example, rather than defining the procedure to give all branches on a tree, the `*-models` crate implementation of the tree can simply focus on which branches are visible at a given LOD. Conversely, the crate implementing the tree does not have to worry about plugging into the generation dependency system from the start. 

> [!IMPORTANT]
> Please update this section if increasing or different layers are consistently implemented at the `-models` level.