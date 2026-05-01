# 3.6: Elder Trees

This page is subsection **3.6** of [RFC-183: Chico Vegetation](../README.md)

Elder Trees are massive tree constructions intended to pair tightly with urbanization. They are not just large vegetation assets; they are living terrain and architectural anchors where paths, platforms, shrines, homes, bridges, and other built features can be constructed on or around the tree.

They build on the same [Stalk and Ball-stick Trees](../03-01-stalk-and-ball-stick-trees/README.md) vocabulary as ordinary trees, but operate at a different planning scale.

## Allocation

For this version, Elder Trees use a separate allocation grid from [Cellular Forests](../03-05-cellular-forests/README.md). Forests remain responsible for broad vegetation cover, while the elder-tree grid places rare, intentional landmarks.

This separation keeps elder trees from behaving like another canopy grove. An elder tree may influence surrounding forests, villages, paths, or clearings, but it is not selected as part of a forest layering.

## Scale

Initial Elder Trees should be large, but not so large that they require separate asset streaming or fundamentally different LOD assets.

A reasonable starting envelope:

```rust
pub struct ElderTreeEnvelope {
    height: 80.0..400.0,
    radius: 20.0..100.0,
}
```

The upper end is large enough to support urban features, but still small enough that ordinary asset and tree LOD techniques can be adapted rather than replaced.

## Urban Pairing

Elder Trees should expose structural affordances for urban systems:

* broad branch shelves for platforms
* trunk flares and buttresses for entrances
* canopy hollows for shrines or small buildings
* root arches for paths and thresholds
* vertical circulation routes around the trunk
* stable anchor points for bridges and hanging structures

The vegetation construction should reserve readable spaces where architecture can attach. A beautiful elder tree that cannot host paths or structures is only a large tree, not an elder-tree settlement anchor.

## Well-known Elder Tree Types

Suggested initial elder-tree types:

* **Elder Storybook Tree**: a giant [Storybook Tree](../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/01-storybook-tree/README.md) with broad rounded canopy shelves and hospitable branch platforms.
* **Elder Braid Oak**: a giant [Braid Oak](../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/13-braid-oak/README.md) with interwoven trunks, bridge-like limbs, and strong cultural-grove identity.
* **Elder Honu Banyan**: a huge [Honu Banyan](../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/05-honu-banyan/README.md) with descenders, aerial roots, and village-like internal rooms.
* **Elder Sope's Banyan**: a mystical [Sope's Banyan](../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md) variant with hanging structures, shrine hollows, and dense vertical roots.
* **Elder Waialea Palm**: a massive [Waialea Palm](../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/08-waialea-palm/README.md) used as a tropical tower, lookout, or ceremonial landmark.
* **Elder Conifer Spire**: a giant [Northern Conifer](../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/11-northern-conifer/README.md) or [Friend's Conifer](../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/14-friend-s-conifer/README.md) with vertical platforms and high-canopy bridge anchors.

These types should reuse ball-stick components where possible, but add explicit urban attachment structure as part of their elder-tree construction.
