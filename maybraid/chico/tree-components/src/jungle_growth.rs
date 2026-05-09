//! **Jungle growth** assembly: secondary foliage at selected canopy balls ([RFC-183 §3.1.6.4](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/04-jungle-growths/README.md)).
//!
//! # Intent for Sope's Banyan ([§3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252))
//!
//! RFC pattern at a canopy node: a **scaled darker noisy ball** for inner depth, plus a **`spawn_tuft`** pass for protruding wet/overgrown silhouette. This module will grow **`tuft`** primitives and orchestration **after** the core Sope recipe (stalk, anchors, chains, sticks, primary balls) is stable.
//!
//! # Ordering
//!
//! Implement **last** in the Sope stack: depends on ball selection hooks and optional dense-variant flags; parameters should remain **clap-friendly** for playground iteration alongside the main tree recipe.
