# RFC-n: Marazion Watersheds

## 1: Motivation

## 2: Prior Art

## 3: Design

The watershed designs proposed in this RFC are referred to as Marazion watersheds. All following the stamping framework proposed in [RFC-105](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain). 

### 3.1: Marazion Pocket Water Stamping

Marazion Pocket Waters are used to satisfy the [Jersey Pocket Waters requirement of RFC-105](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain#384-jersey-pocket-waters-small-hydrology-chains). 

Marazion pocket waters rely on three levels of cellular stamping hierarchy:

1. **[Pre-pocket Cells](#311-pre-pocket-cells):** the base parent cells representing the extents within which **Pocket Cells** are generated. For simplicity, they are a grid of fixed-size AABB cells and each fixes one cell size for all **Pocket Cells** contained within it, creating an internal grid. The role of **Pre-pocket Cell:** is to vary the extents of **Pocket Cells** over the game world, while keeping regional correlations. The noise value for the Pocket Cell size is given by the lower-left coordinate of Pre-pocket Cell, floored to a reasonable multiple.
2. **[Pocket Cells](#312-pocket-cells):** the cells within which certain simple hydrology types are selected. A **Pocket Cell** use a pseudo-random Guillotine cuts, with bounded depth. The noise value for the Guillotine cuts is given by the lower-left coordinate of the Pocket Cell. 
3. **[Pocket Water Cells](#313-pocket-water-cells):** the cells within which independent pocket water types are generated. 

#### 3.1.1: Pre-pocket Cells

#### 3.1.2: Pocket Cells

#### 3.1.3: Pocket Water Cells

The following are the Pocket Water Cells which should be included in Marazion.

To ensure reliable rims, all construction rely on creating a plateau, then depressing everything within the plateau to ensure a rim around the body of water. 

##### 3.1.3.1: Lake

1. Sample the noise to offset the centroid of the lake from the centroid of the cell. 
2. Compute the elevation of the surface of the lake by adding some positive or negative noise generated value to the elevation of the current centroid. Compute the depth from the noise generated value.
3. Then, compute the $\text{ pre-radius }$ of the lake by taking the distance from the offset centroid to the nearest boundary of the cell and subtract some $\mu$. 
4. When sampling, the $\text{ radius } = \text{ pre-radius } - \text{ noise }$ will be treated noisily. Points will be allowed to fall inside. 
5. Raise all points within the $\text{ radius }$ to the surface elevation **plus** some noise value. 
6. Raise all points within the $\text{ radius } + \alpha * \text{ noise } * \mu$ where $\alpha, \text{ noise } \in [0, 1]$ to the surface elevation **plus or minus** some noise. 
7. Depress all points within $\text{ radius } - \alpha * \text{ noise } * \mu$ by subtracting $\text{ dist to centroid } * \text{ noise } * \text{ depth }$ from the current elevation. 

In Rust pseudocode, the elevation modulation to the original SDF would look something like the following: 

```rust
```

##### 3.1.3.2: Stream

##### 3.1.3.3: Bog

##### 3.1.3.4: Lake into Stream

##### 3.1.3.5: Stream into Lake

### 3.2: Marazion Basin Water Stamping

### 3.3: Marazion Hydrology Complex Stamping

### 3.4: Marazion Global Ocean

## 4: Milestones