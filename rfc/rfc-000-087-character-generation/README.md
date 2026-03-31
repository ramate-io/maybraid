# RFC-87: Character Generation

## 1: Background

Maybraid intends to rely on procedural generation, including for the generation of characters. We propose a character generation system and roadmap to this end. 

The system proposed is a basic assembly of multi-meshes with species types controlling high-order patterns. Generally speaking, a species will restrict features to a set of 

In early development, we prioritize simple bi- and quadrupedal designs. We describe how to work to these designs from basic topology: spheroids, cylinders, and simple polygonal volumes. 

Following from our low-poly look, we intend to specify these designs with minimal skinning. 

Animations of the characters will also be subject to slight variation via procedural generation. 

## 2: Prior Art

## 3: Proposed Design

### 3.1: Species Desiderata

> [!NOTE]
> In RFC-87, we do not provide strict species definitions. We leave this for future RFCs. We do provide desiderata for the species and concept art for various limb parts. 

The initial offering of species is called Crozon. Subspecies should follow from procedural generation extension of basic routines. However, we expect the following distinct species routines in Crozon:

- Four humanoid species
- Six grazing species, building for quadruped skeleton
- Three large land predators, building for quadruped skeleton
- Three small land animals, building for quadruped skeleton
- One very large "noble" land animal, building for either quadruped or biped skeleton
- Six medium flying creatures, building for bipedal skeleton
- One very large "noble" flying creature, building for biped skeleton. 
- Three medium-size swimming creatures, building for the quadruped skeleton.
- Three small swimming creatures, building for the quadruped skeleton.
- One very large "noble" swimming creature, building for the quadruped skeleton. 

### 3.2: Multi-meshes

> [!TIP]
> Concept art for the multi-meshes is linked by section to subdirectories under this folder. 

### 3.3: Animations

Initial Animations will be delivered under the title Malo. 

## 4: Milestones

> [!NOTE]
> The milestones below are not intended to cover the entire duration of the project. This is intended with minimal speculation.

### 4.1: Crozon Head Shape Variant Multi-meshes

### 4.2: Crozon Ear Variant Multi-meshes

### 4.3: Crozon Horn Variant Multi-meshes

### 4.4: Crozon Eye Variant Multi-meshes

### 4.5: Crozon Nose and Snout Variant Multi-meshes

### 4.6: Crozon Mouth Variant Multi-meshes

### 4.7: Crozon Head Assembly Variant Multi-meshes

### 4.8: Crozon Neck Variant Multi-meshes

### 4.9: Crozon Lower Limb Variant Multi-meshes

### 4.10: Crozon Upper Limb Variant Multi-meshes

### 4.11: Crozon Hand and Foot Variant Multi-meshes

### 4.12: Crozon Torso Variant Multi-meshes

### 4.13: Crozon Tail Variant Multi-meshes

### 4.14: Biped Skeleton

### 4.15: Quadruped Skeleton

### 4.16: Develop Multi-mesh API

Develop the multi-mesh API for rigging skeletons as described in [RFC-88](../rfc-000-088-bevy-multi-mesh/README.md)

### 4.17: Crozon Species Assembly

### 4.18: Malo Gait Animation for Bipeds

### 4.19: Malo Gait Animation for Quadrupeds

### 4.20: Malo Carrying and Grasping Animation for Bipeds

### 4.21: Malo Expressions Animations