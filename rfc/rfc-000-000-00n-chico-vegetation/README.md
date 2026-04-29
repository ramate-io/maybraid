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

[Jessen's icosahedron](https://en.wikipedia.org/wiki/Jessen%27s_icosahedron) is a good replacement for [Icosahedra](#3121-icosahedron) when additional variety is desired. You can even build far LOD systems to choose between Jessen's icosahedron, the standard icosahedron, and icospheres when you want distant features to look variegated. This sort of construction is covered more completely in [LOD Tricks](#318-lod-tricks)

#### 3.1.3: Ball-stick Anchors

#### 3.1.4: Ball-stick Chains

#### 3.1.5: Ball Selection

#### 3.1.6: Well-known Component Constructions

#### 3.1.7: Well-known Tree Constructions

#### 3.1.8: LOD Tricks

### 3.2: L-system Trees

### 3.4: Ground Cover

### 3.3: Cellular Groves

General name for vegetation type allocation system. Unify types 

### 3.5: Cellular Forests

General name for top-level 

### 3.6: Elder Trees

## 4: Milestone

