# RFC-N: Generalized LOD

## Table of Contents

## 1: Summary

To this point, Maybraid has used its cascade-based LOD system in an ad hoc manner. While we propose maintaining the base cascade, we provide a common pattern for its intended usage. Namely: 

1. A variably-typed `CascadeTracker` system is responsible for updating a `CascadePosition` on tracked entities, respecting updates in the game world. The tracked `CascadePosition` is the prompt for `CascadeChunk` production. 
2. A variably-typed `CascadeProduction` system is responsible for responding to changes in `CascadePositions` and spawning `CascadeChunks` with a `CascadeRequirement` level as children. It also updates a table mapping `CascadeChunks` to child entities. When a chunk needs to be culled, the `CascadeProduction` system acts directly, either marking `Hidden` or removing the chunk entity and all of its children. However, it also spawns `(CascadeChunk, ChunkCulling)` entity, for `ChunkTracker` systems which respond indirectly. 
3. A variably-typed `ChunkTracker` system is responsible for responding to new chunks and chunk culling, and dispatching tasks to meet chunk requirements. Usually, these will be tasks querying or generating over a spatial index, e.g., [RFC-142: Gimme](/rfc/rfc-000-000-142-gimme/README.md). When a `ChunkTracker` system wishes to respond directly to `ChunkProduction`, it should insert its results as children--allowing `ChunkProduction` to manage culling. Otherwise, it should not insert is results as children and should manage culling itself. 
4. A variably-typed `ChunkEntityTracker` system is responsible for responding to updates in the position of `ChunkManaged` entities--mainly so that the entities are not prematurely culled. `ChunkEntityTracker` systems use lookups to a parent `CascadeProduction` node to identify the appropriate chunk to which the child should be reattached.

## 2: Prior Art

## 3: Design

### 3.1: Cascade

### 3.2: `CascadeTracker`

### 3.3: `CascadeProduction`

### 3.4: `ChunkTracker`

#### 3.4.1: Integration with RFC-142: Gimme

The proposed spatial storage engine is currently [Gimme](/rfc/rfc-000-000-142-gimme/README.md). Accordingly, we have prepared an integration guide [here](./integration-with-gimme/README.md).

### 3.5: `ChunkEntityTracker`

## Milestones