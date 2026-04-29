# RFC-N: Terrain Detail

## Table of Contents

## 1: Summary

In response to [#57](https://github.com/ramate-io/maybraid/issues/57), we propose a several simple cellular terrain detail generation systems under the name Durham Terrain Detail. 

## 2: Prior Art

## 3: Design

### 3.1: Sparse Boulders

1. Construct a first order grid which will bound boulder regions for LOD. All boulder placement must remain within the originating cell on this grid. 
2. Construct a second order grid with cells the size of boulder separation, typically bounding boulder size. Parameterize by minimum and maximum steepness for boulder placement. 
3. Sample over noise the cells to determine whether a cell may contain a boulder. 
4. Use the noise to pick an offset in the cell from the cell origin. This may optionally exceed the cell bounds, hence second order cells will not be used for LOD control or will have the same bounds as the first-order cell. If the offset point exceeds the cell in the first order grid, exit. Check the steepness at the point via the Laplacian applied to the underlying terrain. If exceeding steepness bounds, exit. 
5. Use the noise to generate a boulder shape and scale as SDF and thus produce a mesh. Note that, typically, we use a unit SDF for mesh generation and apply the scale to the mesh once spawned and the SDF at physics time. 
6. Place the boulder with some Z-offset, embedding it into the ground. 

### 3.2: Crag Complexes 

Apply parameters to decide whether cell should have a crag complex. Use a hysteresis pathfinding method similar to the [Stream](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#3132-stream) construction in Marazion watersheds to build a polyline or graph along which boulders will be placed within a cell.

### 3.3: 

### 3.4: 

## 4: Milestones