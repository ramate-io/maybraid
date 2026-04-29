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

### 3.3: World Unit Varying Shader for Ground Color

### 3.4: Bump Outs

"Bump Outs" refer to structures placed above the terrain which follow its contours. In Durham terrain detail, we build bump-outs simply by cloning the underlying terrain SDF and adding to its Z extents noisily within some boundary determined via a noisy radius. We provide the general cell and boundary generation description in [3.4.1](#341-cell-and-boundary-generation) and specify particular bump outs in the sections which follow.

#### 3.4.1: Cell and Boundary Generation

#### 3.4.2: Snow Bump Out

1. Parameterize whether cell is snowy by underlying elevation and fractal noise sampling for local consistency.
2. Standard bump out. 
3. Use snow shader. 
4. Don't worry about seasonality yet. 

#### 3.4.3: Sand and Dunes Bump Out

1. Parameterize by whether cell has sand dunes by steepness sampled at a few points and fractal noise sampling for local consistency. 
2. Use inner grid to generate points at which elliptical dunes will exist. 
3. Apply standard bump out noise plus dune "dome" noise around selected elliptical points. 
4. Use sand shader. 

## 4: Milestones