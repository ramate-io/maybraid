# RFC-N: Chico Vegetation

## Table of Contents

## 1: Summary

We propose the Chico vegetation system in response to [#61](https://github.com/ramate-io/maybraid/issues/61).

## 2: Prior Art

## 3: Design

### 3.1: Stalk and Ball-stick Trees

Stalk ball and stick trees are based on current system which uses [an ad hoc stalk with radial projection](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/vegetation/src/tree.rs#L171), a [`BallStick`](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/comproc/src/complex/chain/ball_stick/builder.rs) complex for the canopy, a [noisy cylinder](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/vegetation/src/tree/meshes/trunk/segment.rs) for the trunk and branch segments, and a [planar canopy](https://github.com/ramate-io/maybraid/blob/9c38f45cfd697a392e6114bbc6e67b50005b7f65/procedures/vegetation/src/tree/meshes/canopy/ball.rs).

#### 3.1.1: Stick and Stalk Components

##### 3.1.1.1: Noisy Cylinder

The original design. Good for most tree branches and trunks.

##### 3.1.1.2: Crook Cylinder

Adds a bends to the noisy cylinder. Good for variety. 

#### 3.1.2: Ball Components

Ball components are typically used for canopy. For the Chico vegetation, they do not need to be collision-supporting, hence they do not all have to have SDF backings. 

##### 3.1.2.1: Icosahedron

Good for filling out canopy shape at range. Generally should only be used for low LOD. Shading can be one-sided and opaque when used at range. 

Icospheres are also reasonable in moderate LOD contexts.

Note that icosahedrons and icospheres can also replace [Noisy Balls](#3122-noisy-ball) in [Plane Splays](#3125-plane-splay).

##### 3.1.2.2: Noisy Ball 

Good for filling out canopy at range or overlaying with [Plane Splay](#3125-plane-splay) for detail. Use one-sided shader at range, two-sided shader up close. 

#### 3.1.2.3: Octagonal Plane

Low triangle-count component of [Plane Splay](#3125-plane-splay)

#### 3.1.2.4: Triangular Plane

Low triangle-count component of [Plane Splay](#3125-plane-splay), also used for [Fronds](#3127-fronds) at moderate LOD. 

#### 3.1.2.5: Plane Splay

A refinement of the original design given at the poorly named [`NoisyBall`](https://github.com/ramate-io/maybraid/blob/9c38f45cfd697a392e6114bbc6e67b50005b7f65/procedures/vegetation/src/tree/meshes/canopy/ball.rs#L102-L231). Uses new [3.1.2.2: Noisy Ball](#3122-noisy-ball), [Octagonal Planes](#3123-octagonal-plane), and [Triangular Plane](#3124-triangular-plane). Good for high levels of detail. Can be constructed as a multi-mesh or singular mesh depending on what best suits performance. 

#### 3.1.2.6: Tufts

A jagged-planar projecting type. Good for sprouting trees, jungle growths on branches, combining with other canopy types. Can use at all LOD when not obstructed or part of ensemble. Cull at low LOD when obstructed by greater canopy or part of ensemble. Tufts are also good for [Ground Cover](). 

#### 3.1.2.7: Fronds

An arching series of triangles. Good for palms, bushes, and jungle growths. 

#### 3.1.2.8: Jessen's Icosahedron

[Jessen's icosahedron](https://en.wikipedia.org/wiki/Jessen%27s_icosahedron) is a good replacement for [Icosahedra](#3121-icosahedron) when additional variety is desired. You can even build far LOD systems to choose between Jessen's icosahedron, the standard icosahedron, and icospheres when you want distant features to look variegated. This sort of construction is covered more completely in [Tree LOD Tricks](#318-tree-lod-tricks).

#### 3.1.3: Ball-stick Anchors

#### 3.1.4: Ball-stick Chains

#### 3.1.5: Ball Selection

#### 3.1.6: Well-known Component Constructions

We list some means of achieving less obvious constructions. 

##### 3.1.6.1: Palm Crown

Anchor a series of radially projecting frond rings in quick vertical succession. Give fronds at higher rings greater vertical bias, hence starting upwards at a slightly greater angle above lower fronds. 

##### 3.1.6.2: Palm Trunk

Do not allocate a stalk. Use canopy ball stick to build up from anchor, biasing roughly vertically. For consistent curve give slight angle bias and low variance. This will cause hysteresis to remain tight. 

Additionally, invert the typical stick dimensions s.t. the radius of the bottom of each segment is slightly less than the top. This will give the palm segment impression. 

##### 3.1.6.3: High-bushes and Shoots

Do not allocate a stalk. Bias a single ring of radial projection roughly vertically. 

##### 3.1.6.4: Jungle Growths

At ball points, in addition to canopy, allocate a larger and darker ball and a tuft.

##### 3.1.6.5: Banyan Trunk

Use large radius and high noise value for stalk. 

##### 3.1.6.6: Banyan Descenders 

Use a high radial projection segment count. Give a strong bias to grow vertically downward in excess of the height of the Banyan on every nth segment. 

#### 3.1.7: Well-known Tree Constructions

##### 3.1.7.1: Storybook Tree

A general impression of a tree. 

- Use fairly narrow stalk reaching far upwards to about 80% of the total height of the tree including the canopy. Use multiple rings of moderately-dense radial projection to construct the canopy. 
- Start radial projections at roughly 15% of the height of the tree. 
- Bias upper radial projections to be slightly shorter than lower ones. This tail-off like the logarithm approaching the axis from the positive side. (You should use this or something similar mathematically.)
- Place radial projections roughly every 8% of the height of the tree and roughly every 60 degrees, i.e., 6 per ring. 
- Allow total length of radial projections to range up to 60% of the height of the tree. 
- Upper radial projections should be allowed moderate angular variance about a straight horizontal projection, about 15 degrees. 
- Use 3-5 ball-stick segments per projection. Allowing branching between 1 and 3 with mean roughly 2. 
- Allocate [Plane Splay](#3125-plane-splay) for leaves at highest LOD, programming strong preference for only allocating at the outer layers of the canopy. Use a radius of roughly 9% the height of the tree. 

Good for many kinds of forests, particularly deciduous ones. Accommodates all kinds of shaders for both sticks and leaves.

##### 3.1.7.2: Liam's Conifer

A sparse conifer. 

- Use a narrow stalk to reach to about 100% of the total height of the tree. 
- Bias upper radial projections to reduce length linearly w.r.t. to lower ones. 
- Start radial projections roughly 10% of the height of the tree. 
- Place radial projections roughly every 4% of the height of the tree and roughly every 90 degrees, i.e., 4 per ring. 
- Bias radial projections to angle slightly downward, -2 degrees from horizontal. Allow variance within 8 degrees. 
- Use long first ball-stick segment, followed by two very short segments. Allow branching between 1 and 2 with mean closer to 1. 
- Max length of radial projections should be about 5% the height of the tree.
- Allocate [Tufts](#3126-tufts) for canopy at all ball joints. Use 2 to 3 tufts per joint. Allocate with scale proportional to roughly 2% the height of the tree.

Good for drier deciduous forests. Better with lighter shaders for both sticks and leaves. 

##### 3.1.7.3: Vase Tree

A tree that gets wider towards the top, giving a unique head-trained appearance. 

- Take the basic construction from [Storybook Tree](#3171-storybook-tree) but invert the radial projection length s.t. the width increases as you move up the tree. 
- Give the radial projections a vertical bias of 45 degrees from horizontal, decrease the bias as the height increases. 

Good for variety and mystical elements in deciduous forests. Can also be used for bushes and in urban settings. 

##### 3.1.7.4: Penmarch Torch

A tree that projects upwards like a torch. 

- Take the basic construction from [Vase Tree](#3173-vase-tree) but increase the vertical bias as the height increases. 

Good for chaparral, shorter conifers in arid regions, and urbanized settings. 

##### 3.1.7.5: Honu Banyan

A banyan-like tree, spreading its canopy far and wide, and descending some branches down. 

- Use the [Banyan Trunk](#3165-banyan-trunk) construction.
- Start the radial projections at roughly 80% of the total height of the tree. Do not use too many rings, 2 to 3. 
- Use the [Banyan Descenders](#3166-banyan-descenders) construction, typically biasing the canopy towards near horizontal except for every third to fourth segment. 
- Allocate leaf balls throughout the canopy. 

Good for jungle and riparian regions.  

##### 3.1.7.6: Sope's Banyan

A banyan-like tree with a vase-like crown. 

- Begin with the [Honu Banyan](#3175-honu-banyan) construction. 
- Adjust the canopy to project up vertically a la [Penmarch Torch](#3174-penmarch-torch). Radial projections should now begin at something like 40% of the total height. Descenders will still occur every third to fourth segment. 

Good for jungle and riparian regions. Adds a sense of mysticism. Often good as particularly tall, almost as if an [Elder Tree](#36-elder-trees). 

##### 3.1.7.7: Rory's Head-trained 

A stalk with a thin horizontal canopy at the top.

- Use a standard stock. 
- Begin radial projections at 90% or more of total height.
- Bias radial projections to be nearly horizontal. 
- Use moderate branching and segment length values, similar to [Storybook Tree](#3171-storybook-tree).

Good as a tree in arid regions. Good as a bush, e.g., grape vine. Can be used in almost all groves, except for coniferous ones. 

##### 3.1.7.8: Waialea Palm 

##### 3.1.7.9: Date Palm

##### 3.1.7.9: Palm Bush

##### 3.1.7.10: Northern Conifer

##### 3.1.7.11: Shoot

- Using the 

##### 3.1.7.12: 

#### 3.1.8: Tree LOD Tricks

#### 3.1.9: Stick Shading

#### 3.1.10: Leaf Shading

### 3.2: L-system Trees

### 3.3: Ground Cover

### 3.4: Cellular Groves

General name for vegetation type allocation system. Unify exclusive types you want to plant in a grove. Groves are the level at which planting constraints are painted in.

#### 3.4.1: Parameterization

#### 3.4.2: Cell Selection and Planting Constraints

#### 3.4.3: Well-known Ground Cover Groves

#### 3.4.4: Well-known Tufts Groves

#### 3.4.5: Well-known Bush Groves

#### 3.4.6: Well-known Tree Groves

#### 3.4.7: Grove LOD Tricks

### 3.5: Cellular Forests

General name for top-level grove allocation system. Split into several layers of groves. 

### 3.5.1: Parameterization

#### 3.5.2: Forest Layers

##### 3.5.2.1: Ground Cover Layer

##### 3.5.2.2: Tufts Layer

##### 3.5.2.3: Bush Layer

##### 3.5.2.4: Tree Layer

### 3.5.3: Well-known Forests

```rust
pub enum ForestCell {
    Riparian,
    Chaparral,
    Alpine,
    TemperateConiferous,
    Orchard,
    Coniferous,
    Jungle,
    TropicalJungle,

}
```

#### 3.5.3.1: Riparian 

#### 3.5.3.2: Chaparral

#### 3.5.3.3: Alpine

#### 3.5.3.4: Temperate Coniferous

#### 3.5.3.5: Orchard

#### 3.5.3.6: Coniferous

#### 

### 3.5.4: Forest LOD Tricks

### 3.6: Elder Trees

## 4: Milestone

