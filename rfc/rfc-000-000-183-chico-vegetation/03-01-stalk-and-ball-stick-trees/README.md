# 3.1: Stalk and Ball-stick Trees

This page is subsection **3.1** of [RFC-183: Chico Vegetation](../README.md)


Stalk and ball-stick trees form the core geometric system for Chico vegetation. The design refines the current system which uses [an ad hoc stalk with radial projection](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/vegetation/src/tree.rs#L171), a [`BallStick`](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/comproc/src/complex/chain/ball_stick/builder.rs) complex for the canopy, a [noisy cylinder](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/vegetation/src/tree/meshes/trunk/segment.rs) for trunk and branch segments, and a [planar canopy](https://github.com/ramate-io/maybraid/blob/9c38f45cfd697a392e6114bbc6e67b50005b7f65/procedures/vegetation/src/tree/meshes/canopy/ball.rs).

The goal is to formalize this into a composable, parameterized system:

* **Stalks** define primary vertical structure
* **Sticks** define trunk and branch segments
* **Balls and planes** define canopy and foliage
* **Ball-stick chains** unify tree construction

Tree types are then expressed as parameterized constructions over these primitives rather than bespoke implementations.

---

Subsections:

- [3.1.1: Stick and Stalk Components](./01-stick-and-stalk-components/README.md)
- [3.1.2: Ball Components](./02-ball-components/README.md)
- [3.1.3: Ball-stick Anchors](./03-ball-stick-anchors/README.md)
- [3.1.4: Ball-stick Chains](./04-ball-stick-chains/README.md)
- [3.1.5: Ball Selection](./05-ball-selection/README.md)
- [3.1.6: Well-known Component Constructions](./06-well-known-component-constructions/README.md)
- [3.1.7: Well-known Tree Constructions](./07-well-known-tree-constructions/README.md)
- [3.1.8: Tree LOD Tricks](./08-tree-lod-tricks/README.md)
- [3.1.9: Stick Shading](./09-stick-shading/README.md)
- [3.1.10: Leaf Shading](./10-leaf-shading/README.md)

