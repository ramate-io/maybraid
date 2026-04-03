# RFC-N: Procedural Terrain Generation

## 1: Motivation

## 2: Prior Art

### 2.1: Theory

### 2.2: Practice

### 2.3: Already in Maybraid

## 3: Design

### 3.1: Core Concepts

### 3.2: Noise Base

We start from a simple Perlin noise base and make several improvements. 

### 3.3: Cellular Stamping

For a fixed cell size, we determine whether the cell has a certain kind of stamp applied with a PRNG. This is good for stamps that don't need to preserve multi-cell structure as the PRNG can be basic. For features that should span multiple cells--e.g., rivers, ridges, and valley chains--we can use fractal (noise) approaches as described in [3.4](#34-fractal-stamping).

### 3.4: Fractal Stamping

We recommend the following noise functions by stamp type:

### 3.5: Stamp Generation

Stamps themselves can be subject to noisy variation. 

### 3.6: Stamp Semantics

Some stamps can carry semantic meaning that is reused in later generation layers. For example, a riverbed stamp can both lower the elevation to match a consistent grade and mark its extents as a region in which fish can spawn. 

### 3.7: Stamp Chains

One common requirement will be to chain related stamps. For example, we may want to chain a riverbed to a waterfall to a riverbed. 

#### 3.7.1: Common Noise Chains

A simple approach to achieve global agreement on these chains is to generate stamps we intended to chain from the same noise function. We can accomplish this discretely, mapping noise value bands to different stamp types--or continuously, varying stamps with the value. 

#### 3.7.2: Fractal Neighborhood Stamps (FNS)

One particularly useful chaining approach is what we call Fractal Neighborhood Stamps (FNS). Instead of taking one seed value, Fractal Neighborhood Stamps are designed to take the values of their neighbors. This allows implementing clean connectivity patterns within the stamp itself. FNS does not solve the problem of creating features that require globally consistent variation, e.g., the consistent grade of a riverbed. 

#### 3.7.3: Fractal Paths

Sometimes, we need a chain of stamps that adapts terrain in consistent manner along a path. To achieve this we use...

#### 3.7.4: Higher-order Patterns and the Power of Large Extents

At times, it may seem that they patterns above are too restrictive. It's still hard to get a consistently graded river if you don't have some sense of the elevation at start and end--event harder if you want it to even out with other parts of the landscape.

Thus, sometimes, we will still want to have more restrictive regimes, to generate or at least prepare to generate complete geographical features. 

At the same time, however, we acknowledge that we cannot 

Thus, it is often helpful to think of large chained features as stamps themselves. When the cells for these stamps are loaded, we can invoke generation bespoke generation routines, describe cells within the stamp, etc.

Composing several such layers together we...

In many ways, this is a corollary of BVH, and parts of these generation schemes should plug into general BVH LOD systems. 

### 3.8: Jersey Stamps

We propose the following stamps be released under the Jersey edition of Maybraid terrain.

## 4: Milestones

