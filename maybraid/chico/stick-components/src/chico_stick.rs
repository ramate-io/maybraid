//! **Chico stick**: noisy tapered cylinder along +Y for ball-stick **segment** meshes.
//!
//! # Role in Sope's Banyan ([RFC-183 §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252))
//!
//! Sticks are the mesh primitive for each graph edge between parent and child nodes once a `BallStickChain` (and render helpers in `chico-sbs-geometry`) supply segment transforms. Bark-facing materials in the RFC (dark / wet / high-contrast fantasy bark) attach at the tree or playground layer; this crate stays the **reusable stick component** with a `FromScalarNoise` implementation for procedural variation.
